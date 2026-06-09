use serde::{Deserialize, Serialize};

/// Exhaustive catalog of all effect types the headless engine can emit.
///
/// This is the single source of truth for effect type names — the string
/// representations produced by `as_str()` are the ones the platform (Kotlin,
/// JS, etc.) matches against in its effect dispatcher.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectKind {
    ClearPlaybackProgress,
    EnqueueOfflineDownload,
    EnqueueTraktScrobble,
    ExchangeAuthCode,
    FetchAddonManifest,
    FetchAddonResource,
    FetchCatalogPage,
    FetchDetailSecondary,
    FetchDetailStreams,
    FetchIntroSegments,
    FetchMetaDetail,
    FetchMetaDetailLookup,
    FetchSeasonEpisodes,
    FetchSubtitles,
    NotifyReleasedEpisodes,
    PrefetchDetailStreams,
    PrefetchNextEpisodeStreams,
    PrepareDirectPlayback,
    ReadCalendarMonth,
    ReadDetailLocalState,
    ReadDiscoverCatalogFilters,
    ReadHomeBootstrap,
    ReadLibraryState,
    ReadPlaybackProgress,
    RefreshAuthToken,
    RefreshInstalledAddons,
    ReplaceExternalContinueWatching,
    ResolveIntroImdbId,
    RunAuthFlow,
    RunDiscover,
    RunExternalSync,
    RunSearch,
    StartTorrentStream,
    StopTorrent,
    SyncExternalIntegration,
    SyncWatchedState,
    UpdateCalendarWidget,
    WriteFeedback,
    WriteLibraryCommand,
    WritePlaybackProgress,
    WriteSettings,
}

impl EffectKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EffectKind::ClearPlaybackProgress => "clearPlaybackProgress",
            EffectKind::EnqueueOfflineDownload => "enqueueOfflineDownload",
            EffectKind::EnqueueTraktScrobble => "enqueueTraktScrobble",
            EffectKind::ExchangeAuthCode => "exchangeAuthCode",
            EffectKind::FetchAddonManifest => "fetchAddonManifest",
            EffectKind::FetchAddonResource => "fetchAddonResource",
            EffectKind::FetchCatalogPage => "fetchCatalogPage",
            EffectKind::FetchDetailSecondary => "fetchDetailSecondary",
            EffectKind::FetchDetailStreams => "fetchDetailStreams",
            EffectKind::FetchIntroSegments => "fetchIntroSegments",
            EffectKind::FetchMetaDetail => "fetchMetaDetail",
            EffectKind::FetchMetaDetailLookup => "fetchMetaDetailLookup",
            EffectKind::FetchSeasonEpisodes => "fetchSeasonEpisodes",
            EffectKind::FetchSubtitles => "fetchSubtitles",
            EffectKind::NotifyReleasedEpisodes => "notifyReleasedEpisodes",
            EffectKind::PrefetchDetailStreams => "prefetchDetailStreams",
            EffectKind::PrefetchNextEpisodeStreams => "prefetchNextEpisodeStreams",
            EffectKind::PrepareDirectPlayback => "prepareDirectPlayback",
            EffectKind::ReadCalendarMonth => "readCalendarMonth",
            EffectKind::ReadDetailLocalState => "readDetailLocalState",
            EffectKind::ReadDiscoverCatalogFilters => "readDiscoverCatalogFilters",
            EffectKind::ReadHomeBootstrap => "readHomeBootstrap",
            EffectKind::ReadLibraryState => "readLibraryState",
            EffectKind::ReadPlaybackProgress => "readPlaybackProgress",
            EffectKind::RefreshAuthToken => "refreshAuthToken",
            EffectKind::RefreshInstalledAddons => "refreshInstalledAddons",
            EffectKind::ReplaceExternalContinueWatching => "replaceExternalContinueWatching",
            EffectKind::ResolveIntroImdbId => "resolveIntroImdbId",
            EffectKind::RunAuthFlow => "runAuthFlow",
            EffectKind::RunDiscover => "runDiscover",
            EffectKind::RunExternalSync => "runExternalSync",
            EffectKind::RunSearch => "runSearch",
            EffectKind::StartTorrentStream => "startTorrentStream",
            EffectKind::StopTorrent => "stopTorrent",
            EffectKind::SyncExternalIntegration => "syncExternalIntegration",
            EffectKind::SyncWatchedState => "syncWatchedState",
            EffectKind::UpdateCalendarWidget => "updateCalendarWidget",
            EffectKind::WriteFeedback => "writeFeedback",
            EffectKind::WriteLibraryCommand => "writeLibraryCommand",
            EffectKind::WritePlaybackProgress => "writePlaybackProgress",
            EffectKind::WriteSettings => "writeSettings",
        }
    }
}

/// Wire format for an effect emitted by the headless engine.
///
/// Matches the `NativeHeadlessEffect` data class on the Kotlin side:
/// ```kotlin
/// data class NativeHeadlessEffect(
///     val id: String,
///     val type: String,
///     val generation: Long,
///     val payload: Map<String, Any?>
/// )
/// ```
///
/// `id` is a monotonically-increasing opaque string (`"fx-N"`).
/// `generation` lets the platform discard stale completions.
/// `payload` carries effect-specific parameters as a JSON object.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectEnvelope {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub generation: u64,
    pub payload: serde_json::Value,
}

impl EffectEnvelope {
    pub fn new(id: String, kind: EffectKind, generation: u64, payload: serde_json::Value) -> Self {
        Self {
            id,
            kind: kind.as_str().to_owned(),
            generation,
            payload,
        }
    }

    pub fn raw(id: String, kind: &str, generation: u64, payload: serde_json::Value) -> Self {
        Self {
            id,
            kind: kind.to_owned(),
            generation,
            payload,
        }
    }
}

/// Typed effect emitted by a portable engine model for the platform to execute.
///
/// Mirrors stremio-core's `Effect` enum.  Each variant carries fully-typed
/// payload fields, making it compile-time verified and WASM-safe.
///
/// The headless engine currently serializes effects through `EffectEnvelope`
/// (untyped payload).  This typed enum is the long-term migration target for
/// models that have been fully ported away from `serde_json::Value` state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Effect {
    FetchAddonResource {
        effect_id: String,
        url: String,
        timeout_ms: u32,
    },
    FetchAddonManifest {
        effect_id: String,
        url: String,
        timeout_ms: u32,
    },
    FetchCatalogPage {
        effect_id: String,
        url: String,
    },
    FetchMetaDetail {
        effect_id: String,
        url: String,
    },
    FetchStreams {
        effect_id: String,
        url: String,
    },
    FetchSubtitles {
        effect_id: String,
        url: String,
    },
    GetStorage {
        effect_id: String,
        key: String,
    },
    SetStorage {
        effect_id: String,
        key: String,
        value: Option<String>,
    },
    Log {
        message: String,
    },
}

#[derive(Debug, Clone, Default)]
pub struct Effects {
    pub effects: Vec<Effect>,
    pub has_changed: bool,
}

impl Effects {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn changed() -> Self {
        Self {
            effects: vec![],
            has_changed: true,
        }
    }

    pub fn with_effect(mut self, effect: Effect) -> Self {
        self.effects.push(effect);
        self
    }

    pub fn merge(mut self, other: Effects) -> Self {
        self.effects.extend(other.effects);
        self.has_changed = self.has_changed || other.has_changed;
        self
    }
}
