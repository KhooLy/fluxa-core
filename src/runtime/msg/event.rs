use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Event {
    ProfileChanged,
    LibrarySynced,
    AddonInstalled {
        transport_url: String,
    },
    AddonUninstalled {
        transport_url: String,
    },
    PlayerStopped {
        content_type: Option<String>,
        id: Option<String>,
        video_id: Option<String>,
        duration_ms: i64,
        time_offset_ms: i64,
    },
    Error {
        code: String,
        message: String,
    },
}
