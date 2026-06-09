use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Internal {
    AddonResourceFetched {
        effect_id: String,
        url: String,
        status_code: u16,
        body: Option<String>,
    },
    AddonManifestFetched {
        effect_id: String,
        url: String,
        status_code: u16,
        body: Option<String>,
    },
    StorageRead {
        effect_id: String,
        key: String,
        value: Option<String>,
    },
    StorageWritten {
        effect_id: String,
        key: String,
    },
    HttpError {
        effect_id: String,
        message: String,
    },
}
