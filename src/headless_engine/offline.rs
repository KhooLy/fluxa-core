use super::helpers::{current_generation, normalize_error};
use super::{EffectResultInput, HeadlessEngine};
use crate::runtime::{EffectEnvelope, EffectKind};
use serde_json::{json, Value};

pub(super) fn dispatch(
    engine: &mut HeadlessEngine,
    meta: Value,
    stream: Value,
    video_id: Option<String>,
    video: Option<Value>,
    subtitle: Option<Value>,
    profile_id: Option<String>,
    language: Option<String>,
) -> Vec<EffectEnvelope> {
    let generation = engine.bump_generation("offlineGeneration");
    engine.state["offline"]["lastRequest"] = json!({
        "meta": meta,
        "stream": stream,
        "videoId": video_id,
        "video": video,
        "subtitle": subtitle,
        "profileId": profile_id,
        "language": language
    });
    vec![engine.effect(EffectKind::EnqueueOfflineDownload, generation, engine.state["offline"]["lastRequest"].clone())]
}

pub(super) fn complete(engine: &mut HeadlessEngine, generation: u64, result: &EffectResultInput) -> Vec<EffectEnvelope> {
    if generation == current_generation(&engine.state, "offlineGeneration") {
        if result.status == "ok" {
            engine.state["offline"]["lastEnqueued"] = result.value.clone();
            engine.state["offline"]["error"] = Value::Null;
        } else {
            engine.state["offline"]["error"] = normalize_error(result.error.clone());
        }
    }
    vec![]
}
