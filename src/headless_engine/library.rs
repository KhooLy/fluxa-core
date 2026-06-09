use super::helpers::{active_profile_id, current_generation, normalize_error, should_sync_watched_state};
use super::{EffectResultInput, HeadlessEngine};
use crate::runtime::{EffectEnvelope, EffectKind};
use serde_json::{json, Value};

pub(super) fn dispatch_profile_activated(
    engine: &mut HeadlessEngine,
    profile: Value,
) -> Vec<EffectEnvelope> {
    let profile_id = profile["id"]
        .as_str()
        .filter(|value| !value.is_empty())
        .unwrap_or("guest")
        .to_string();
    engine.state["profile"]["active"] = profile.clone();
    engine.state["profile"]["activeProfileId"] = json!(profile_id.clone());
    engine.state["library"]["activeProfileId"] = json!(profile_id);
    vec![]
}

pub(super) fn dispatch_hydrate(
    engine: &mut HeadlessEngine,
    profile_id: Option<String>,
) -> Vec<EffectEnvelope> {
    let generation = engine.bump_generation("libraryGeneration");
    let resolved_profile_id =
        profile_id.unwrap_or_else(|| active_profile_id(&engine.state, &Value::Null));
    engine.state["library"]["activeProfileId"] = json!(resolved_profile_id.clone());
    engine.state["library"]["isLoading"] = json!(true);
    engine.state["library"]["error"] = Value::Null;
    engine.state["library"]["generation"] = json!(generation);
    vec![engine.effect(
        EffectKind::ReadLibraryState,
        generation,
        json!({ "profileId": resolved_profile_id }),
    )]
}

pub(super) fn dispatch_toggle_watchlist(
    engine: &mut HeadlessEngine,
    item: Value,
) -> Vec<EffectEnvelope> {
    let generation = engine.bump_generation("libraryGeneration");
    let profile_id = active_profile_id(&engine.state, &Value::Null);
    engine.state["library"]["lastCommand"] = json!({
        "type": "toggleWatchlist",
        "item": item
    });
    vec![engine.effect(
        EffectKind::WriteLibraryCommand,
        generation,
        json!({
            "profileId": profile_id,
            "command": engine.state["library"]["lastCommand"].clone()
        }),
    )]
}

pub(super) fn dispatch_set_feedback(
    engine: &mut HeadlessEngine,
    id: String,
    value: Option<bool>,
    meta: Value,
) -> Vec<EffectEnvelope> {
    let generation = engine.bump_generation("libraryGeneration");
    vec![engine.effect(
        EffectKind::WriteFeedback,
        generation,
        json!({ "id": id, "value": value, "meta": meta }),
    )]
}

pub(super) fn dispatch_clear_progress(
    engine: &mut HeadlessEngine,
    profile: Option<Value>,
    meta: Value,
) -> Vec<EffectEnvelope> {
    let generation = engine.bump_generation("libraryGeneration");
    vec![engine.effect(
        EffectKind::ClearPlaybackProgress,
        generation,
        json!({ "profile": profile.unwrap_or(Value::Null), "meta": meta }),
    )]
}

#[allow(clippy::too_many_arguments)]
pub(super) fn dispatch_save_progress(
    engine: &mut HeadlessEngine,
    profile: Option<Value>,
    meta: Value,
    time_offset: i64,
    duration: i64,
    last_video_id: Option<String>,
    last_stream_index: Option<i32>,
    last_episode_name: Option<String>,
    last_episode_season: Option<i64>,
    last_episode_number: Option<i64>,
    last_episode_thumbnail: Option<String>,
    last_stream_url: Option<String>,
    last_stream_title: Option<String>,
    last_audio_language: Option<String>,
    last_subtitle_language: Option<String>,
    scrobble_trakt_pause: Option<bool>,
) -> Vec<EffectEnvelope> {
    let generation = engine.bump_generation("libraryGeneration");
    let profile_id = active_profile_id(&engine.state, &Value::Null);
    let progress = json!({
        "meta": meta,
        "timeOffset": time_offset.max(0),
        "duration": duration.max(0),
        "lastVideoId": last_video_id,
        "lastStreamIndex": last_stream_index,
        "lastEpisodeName": last_episode_name,
        "lastEpisodeSeason": last_episode_season,
        "lastEpisodeNumber": last_episode_number,
        "lastEpisodeThumbnail": last_episode_thumbnail,
        "lastStreamUrl": last_stream_url,
        "lastStreamTitle": last_stream_title,
        "lastAudioLanguage": last_audio_language,
        "lastSubtitleLanguage": last_subtitle_language
    });
    engine.state["library"]["pendingPlaybackProgress"] = progress.clone();
    vec![engine.effect(
        EffectKind::WritePlaybackProgress,
        generation,
        json!({
            "profileId": profile_id,
            "profile": profile.unwrap_or(Value::Null),
            "scrobbleTraktPause": scrobble_trakt_pause.unwrap_or(true),
            "progress": progress
        }),
    )]
}

pub(super) fn dispatch_mark_watched(
    engine: &mut HeadlessEngine,
    series_id: String,
    video_ids: Vec<String>,
    watched: Option<bool>,
    meta: Option<Value>,
    episodes: Option<Vec<Value>>,
    profile: Option<Value>,
) -> Vec<EffectEnvelope> {
    let generation = engine.bump_generation("libraryGeneration");
    let profile_id = active_profile_id(&engine.state, &Value::Null);
    let watched_value = watched.unwrap_or(true);
    let clean_video_ids: Vec<String> = video_ids
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .fold(Vec::new(), |mut acc, value| {
            if !acc.contains(&value) {
                acc.push(value);
            }
            acc
        });
    engine.state["library"]["lastCommand"] = json!({
        "type": "markWatched",
        "seriesId": series_id,
        "videoIds": clean_video_ids,
        "watched": watched_value
    });
    let mut effects = vec![engine.effect(
        EffectKind::WriteLibraryCommand,
        generation,
        json!({
            "profileId": profile_id,
            "command": engine.state["library"]["lastCommand"].clone()
        }),
    )];
    if should_sync_watched_state(profile.as_ref(), meta.as_ref()) {
        effects.push(engine.effect(
            EffectKind::SyncWatchedState,
            generation,
            json!({
                "profile": profile.unwrap_or(Value::Null),
                "meta": meta.unwrap_or(Value::Null),
                "episodes": episodes.unwrap_or_default(),
                "watched": watched_value
            }),
        ));
    }
    effects
}

pub(super) fn complete(
    engine: &mut HeadlessEngine,
    effect_type: &str,
    generation: u64,
    result: &EffectResultInput,
) -> Vec<EffectEnvelope> {
    match effect_type {
        "readLibraryState" => {
            if generation == current_generation(&engine.state, "libraryGeneration") {
                engine.state["library"]["isLoading"] = json!(false);
                if result.status == "ok" {
                    engine.state["library"]["watchlist"] = result
                        .value
                        .get("watchlist")
                        .cloned()
                        .unwrap_or_else(|| json!([]));
                    engine.state["library"]["continueWatching"] = result
                        .value
                        .get("continueWatching")
                        .cloned()
                        .unwrap_or_else(|| json!([]));
                    engine.state["library"]["liked"] = result
                        .value
                        .get("liked")
                        .cloned()
                        .unwrap_or_else(|| json!([]));
                    engine.state["library"]["watched"] = result
                        .value
                        .get("watched")
                        .cloned()
                        .unwrap_or_else(|| json!({}));
                    engine.state["library"]["error"] = Value::Null;
                } else {
                    engine.state["library"]["error"] = normalize_error(result.error.clone());
                }
            }
        }
        "writeLibraryCommand" => {
            if generation == current_generation(&engine.state, "libraryGeneration") {
                if result.status == "ok" {
                    engine.state["library"]["lastWrite"] = result.value.clone();
                    engine.state["library"]["lastWriteError"] = Value::Null;
                    if let Some(value) =
                        engine.state["library"]["lastWrite"].get("isInWatchlist")
                    {
                        engine.state["detail"]["isInWatchlist"] = value.clone();
                    }
                    if let Some(value) =
                        engine.state["library"]["lastWrite"].get("localWatchedVideoIds")
                    {
                        engine.state["detail"]["localWatchedVideoIds"] = value.clone();
                    }
                } else {
                    engine.state["library"]["lastWriteError"] =
                        normalize_error(result.error.clone());
                }
            }
        }
        "writeFeedback" => {
            if generation == current_generation(&engine.state, "libraryGeneration") {
                if result.status == "ok" {
                    engine.state["detail"]["feedback"] =
                        result.value.get("feedback").cloned().unwrap_or(Value::Null);
                    engine.state["library"]["lastWriteError"] = Value::Null;
                } else {
                    engine.state["library"]["lastWriteError"] =
                        normalize_error(result.error.clone());
                }
            }
        }
        "clearPlaybackProgress" => {
            if generation == current_generation(&engine.state, "libraryGeneration") {
                if result.status == "ok" {
                    engine.state["detail"]["savedPlayback"] = Value::Null;
                    engine.state["library"]["lastWriteError"] = Value::Null;
                    // Remove the dropped item from home.continueWatching so stale state
                    // doesn't reappear when the user navigates back to the home screen.
                    if let Some(dropped_id) = result.value.get("droppedId").and_then(Value::as_str) {
                        if let Some(cw) = engine.state["home"]["continueWatching"].as_array_mut() {
                            cw.retain(|item| {
                                item.get("id").and_then(Value::as_str) != Some(dropped_id)
                            });
                        }
                    }
                } else {
                    engine.state["library"]["lastWriteError"] =
                        normalize_error(result.error.clone());
                }
            }
        }
        "writePlaybackProgress" => {
            if generation == current_generation(&engine.state, "libraryGeneration") {
                if result.status == "ok" {
                    engine.state["library"]["savedPlaybackProgress"] =
                        engine.state["library"]["pendingPlaybackProgress"].clone();
                    engine.state["library"]["pendingPlaybackProgress"] = Value::Null;
                    engine.state["library"]["lastWriteError"] = Value::Null;
                } else {
                    engine.state["library"]["lastWriteError"] =
                        normalize_error(result.error.clone());
                }
            }
        }
        "syncWatchedState" => {
            if generation == current_generation(&engine.state, "libraryGeneration") {
                if result.status == "ok" {
                    engine.state["library"]["lastWatchedSync"] = result.value.clone();
                    engine.state["library"]["lastWatchedSyncError"] = Value::Null;
                } else {
                    engine.state["library"]["lastWatchedSyncError"] =
                        normalize_error(result.error.clone());
                }
            }
        }
        _ => {}
    }
    vec![]
}
