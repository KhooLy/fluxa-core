use super::addons::AddonsState;
use super::auth::AuthState;
use super::calendar::CalendarState;
use super::detail::{DetailState, LookupState};
use super::discover::DiscoverState;
use super::home::HomeState;
use super::library::LibraryState;
use super::navigation::NavigationState;
use super::offline::OfflineState;
use super::player::PlayerState;
use super::profile::ProfileState;
use super::search::SearchState;
use super::settings::SettingsState;
use super::sync::SyncState;
use crate::runtime::EffectEnvelope;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GenerationKey {
    Detail,
    Player,
    Home,
    Library,
    Addon,
    Search,
    Discover,
    Sync,
    Auth,
    Settings,
    Calendar,
    Offline,
    DetailStreams,
    Lookup,
    PlaybackPrep,
    Intro,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub(super) struct RuntimeGenerations {
    detail_generation: u64,
    player_generation: u64,
    home_generation: u64,
    library_generation: u64,
    addon_generation: u64,
    search_generation: u64,
    discover_generation: u64,
    sync_generation: u64,
    auth_generation: u64,
    settings_generation: u64,
    calendar_generation: u64,
    offline_generation: u64,
    detail_streams_generation: u64,
    lookup_generation: u64,
    playback_prep_generation: u64,
    intro_generation: u64,
}

impl RuntimeGenerations {
    pub(super) fn get(&self, key: GenerationKey) -> u64 {
        match key {
            GenerationKey::Detail => self.detail_generation,
            GenerationKey::Player => self.player_generation,
            GenerationKey::Home => self.home_generation,
            GenerationKey::Library => self.library_generation,
            GenerationKey::Addon => self.addon_generation,
            GenerationKey::Search => self.search_generation,
            GenerationKey::Discover => self.discover_generation,
            GenerationKey::Sync => self.sync_generation,
            GenerationKey::Auth => self.auth_generation,
            GenerationKey::Settings => self.settings_generation,
            GenerationKey::Calendar => self.calendar_generation,
            GenerationKey::Offline => self.offline_generation,
            GenerationKey::DetailStreams => self.detail_streams_generation,
            GenerationKey::Lookup => self.lookup_generation,
            GenerationKey::PlaybackPrep => self.playback_prep_generation,
            GenerationKey::Intro => self.intro_generation,
        }
    }

    pub(super) fn bump(&mut self, key: GenerationKey) -> u64 {
        let slot = match key {
            GenerationKey::Detail => &mut self.detail_generation,
            GenerationKey::Player => &mut self.player_generation,
            GenerationKey::Home => &mut self.home_generation,
            GenerationKey::Library => &mut self.library_generation,
            GenerationKey::Addon => &mut self.addon_generation,
            GenerationKey::Search => &mut self.search_generation,
            GenerationKey::Discover => &mut self.discover_generation,
            GenerationKey::Sync => &mut self.sync_generation,
            GenerationKey::Auth => &mut self.auth_generation,
            GenerationKey::Settings => &mut self.settings_generation,
            GenerationKey::Calendar => &mut self.calendar_generation,
            GenerationKey::Offline => &mut self.offline_generation,
            GenerationKey::DetailStreams => &mut self.detail_streams_generation,
            GenerationKey::Lookup => &mut self.lookup_generation,
            GenerationKey::PlaybackPrep => &mut self.playback_prep_generation,
            GenerationKey::Intro => &mut self.intro_generation,
        };
        *slot = slot.saturating_add(1);
        *slot
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub(super) struct EngineState {
    pub(super) navigation: NavigationState,
    pub(super) home: HomeState,
    pub(super) search: SearchState,
    pub(super) discover: DiscoverState,
    pub(super) detail: DetailState,
    pub(super) player: PlayerState,
    pub(super) library: LibraryState,
    pub(super) profile: ProfileState,
    pub(super) settings: SettingsState,
    pub(super) calendar: CalendarState,
    pub(super) addons: AddonsState,
    pub(super) auth: AuthState,
    pub(super) sync: SyncState,
    pub(super) lookup: LookupState,
    pub(super) offline: OfflineState,
    pub(super) pending_effects: Vec<EffectEnvelope>,
    #[serde(rename = "_runtime")]
    pub(super) runtime: RuntimeGenerations,
}
