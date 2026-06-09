use serde_json::{json, Value};

pub(super) fn current_generation(state: &Value, key: &str) -> u64 {
    state["_runtime"][key].as_u64().unwrap_or(0)
}

pub(super) fn pending_effect(state: &Value, effect_id: &str) -> Option<Value> {
    state["pendingEffects"]
        .as_array()?
        .iter()
        .find(|effect| effect["id"].as_str() == Some(effect_id))
        .cloned()
}

pub(super) fn remove_pending_effect(state: &mut Value, effect_id: &str) {
    if let Some(effects) = state["pendingEffects"].as_array_mut() {
        effects.retain(|effect| effect["id"].as_str() != Some(effect_id));
    }
}

pub(super) fn normalize_error(error: Value) -> Value {
    if error.is_null() {
        json!({ "code": "generic" })
    } else {
        error
    }
}

pub(super) fn error_code(error: &Value) -> String {
    error["code"]
        .as_str()
        .or_else(|| error.as_str())
        .unwrap_or("generic")
        .to_string()
}

pub(super) fn active_profile_id(state: &Value, profile: &Value) -> String {
    profile["id"]
        .as_str()
        .or_else(|| state["profile"]["activeProfileId"].as_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("guest")
        .to_string()
}

pub(super) fn visible_streams(streams: &Value, selected_addon: Option<&str>) -> Value {
    let Some(selected_addon) = selected_addon
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return streams.clone();
    };
    let selected_lower = selected_addon.to_lowercase();
    let filtered = streams
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter(|stream| {
                    stream["addonName"]
                        .as_str()
                        .map(|addon_name| addon_name.trim().to_lowercase() == selected_lower)
                        .unwrap_or(false)
                })
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    json!(filtered)
}

pub(super) fn value_array_is_empty(value: &Value) -> bool {
    value.as_array().map(Vec::is_empty).unwrap_or(true)
}

pub(super) fn normalize_meta_trailers(meta: &Value) -> Value {
    let trailers = meta["trailers"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(normalize_meta_trailer)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Value::Array(trailers)
}

fn normalize_meta_trailer(trailer: &Value) -> Option<Value> {
    let youtube_id = non_blank_str(trailer, "ytId").or_else(|| {
        non_blank_str(trailer, "source")
            .filter(|value| !value.starts_with("http://") && !value.starts_with("https://"))
    });
    let url = non_blank_str(trailer, "externalUrl")
        .or_else(|| non_blank_str(trailer, "url"))
        .or_else(|| {
            youtube_id
                .as_ref()
                .map(|value| format!("https://www.youtube.com/watch?v={value}"))
        })?;
    let id = youtube_id.clone().unwrap_or_else(|| url.clone());
    let item_type = non_blank_str(trailer, "type").unwrap_or_else(|| "Trailer".to_string());
    let title = non_blank_str(trailer, "name")
        .or_else(|| non_blank_str(trailer, "title"))
        .or_else(|| non_blank_str(trailer, "description"))
        .unwrap_or_else(|| item_type.clone());
    let thumbnail = non_blank_str(trailer, "thumbnail").or_else(|| {
        youtube_id
            .as_ref()
            .map(|value| format!("https://i.ytimg.com/vi/{value}/hqdefault.jpg"))
    });
    Some(json!({
        "id": id,
        "title": title,
        "type": item_type,
        "url": url,
        "thumbnail": thumbnail,
        "source": "addon"
    }))
}

pub(super) fn non_blank_str(value: &Value, key: &str) -> Option<String> {
    value[key]
        .as_str()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

pub(super) fn should_sync_watched_state(profile: Option<&Value>, meta: Option<&Value>) -> bool {
    let Some(meta) = meta else { return false };
    if meta["id"]
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .is_none()
    {
        return false;
    }
    let Some(profile) = profile else { return false };
    let is_guest = profile["isGuest"].as_bool().unwrap_or(false);
    let has_trakt_token = profile["traktAccessToken"]
        .as_str()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    !is_guest || has_trakt_token
}

pub(super) fn upsert_by_key(target: &mut Value, key: &str, value: &str, item: Value) {
    if !target.is_array() {
        *target = json!([]);
    }
    let Some(items) = target.as_array_mut() else { return };
    if let Some(existing) = items
        .iter_mut()
        .find(|existing| existing[key].as_str() == Some(value))
    {
        *existing = item;
    } else {
        items.push(item);
    }
}

pub(super) fn merge_object(target: &mut Value, patch: Value) {
    match (target.as_object_mut(), patch) {
        (Some(target), Value::Object(patch)) => {
            for (key, value) in patch {
                target.insert(key, value);
            }
        }
        _ => {}
    }
}
