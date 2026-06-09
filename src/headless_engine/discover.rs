use super::helpers::{active_profile_id, current_generation, normalize_error};
use super::{EffectResultInput, HeadlessEngine};
use crate::runtime::{EffectEnvelope, EffectKind};
use serde_json::{json, Value};

pub(super) fn dispatch_discover(
    engine: &mut HeadlessEngine,
    content_type: String,
    filters: Option<Value>,
    profile: Option<Value>,
    language: Option<String>,
) -> Vec<EffectEnvelope> {
    let generation = engine.bump_generation("discoverGeneration");
    let profile_value = profile.unwrap_or_else(|| engine.state["profile"]["active"].clone());
    engine.state["discover"] = json!({
        "contentType": content_type,
        "filters": filters.clone().unwrap_or(Value::Null),
        "isLoading": true,
        "results": [],
        "error": Value::Null,
        "generation": generation
    });
    vec![engine.effect(
        EffectKind::RunDiscover,
        generation,
        json!({
            "contentType": engine.state["discover"]["contentType"].clone(),
            "filters": engine.state["discover"]["filters"].clone(),
            "profileId": active_profile_id(&engine.state, &profile_value),
            "profile": profile_value,
            "language": language.unwrap_or_else(|| "en".to_string())
        }),
    )]
}

pub(super) fn dispatch_catalog_filters(
    engine: &mut HeadlessEngine,
    content_type: String,
    selected_catalog_key: Option<String>,
    profile: Option<Value>,
    language: Option<String>,
) -> Vec<EffectEnvelope> {
    let generation = engine.bump_generation("discoverGeneration");
    let profile_value = profile.unwrap_or_else(|| engine.state["profile"]["active"].clone());
    vec![engine.effect(
        EffectKind::ReadDiscoverCatalogFilters,
        generation,
        json!({
            "contentType": content_type,
            "selectedCatalogKey": selected_catalog_key,
            "profileId": active_profile_id(&engine.state, &profile_value),
            "profile": profile_value,
            "language": language.unwrap_or_else(|| "en".to_string())
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
        "runDiscover" => {
            if generation == current_generation(&engine.state, "discoverGeneration") {
                engine.state["discover"]["isLoading"] = json!(false);
                if result.status == "ok" {
                    engine.state["discover"]["results"] = result
                        .value
                        .get("results")
                        .cloned()
                        .unwrap_or_else(|| result.value.clone());
                    engine.state["discover"]["resultSources"] = result
                        .value
                        .get("resultSources")
                        .cloned()
                        .unwrap_or_else(|| json!({}));
                    engine.state["discover"]["error"] = Value::Null;
                } else {
                    engine.state["discover"]["error"] = normalize_error(result.error.clone());
                }
            }
        }
        "readDiscoverCatalogFilters" => {
            if generation == current_generation(&engine.state, "discoverGeneration") {
                if result.status == "ok" {
                    engine.state["discover"]["catalogs"] =
                        result.value.get("catalogs").cloned().unwrap_or_else(|| json!([]));
                    engine.state["discover"]["genres"] =
                        result.value.get("genres").cloned().unwrap_or_else(|| json!([]));
                    engine.state["discover"]["error"] = Value::Null;
                } else {
                    engine.state["discover"]["error"] = normalize_error(result.error.clone());
                }
            }
        }
        _ => {}
    }
    vec![]
}
