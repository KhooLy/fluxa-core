use super::helpers::{active_profile_id, current_generation, normalize_error};
use super::{EffectResultInput, HeadlessEngine};
use crate::runtime::{EffectEnvelope, EffectKind};
use serde_json::{json, Value};

pub(super) fn dispatch_external_sync(
    engine: &mut HeadlessEngine,
    provider: String,
    profile: Option<Value>,
    language: Option<String>,
) -> Vec<EffectEnvelope> {
    let generation = engine.bump_generation("syncGeneration");
    let profile_value = profile.unwrap_or_else(|| engine.state["profile"]["active"].clone());
    engine.state["sync"] = json!({
        "provider": provider,
        "isLoading": true,
        "error": Value::Null,
        "generation": generation
    });
    vec![engine.effect(
        EffectKind::RunExternalSync,
        generation,
        json!({
            "provider": engine.state["sync"]["provider"].clone(),
            "profileId": active_profile_id(&engine.state, &profile_value),
            "profile": profile_value,
            "language": language.unwrap_or_else(|| "en".to_string())
        }),
    )]
}

pub(super) fn dispatch_integration_sync(
    engine: &mut HeadlessEngine,
    provider: String,
    profile: Value,
    language: Option<String>,
) -> Vec<EffectEnvelope> {
    let generation = engine.bump_generation("syncGeneration");
    engine.state["sync"] = json!({
        "provider": provider,
        "isLoading": true,
        "error": Value::Null,
        "generation": generation
    });
    vec![engine.effect(
        EffectKind::SyncExternalIntegration,
        generation,
        json!({
            "provider": engine.state["sync"]["provider"].clone(),
            "profile": profile,
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
        "runExternalSync" => {
            if generation == current_generation(&engine.state, "syncGeneration") {
                engine.state["sync"]["isLoading"] = json!(false);
                if result.status == "ok" {
                    engine.state["sync"]["snapshot"] = result.value.clone();
                    engine.state["sync"]["error"] = Value::Null;
                } else {
                    engine.state["sync"]["error"] = normalize_error(result.error.clone());
                }
            }
        }
        "syncExternalIntegration" => {
            if generation == current_generation(&engine.state, "syncGeneration") {
                engine.state["sync"]["isLoading"] = json!(false);
                if result.status == "ok" {
                    let updated_profile =
                        result.value.get("profile").cloned().unwrap_or(Value::Null);
                    engine.state["sync"]["snapshot"] =
                        result.value.get("snapshot").cloned().unwrap_or(Value::Null);
                    if !updated_profile.is_null() {
                        engine.state["profile"]["active"] = updated_profile.clone();
                        engine.state["home"]["activeProfile"] = updated_profile;
                    }
                    engine.state["home"]["externalContinueWatching"] = result
                        .value
                        .get("externalContinueWatching")
                        .cloned()
                        .unwrap_or_else(|| json!([]));
                    engine.state["sync"]["error"] = Value::Null;
                } else {
                    engine.state["sync"]["error"] = normalize_error(result.error.clone());
                }
            }
        }
        _ => {}
    }
    vec![]
}
