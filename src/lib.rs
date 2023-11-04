use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_plugin::*;
use fuzzy_matcher::FuzzyMatcher;
use hyprland::{
    data::{Client, Clients},
    dispatch::*,
    prelude::*,
    shared::Address,
};

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "Windows".into(),
        icon: "preferences-system-windows".into(),
    }
}

#[derive(serde::Deserialize, Debug)]
struct Config {
    prefix: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            prefix: "".to_string(),
        }
    }
}

pub struct State {
    config: Config,
    entries: Vec<Client>,
}

#[init]
fn init(config_dir: RString) -> State {
    let config = match std::fs::read_to_string(format!("{}/anyrun-hyprland.ron", config_dir)) {
        Ok(content) => ron::from_str(&content).unwrap_or_default(),
        Err(_) => Config::default(),
    };

    let entries = match Clients::get() {
        Ok(entries) => entries,
        Err(why) => {
            eprintln!("Could not get entries: {}", why);
            return State {
                config,
                entries: Vec::new(),
            };
        }
    }
    .to_vec();

    State { config, entries }
}

fn address_to_u64(address: &Address) -> u64 {
    // SAFETY: Hyprland always returns addresses of the form "0x...",
    // so this shouldn't crash.
    u64::from_str_radix(&address.to_string()[2..], 16).unwrap()
}

fn address_from_u64(id: u64) -> Address {
    Address::new(format!("{:#x}", id))
}

#[get_matches]
fn get_matches(input: RString, state: &State) -> RVec<Match> {
    if !input.starts_with(&state.config.prefix) {
        return RVec::new();
    }

    let matcher = fuzzy_matcher::skim::SkimMatcherV2::default().smart_case();
    let term = input.trim_start_matches(&state.config.prefix);

    state
        .entries
        .clone()
        .into_iter()
        .filter_map(|info| {
            matcher
                .fuzzy_match(&info.title, &term)
                .map(|score| (info, score))
        })
        .map(|(info, _)| Match {
            title: info.title.clone().into(),
            description: ROption::RSome(format!("Window {}", info.address).into()),
            use_pango: false,
            icon: ROption::RNone,
            id: ROption::RSome(address_to_u64(&info.address)),
        })
        .collect::<Vec<_>>()
        .into()
}

#[handler]
fn handler(selection: Match) -> HandleResult {
    if let Err(why) = hyprland::dispatch!(
        FocusWindow,
        WindowIdentifier::Address(address_from_u64(selection.id.unwrap()))
    ) {
        eprintln!("Failed to focus window: {}", why);
    }

    HandleResult::Close
}
