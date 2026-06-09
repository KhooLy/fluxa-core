use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TogglePlanRequest {
    item: Value,
    #[serde(default)]
    is_currently_in_watchlist: bool,
    #[serde(default)]
    profile_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExternalMergeRequest {
    #[serde(default)]
    local_items: Vec<Value>,
    #[serde(default)]
    external_items: Vec<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CollectionImportRequest {
    #[serde(default)]
    collections: Vec<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OfflineGroupingRequest {
    #[serde(default)]
    items: Vec<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProgressMergeRequest {
    existing: Value,
    incoming: Value,
}

pub(crate) fn watchlist_toggle_plan_json(request_json: &str) -> Option<String> {
    let request = serde_json::from_str::<TogglePlanRequest>(request_json).ok()?;
    let is_in_watchlist = request.is_currently_in_watchlist;
    let should_add = !is_in_watchlist;
    let item_id = request
        .item
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    serde_json::to_string(&json!({
        "command": if should_add { "add" } else { "remove" },
        "itemId": item_id,
        "optimisticIsInWatchlist": should_add,
        "profileId": request.profile_id
    }))
    .ok()
}

pub(crate) fn library_external_merge_plan_json(request_json: &str) -> Option<String> {
    let request = serde_json::from_str::<ExternalMergeRequest>(request_json).ok()?;
    let local_ids: std::collections::HashSet<String> = request
        .local_items
        .iter()
        .filter_map(|item| item.get("id").and_then(Value::as_str).map(ToString::to_string))
        .collect();
    let merged_external: Vec<&Value> = request
        .external_items
        .iter()
        .filter(|item| {
            item.get("id")
                .and_then(Value::as_str)
                .is_some_and(|id| !local_ids.contains(id))
        })
        .collect();
    let mut merged: Vec<Value> = request.local_items.clone();
    merged.extend(merged_external.into_iter().cloned());
    serde_json::to_string(&json!({
        "merged": merged,
        "localCount": request.local_items.len(),
        "externalOnlyCount": merged.len() - request.local_items.len()
    }))
    .ok()
}

pub(crate) fn library_collection_import_validation_json(request_json: &str) -> Option<String> {
    let request = serde_json::from_str::<CollectionImportRequest>(request_json).ok()?;
    let mut issues = Vec::<String>::new();
    let mut valid_collections = Vec::<Value>::new();
    for (i, col) in request.collections.iter().enumerate() {
        let id = col.get("id").and_then(Value::as_str).unwrap_or("").trim();
        let title = col.get("title").and_then(Value::as_str).unwrap_or("").trim();
        if id.is_empty() {
            issues.push(format!("collection[{}]: missing id", i));
            continue;
        }
        if title.is_empty() {
            issues.push(format!("collection[{}]: missing title", i));
            continue;
        }
        valid_collections.push(col.clone());
    }
    serde_json::to_string(&json!({
        "isValid": issues.is_empty(),
        "validCollections": valid_collections,
        "issues": issues
    }))
    .ok()
}

pub(crate) fn library_offline_grouping_json(request_json: &str) -> Option<String> {
    let request = serde_json::from_str::<OfflineGroupingRequest>(request_json).ok()?;
    let mut ready = Vec::<&Value>::new();
    let mut downloading = Vec::<&Value>::new();
    let mut queued = Vec::<&Value>::new();
    let mut failed = Vec::<&Value>::new();
    for item in &request.items {
        match item.get("status").and_then(Value::as_str).unwrap_or("") {
            "ready" | "complete" => ready.push(item),
            "downloading" | "in_progress" => downloading.push(item),
            "failed" | "error" => failed.push(item),
            _ => queued.push(item),
        }
    }
    serde_json::to_string(&json!({
        "ready": ready,
        "downloading": downloading,
        "queued": queued,
        "failed": failed
    }))
    .ok()
}

pub(crate) fn playback_progress_merge_plan_json(request_json: &str) -> Option<String> {
    let request = serde_json::from_str::<ProgressMergeRequest>(request_json).ok()?;
    let existing = &request.existing;
    let incoming = &request.incoming;

    let existing_video_id = existing.get("lastVideoId").and_then(Value::as_str);
    let incoming_video_id = incoming.get("lastVideoId").and_then(Value::as_str);
    let video_changed =
        incoming_video_id.is_some() && incoming_video_id != existing_video_id;

    let resolve_field = |key: &str| -> Value {
        incoming
            .get(key)
            .filter(|v| !v.is_null())
            .cloned()
            .or_else(|| existing.get(key).cloned())
            .unwrap_or(Value::Null)
    };

    let last_episode_name = if video_changed {
        incoming.get("lastEpisodeName").cloned().unwrap_or(Value::Null)
    } else {
        resolve_field("lastEpisodeName")
    };
    let resolve_episode_field = |key: &str| -> Value {
        if video_changed {
            incoming.get(key).cloned().unwrap_or(Value::Null)
        } else {
            resolve_field(key)
        }
    };

    serde_json::to_string(&json!({
        "lastVideoId": resolve_field("lastVideoId"),
        "timeOffset": incoming.get("timeOffset").cloned().unwrap_or(Value::Null),
        "duration": incoming.get("duration").cloned().unwrap_or(Value::Null),
        "lastStreamIndex": resolve_field("lastStreamIndex"),
        "lastEpisodeName": last_episode_name,
        "lastEpisodeSeason": resolve_episode_field("lastEpisodeSeason"),
        "lastEpisodeNumber": resolve_episode_field("lastEpisodeNumber"),
        "lastEpisodeThumbnail": resolve_episode_field("lastEpisodeThumbnail"),
        "lastStreamUrl": resolve_field("lastStreamUrl"),
        "lastStreamTitle": resolve_field("lastStreamTitle"),
        "continueWatchingPoster": resolve_field("continueWatchingPoster"),
        "continueWatchingBackground": resolve_field("continueWatchingBackground"),
        "lastAudioLanguage": resolve_field("lastAudioLanguage"),
        "lastSubtitleLanguage": resolve_field("lastSubtitleLanguage"),
        "videoChanged": video_changed
    }))
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn toggle_plan_adds_when_not_in_watchlist() {
        let result: Value = serde_json::from_str(
            &watchlist_toggle_plan_json(
                r#"{"item":{"id":"tt1","type":"movie"},"isCurrentlyInWatchlist":false}"#,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(result["command"], "add");
        assert_eq!(result["optimisticIsInWatchlist"], true);
    }

    #[test]
    fn toggle_plan_removes_when_in_watchlist() {
        let result: Value = serde_json::from_str(
            &watchlist_toggle_plan_json(
                r#"{"item":{"id":"tt1","type":"movie"},"isCurrentlyInWatchlist":true}"#,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(result["command"], "remove");
        assert_eq!(result["optimisticIsInWatchlist"], false);
    }

    #[test]
    fn external_merge_deduplicates_preferring_local() {
        let result: Value = serde_json::from_str(
            &library_external_merge_plan_json(
                r#"{"localItems":[{"id":"tt1","source":"local"}],"externalItems":[{"id":"tt1","source":"external"},{"id":"tt2","source":"external"}]}"#,
            )
            .unwrap(),
        )
        .unwrap();
        let merged = result["merged"].as_array().unwrap();
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0]["source"], "local");
        assert_eq!(merged[1]["source"], "external");
        assert_eq!(merged[1]["id"], "tt2");
    }

    #[test]
    fn collection_import_validation_rejects_missing_id() {
        let result: Value = serde_json::from_str(
            &library_collection_import_validation_json(
                r#"{"collections":[{"title":"My List"},{"id":"c1","title":"Valid"}]}"#,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(result["isValid"], false);
        assert_eq!(result["validCollections"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn offline_grouping_partitions_by_status() {
        let result: Value = serde_json::from_str(
            &library_offline_grouping_json(
                r#"{"items":[{"id":"a","status":"ready"},{"id":"b","status":"downloading"},{"id":"c","status":"failed"},{"id":"d"}]}"#,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(result["ready"].as_array().unwrap().len(), 1);
        assert_eq!(result["downloading"].as_array().unwrap().len(), 1);
        assert_eq!(result["failed"].as_array().unwrap().len(), 1);
        assert_eq!(result["queued"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn progress_merge_preserves_existing_fields_when_incoming_is_null() {
        let result: Value = serde_json::from_str(
            &playback_progress_merge_plan_json(
                r#"{"existing":{"lastStreamUrl":"http://old","lastVideoId":"v1","timeOffset":1000},"incoming":{"lastVideoId":"v1","timeOffset":2000,"duration":5000}}"#,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(result["timeOffset"], 2000);
        assert_eq!(result["lastStreamUrl"], "http://old");
        assert_eq!(result["videoChanged"], false);
    }
}
