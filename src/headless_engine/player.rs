use super::helpers::{current_generation, error_code, normalize_error};
use super::{EffectResultInput, HeadlessEngine};
use crate::runtime::{EffectEnvelope, EffectKind};
use crate::{player_flow, stream_policy};
use serde_json::{json, Value};

#[allow(clippy::too_many_arguments)]
pub(super) fn dispatch_next_episode_prefetch(
    engine: &mut HeadlessEngine,
    content_type: String,
    series_id: String,
    next_video_id: String,
    title: Option<String>,
    original_name: Option<String>,
    year: Option<i32>,
    language: Option<String>,
    profile: Option<Value>,
) -> Vec<EffectEnvelope> {
    let already_prefetching = engine.state["player"]["prefetchingNextVideoId"]
        .as_str()
        .is_some_and(|v| v == next_video_id);
    let already_cached = engine.state["player"]["prefetchedNextEpisode"]["videoId"]
        .as_str()
        .is_some_and(|v| v == next_video_id);
    if already_prefetching || already_cached {
        return vec![];
    }

    let generation = current_generation(&engine.state, "playerGeneration");
    engine.state["player"]["prefetchingNextVideoId"] = json!(next_video_id);
    vec![engine.effect(
        EffectKind::PrefetchNextEpisodeStreams,
        generation,
        json!({
            "contentType": content_type,
            "seriesId": series_id,
            "nextVideoId": next_video_id,
            "title": title.unwrap_or_default(),
            "originalName": original_name,
            "year": year,
            "language": language.unwrap_or_else(|| "en".to_string()),
            "profile": profile.unwrap_or(Value::Null)
        }),
    )]
}

#[allow(clippy::too_many_arguments)]
pub(super) fn dispatch_load_streams(
    engine: &mut HeadlessEngine,
    content_type: String,
    id: String,
    current_video_id: Option<String>,
    initial_video_id: Option<String>,
    initial_streams: Option<Vec<Value>>,
    initial_stream_index: Option<i32>,
    saved_url: Option<String>,
    saved_title: Option<String>,
    source_selection_mode: Option<String>,
    regex_pattern: Option<String>,
    preferred_binge_group: Option<String>,
    title: Option<String>,
    original_name: Option<String>,
    year: Option<i32>,
    language: Option<String>,
    profile: Option<Value>,
) -> Vec<EffectEnvelope> {
    let generation = engine.bump_generation("playerGeneration");
    let mut initial_streams = initial_streams.unwrap_or_default();
    let initial_stream_index = initial_stream_index.unwrap_or(0);

    // If no initial streams were provided by the caller but we have a prefetch
    // cache hit for this video_id, inject those streams so playback can start
    // without waiting for a fresh fetch.
    let mut effective_initial_video_id = initial_video_id.clone();
    if initial_streams.is_empty() {
        let prefetched = &engine.state["player"]["prefetchedNextEpisode"];
        let cached_video_id = prefetched["videoId"].as_str().map(str::to_string);
        if cached_video_id.is_some() && cached_video_id == current_video_id {
            initial_streams = prefetched["streams"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            effective_initial_video_id = cached_video_id;
            engine.state["player"]["prefetchedNextEpisode"] = Value::Null;
        }
    }
    let player_flow_state = engine.state.get("player").cloned().unwrap_or_else(|| json!({}));
    let Some(flow) = player_flow::player_flow_dispatch_json(
        &player_flow_state.to_string(),
        &json!({
            "type": "loadStreamsRequested",
            "contentType": content_type,
            "id": id,
            "currentVideoId": current_video_id,
            "initialVideoId": effective_initial_video_id,
            "initialStreams": initial_streams,
            "initialStreamIndex": initial_stream_index
        })
        .to_string(),
    ) else {
        engine.state["player"]["playerError"] = json!("invalid_flow_action");
        return vec![];
    };
    let Ok(flow) = serde_json::from_str::<Value>(&flow) else {
        engine.state["player"]["playerError"] = json!("invalid_flow_result");
        return vec![];
    };
    engine.state["player"] = flow["state"].clone();
    engine.state["player"]["pendingStreamLoad"] = json!({
        "savedUrl": saved_url,
        "savedTitle": saved_title,
        "sourceSelectionMode": source_selection_mode.unwrap_or_else(|| "manual".to_string()),
        "regexPattern": regex_pattern,
        "preferredBingeGroup": preferred_binge_group,
        "initialStreams": initial_streams,
        "initialStreamIndex": initial_stream_index,
        "currentVideoId": current_video_id,
        "title": title,
        "originalName": original_name,
        "year": year,
        "language": language.unwrap_or_else(|| "en".to_string()),
        "profile": profile.unwrap_or(Value::Null)
    });
    flow["effects"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|effect| {
            let mut payload = effect.clone();
            if effect["type"].as_str() == Some("loadStreams") {
                payload["initialStreams"] =
                    engine.state["player"]["pendingStreamLoad"]["initialStreams"].clone();
                payload["title"] =
                    engine.state["player"]["pendingStreamLoad"]["title"].clone();
                payload["originalName"] =
                    engine.state["player"]["pendingStreamLoad"]["originalName"].clone();
                payload["year"] =
                    engine.state["player"]["pendingStreamLoad"]["year"].clone();
                payload["language"] =
                    engine.state["player"]["pendingStreamLoad"]["language"].clone();
                payload["profile"] =
                    engine.state["player"]["pendingStreamLoad"]["profile"].clone();
            }
            engine.effect_raw(effect["type"].as_str().unwrap_or("unknown"), generation, payload)
        })
        .collect()
}

pub(super) fn dispatch_streams_loaded(
    engine: &mut HeadlessEngine,
    streams: Vec<Value>,
    current_video_id: Option<String>,
    initial_stream_index: Option<i32>,
    saved_url: Option<String>,
    saved_title: Option<String>,
    source_selection_mode: Option<String>,
    regex_pattern: Option<String>,
    preferred_binge_group: Option<String>,
) -> Vec<EffectEnvelope> {
    let generation = current_generation(&engine.state, "playerGeneration");
    let player_flow_state = engine.state.get("player").cloned().unwrap_or_else(|| json!({}));
    let Some(flow) = player_flow::player_flow_dispatch_json(
        &player_flow_state.to_string(),
        &json!({
            "type": "streamsLoaded",
            "streams": streams,
            "currentVideoId": current_video_id,
            "initialStreamIndex": initial_stream_index.unwrap_or(0),
            "savedUrl": saved_url,
            "savedTitle": saved_title,
            "sourceSelectionMode": source_selection_mode,
            "regexPattern": regex_pattern,
            "preferredBingeGroup": preferred_binge_group
        })
        .to_string(),
    ) else {
        engine.state["player"]["playerError"] = json!("invalid_flow_action");
        return vec![];
    };
    let Ok(flow) = serde_json::from_str::<Value>(&flow) else {
        engine.state["player"]["playerError"] = json!("invalid_flow_result");
        return vec![];
    };
    engine.state["player"] = flow["state"].clone();
    engine.state["player"]["generation"] = json!(generation);
    vec![]
}

pub(super) fn dispatch_streams_failed(
    engine: &mut HeadlessEngine,
    err_code: Option<String>,
) -> Vec<EffectEnvelope> {
    let player_flow_state = engine.state.get("player").cloned().unwrap_or_else(|| json!({}));
    let Some(flow) = player_flow::player_flow_dispatch_json(
        &player_flow_state.to_string(),
        &json!({ "type": "streamsFailed", "errorCode": err_code }).to_string(),
    ) else {
        engine.state["player"]["playerError"] = json!("invalid_flow_action");
        return vec![];
    };
    let Ok(flow) = serde_json::from_str::<Value>(&flow) else {
        engine.state["player"]["playerError"] = json!("invalid_flow_result");
        return vec![];
    };
    engine.state["player"] = flow["state"].clone();
    vec![]
}

pub(super) fn dispatch_resolve_playback(
    engine: &mut HeadlessEngine,
    url: String,
    stream: Option<Value>,
    current_video_id: Option<String>,
    title: Option<String>,
) -> Vec<EffectEnvelope> {
    let generation = engine.bump_generation("playerGeneration");
    engine.state["player"]["currentUrl"] = json!(url);
    engine.state["player"]["resolvedUrl"] = Value::Null;
    engine.state["player"]["isBuffering"] = json!(true);
    engine.state["player"]["hasStartedPlaying"] = json!(false);
    engine.state["player"]["isVideoRendered"] = json!(false);
    engine.state["player"]["playerError"] = Value::Null;
    if stream_policy::is_torrent_playback_url(&url) {
        let stream_value = stream.unwrap_or(Value::Null);
        let file_idx = stream_value["fileIdx"].as_i64();
        let preferred_filename = stream_value["effectiveFilename"]
            .as_str()
            .map(ToString::to_string);
        vec![engine.effect(
            EffectKind::StartTorrentStream,
            generation,
            json!({
                "url": url,
                "stream": stream_value.clone(),
                "currentVideoId": current_video_id,
                "title": title.unwrap_or_else(|| "Fluxa".to_string()),
                "fileIdx": file_idx,
                "preferredFilename": preferred_filename,
                "sources": stream_value["sources"].as_array().cloned().unwrap_or_default()
            }),
        )]
    } else {
        engine.state["player"]["resolvedUrl"] = engine.state["player"]["currentUrl"].clone();
        engine.state["player"]["isBuffering"] = json!(false);
        vec![engine.effect(
            EffectKind::StopTorrent,
            generation,
            json!({ "reason": "directPlayback" }),
        )]
    }
}

pub(super) fn dispatch_scrobble(
    engine: &mut HeadlessEngine,
    token: String,
    meta_type: String,
    item_id: String,
    progress: f64,
    action_name: String,
    profile: Option<Value>,
) -> Vec<EffectEnvelope> {
    let generation = engine.bump_generation("playerGeneration");
    vec![engine.effect(
        EffectKind::EnqueueTraktScrobble,
        generation,
        json!({
            "token": token,
            "metaType": meta_type,
            "itemId": item_id,
            "progress": progress,
            "actionName": action_name,
            "profile": profile.unwrap_or(Value::Null)
        }),
    )]
}

pub(super) fn dispatch_intro_segments(
    engine: &mut HeadlessEngine,
    imdb_id: String,
    season: i32,
    episode: i32,
    title: Option<String>,
    use_intro_db: bool,
    use_ani_skip: bool,
) -> Vec<EffectEnvelope> {
    let generation = engine.bump_generation("introGeneration");
    vec![engine.effect(
        EffectKind::FetchIntroSegments,
        generation,
        json!({
            "imdbId": imdb_id,
            "season": season,
            "episode": episode,
            "title": title,
            "useIntroDb": use_intro_db,
            "useAniSkip": use_ani_skip
        }),
    )]
}

pub(super) fn dispatch_intro_imdb_id(
    engine: &mut HeadlessEngine,
    meta: Value,
    video_id: Option<String>,
    language: Option<String>,
) -> Vec<EffectEnvelope> {
    let generation = engine.bump_generation("introGeneration");
    vec![engine.effect(
        EffectKind::ResolveIntroImdbId,
        generation,
        json!({
            "meta": meta,
            "videoId": video_id,
            "language": language.unwrap_or_else(|| "en".to_string())
        }),
    )]
}

pub(super) fn dispatch_subtitle_load(
    engine: &mut HeadlessEngine,
    stream: Value,
    content_type: String,
    id: String,
    extra_args: Option<String>,
) -> Vec<EffectEnvelope> {
    let generation = engine.bump_generation("playerGeneration");
    engine.state["player"]["subtitleLoading"] = json!(true);
    vec![engine.effect(
        EffectKind::FetchSubtitles,
        generation,
        json!({
            "stream": stream,
            "contentType": content_type,
            "id": id,
            "extraArgs": extra_args.unwrap_or_default()
        }),
    )]
}

pub(super) fn complete(
    engine: &mut HeadlessEngine,
    effect_type: &str,
    generation: u64,
    result: &EffectResultInput,
) -> Vec<EffectEnvelope> {
    match effect_type {
        "loadStreams" => {
            if generation == current_generation(&engine.state, "playerGeneration") {
                let pending = engine.state["player"]["pendingStreamLoad"].clone();
                if result.status == "ok" {
                    dispatch_streams_loaded(
                        engine,
                        result.value.as_array().cloned().unwrap_or_default(),
                        pending["currentVideoId"].as_str().map(ToString::to_string),
                        Some(pending["initialStreamIndex"].as_i64().unwrap_or(0) as i32),
                        pending["savedUrl"].as_str().map(ToString::to_string),
                        pending["savedTitle"].as_str().map(ToString::to_string),
                        pending["sourceSelectionMode"].as_str().map(ToString::to_string),
                        pending["regexPattern"].as_str().map(ToString::to_string),
                        pending["preferredBingeGroup"].as_str().map(ToString::to_string),
                    );
                } else {
                    dispatch_streams_failed(engine, Some(error_code(&result.error)));
                }
                engine.state["player"]["pendingStreamLoad"] = Value::Null;
            }
        }
        "startTorrentStream" => {
            if generation == current_generation(&engine.state, "playerGeneration") {
                if result.status == "ok" {
                    engine.state["player"]["resolvedUrl"] =
                        result.value.get("url").cloned().unwrap_or(Value::Null);
                    engine.state["player"]["isBuffering"] = json!(false);
                    engine.state["player"]["playerError"] = Value::Null;
                } else {
                    engine.state["player"]["resolvedUrl"] = Value::Null;
                    engine.state["player"]["isBuffering"] = json!(false);
                    engine.state["player"]["playerError"] = json!(error_code(&result.error));
                }
            }
        }
        "enqueueTraktScrobble" => {
            if generation == current_generation(&engine.state, "playerGeneration") {
                if result.status == "ok" {
                    engine.state["player"]["lastScrobble"] = result.value.clone();
                    engine.state["player"]["playerError"] = Value::Null;
                } else {
                    engine.state["player"]["playerError"] = json!(error_code(&result.error));
                }
            }
        }
        "stopTorrent" => {
            if generation == current_generation(&engine.state, "playerGeneration")
                && result.status != "ok"
            {
                engine.state["player"]["stopTorrentWarning"] =
                    normalize_error(result.error.clone());
            }
        }
        "fetchIntroSegments" => {
            if generation == current_generation(&engine.state, "introGeneration") {
                if result.status == "ok" {
                    engine.state["player"]["introSegments"] = result.value.clone();
                    engine.state["player"]["playerError"] = Value::Null;
                } else {
                    engine.state["player"]["introSegments"] = json!([]);
                    engine.state["player"]["playerError"] = json!(error_code(&result.error));
                }
            }
        }
        "resolveIntroImdbId" => {
            if generation == current_generation(&engine.state, "introGeneration") {
                if result.status == "ok" {
                    engine.state["player"]["introImdbId"] = result.value.clone();
                    engine.state["player"]["playerError"] = Value::Null;
                } else {
                    engine.state["player"]["introImdbId"] = Value::Null;
                    engine.state["player"]["playerError"] = json!(error_code(&result.error));
                }
            }
        }
        "fetchSubtitles" => {
            if generation == current_generation(&engine.state, "playerGeneration") {
                engine.state["player"]["subtitleLoading"] = json!(false);
                if result.status == "ok" {
                    engine.state["player"]["subtitles"] = result
                        .value
                        .get("subtitles")
                        .cloned()
                        .unwrap_or_else(|| result.value.clone());
                    engine.state["player"]["playerError"] = Value::Null;
                } else {
                    engine.state["player"]["playerError"] = json!(error_code(&result.error));
                }
            }
        }
        "prefetchNextEpisodeStreams" => {
            // Only accept if the prefetch generation is still current (player hasn't moved on).
            if generation == current_generation(&engine.state, "playerGeneration") {
                if result.status == "ok" {
                    let prefetched_video_id = engine.state["player"]["prefetchingNextVideoId"].clone();
                    engine.state["player"]["prefetchedNextEpisode"] = json!({
                        "videoId": prefetched_video_id,
                        "streams": result.value.get("streams").cloned().unwrap_or_else(|| json!([]))
                    });
                }
                engine.state["player"]["prefetchingNextVideoId"] = Value::Null;
            }
        }
        _ => {}
    }
    vec![]
}
