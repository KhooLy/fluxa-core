use super::helpers::{current_generation, normalize_error};
use super::{EffectResultInput, HeadlessEngine};
use crate::runtime::{EffectEnvelope, EffectKind};
use serde_json::{json, Value};

pub(super) fn dispatch(engine: &mut HeadlessEngine, key: String, value: Value) -> Vec<EffectEnvelope> {
    let generation = engine.bump_generation("settingsGeneration");
    engine.state["settings"]["values"][key.as_str()] = value.clone();
    vec![engine.effect(EffectKind::WriteSettings, generation, json!({ "key": key, "value": value }))]
}

pub(super) fn complete(engine: &mut HeadlessEngine, generation: u64, result: &EffectResultInput) -> Vec<EffectEnvelope> {
    if generation == current_generation(&engine.state, "settingsGeneration") {
        if result.status != "ok" {
            engine.state["settings"]["lastWriteError"] = normalize_error(result.error.clone());
        } else {
            engine.state["settings"]["lastWriteError"] = Value::Null;
        }
    }
    vec![]
}
