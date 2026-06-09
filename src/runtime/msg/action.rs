use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Action {
    LoadHome,
    LoadCatalog {
        addon_transport_url: String,
        catalog_id: String,
        content_type: String,
    },
    LoadMetaDetail {
        content_type: String,
        id: String,
    },
    LoadStreams {
        content_type: String,
        id: String,
    },
    Search {
        query: String,
    },
    SetProfile {
        profile_json: String,
    },
    SyncLibrary,
    PlayerStarted {
        content_type: String,
        id: String,
        video_id: Option<String>,
    },
    PlayerStopped,
    PlayerProgressUpdate {
        position_ms: i64,
        duration_ms: i64,
    },
    InstallAddon {
        transport_url: String,
    },
    UninstallAddon {
        transport_url: String,
    },
    LibraryItemWatched {
        id: String,
        watched: bool,
    },
}
