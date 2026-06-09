use super::HeadlessEngine;
use crate::runtime::EffectEnvelope;
use serde_json::{json, Value};

pub(super) fn dispatch(engine: &mut HeadlessEngine, route: String, params: Option<Value>) -> Vec<EffectEnvelope> {
    engine.state["navigation"] = json!({
        "route": route,
        "params": params.unwrap_or(Value::Null)
    });
    vec![]
}
