use super::helpers::{active_profile_id, current_generation, normalize_error};
use super::{EffectResultInput, HeadlessEngine};
use crate::runtime::{EffectEnvelope, EffectKind};
use serde_json::{json, Value};

pub(super) fn dispatch(
    engine: &mut HeadlessEngine,
    query: String,
    profile: Option<Value>,
    language: Option<String>,
) -> Vec<EffectEnvelope> {
    let generation = engine.bump_generation("searchGeneration");
    let profile_value = profile.unwrap_or_else(|| engine.state["profile"]["active"].clone());
    engine.state["search"] = json!({
        "query": query,
        "isLoading": true,
        "results": [],
        "error": Value::Null,
        "generation": generation
    });
    vec![engine.effect(
        EffectKind::RunSearch,
        generation,
        json!({
            "query": engine.state["search"]["query"].clone(),
            "profileId": active_profile_id(&engine.state, &profile_value),
            "profile": profile_value,
            "language": language.unwrap_or_else(|| "en".to_string())
        }),
    )]
}

pub(super) fn complete(engine: &mut HeadlessEngine, generation: u64, result: &EffectResultInput) -> Vec<EffectEnvelope> {
    if generation == current_generation(&engine.state, "searchGeneration") {
        engine.state["search"]["isLoading"] = json!(false);
        if result.status == "ok" {
            engine.state["search"]["results"] = result
                .value
                .get("results")
                .cloned()
                .unwrap_or_else(|| result.value.clone());
            engine.state["search"]["categories"] = result
                .value
                .get("categories")
                .cloned()
                .unwrap_or_else(|| json!([]));
            engine.state["search"]["grouping"] =
                result.value.get("grouping").cloned().unwrap_or(Value::Null);
            engine.state["search"]["error"] = Value::Null;
        } else {
            engine.state["search"]["error"] = normalize_error(result.error.clone());
        }
    }
    vec![]
}
