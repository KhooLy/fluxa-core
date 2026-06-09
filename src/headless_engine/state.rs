use serde_json::{json, Value};

pub(super) fn default_state() -> Value {
    json!({
        "navigation": { "route": "home", "params": Value::Null },
        "home": {},
        "search": {},
        "discover": {},
        "detail": {},
        "player": {},
        "library": {},
        "profile": {},
        "settings": {},
        "calendar": {},
        "addons": { "installed": [] },
        "auth": {},
        "sync": {},
        "lookup": {},
        "offline": {},
        "pendingEffects": [],
        "_runtime": {
            "detailGeneration": 0,
            "playerGeneration": 0,
            "homeGeneration": 0,
            "libraryGeneration": 0,
            "addonGeneration": 0,
            "searchGeneration": 0,
            "discoverGeneration": 0,
            "syncGeneration": 0,
            "authGeneration": 0,
            "settingsGeneration": 0,
            "calendarGeneration": 0,
            "offlineGeneration": 0,
            "detailStreamsGeneration": 0,
            "lookupGeneration": 0,
            "playbackPrepGeneration": 0,
            "introGeneration": 0
        }
    })
}
