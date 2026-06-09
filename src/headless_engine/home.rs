use super::helpers::{active_profile_id, current_generation, normalize_error};
use super::{EffectResultInput, HeadlessEngine};
use crate::runtime::{EffectEnvelope, EffectKind};
use serde_json::{json, Value};

pub(super) fn dispatch_load(
    engine: &mut HeadlessEngine,
    profile: Option<Value>,
    language: Option<String>,
    force: Option<bool>,
) -> Vec<EffectEnvelope> {
    let generation = engine.bump_generation("homeGeneration");
    let profile_value = profile.unwrap_or_else(|| engine.state["profile"]["active"].clone());
    let profile_id = active_profile_id(&engine.state, &profile_value);
    engine.state["home"] = json!({
        "isLoading": true,
        "categories": [],
        "continueWatching": [],
        "userAddons": [],
        "metadataFeeds": [],
        "billboard": Value::Null,
        "error": Value::Null,
        "generation": generation
    });
    vec![engine.effect(
        EffectKind::ReadHomeBootstrap,
        generation,
        json!({
            "profileId": profile_id,
            "profile": profile_value,
            "language": language.unwrap_or_else(|| "en".to_string()),
            "force": force.unwrap_or(false)
        }),
    )]
}

pub(super) fn dispatch_direct_playback(
    engine: &mut HeadlessEngine,
    meta: Value,
    language: Option<String>,
    profile: Option<Value>,
) -> Vec<EffectEnvelope> {
    let generation = engine.bump_generation("playbackPrepGeneration");
    engine.state["home"]["isDirectLoading"] = json!(true);
    vec![engine.effect(
        EffectKind::PrepareDirectPlayback,
        generation,
        json!({
            "meta": meta,
            "language": language.unwrap_or_else(|| "en".to_string()),
            "profile": profile.unwrap_or(Value::Null)
        }),
    )]
}

pub(super) fn dispatch_catalog_page(
    engine: &mut HeadlessEngine,
    category_id: String,
    transport_url: Option<String>,
    content_type: String,
    catalog_id: String,
    skip: Option<i32>,
    genre: Option<String>,
    search: Option<String>,
) -> Vec<EffectEnvelope> {
    let generation = engine.bump_generation("homeGeneration");
    engine.state["home"]["paging"] = json!({
        "categoryId": category_id,
        "isLoading": true,
        "error": Value::Null
    });
    vec![engine.effect(
        EffectKind::FetchCatalogPage,
        generation,
        json!({
            "categoryId": engine.state["home"]["paging"]["categoryId"].clone(),
            "transportUrl": transport_url,
            "contentType": content_type,
            "catalogId": catalog_id,
            "skip": skip.unwrap_or(0).max(0),
            "genre": genre,
            "search": search
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
        "readHomeBootstrap" => {
            if generation == current_generation(&engine.state, "homeGeneration") {
                engine.state["home"]["isLoading"] = json!(false);
                if result.status == "ok" {
                    engine.state["home"]["categories"] = result
                        .value
                        .get("categories")
                        .cloned()
                        .unwrap_or_else(|| json!([]));
                    engine.state["home"]["continueWatching"] = result
                        .value
                        .get("continueWatching")
                        .cloned()
                        .unwrap_or_else(|| json!([]));
                    engine.state["home"]["watchlist"] = result
                        .value
                        .get("watchlist")
                        .cloned()
                        .unwrap_or_else(|| json!([]));
                    engine.state["home"]["userAddons"] = result
                        .value
                        .get("userAddons")
                        .cloned()
                        .unwrap_or_else(|| json!([]));
                    engine.state["home"]["metadataFeeds"] = result
                        .value
                        .get("metadataFeeds")
                        .cloned()
                        .unwrap_or_else(|| json!([]));
                    engine.state["home"]["billboard"] = result
                        .value
                        .get("billboard")
                        .cloned()
                        .unwrap_or(Value::Null);
                    engine.state["home"]["error"] = Value::Null;
                } else {
                    engine.state["home"]["error"] = normalize_error(result.error.clone());
                }
            }
        }
        "prepareDirectPlayback" => {
            if generation == current_generation(&engine.state, "playbackPrepGeneration") {
                engine.state["home"]["isDirectLoading"] = json!(false);
                if result.status == "ok" {
                    engine.state["player"]["directPlaybackTarget"] = result.value.clone();
                    engine.state["player"]["playerError"] = Value::Null;
                } else {
                    engine.state["player"]["directPlaybackTarget"] = Value::Null;
                    engine.state["player"]["playerError"] =
                        json!(super::helpers::error_code(&result.error));
                }
            }
        }
        "fetchCatalogPage" => {
            if generation == current_generation(&engine.state, "homeGeneration") {
                engine.state["home"]["paging"]["isLoading"] = json!(false);
                if result.status == "ok" {
                    engine.state["home"]["paging"]["items"] = result
                        .value
                        .get("items")
                        .cloned()
                        .unwrap_or_else(|| result.value.clone());
                    engine.state["home"]["paging"]["error"] = Value::Null;
                } else {
                    engine.state["home"]["paging"]["error"] =
                        normalize_error(result.error.clone());
                }
            }
        }
        _ => {}
    }
    vec![]
}
