use super::helpers::{current_generation, normalize_error, upsert_by_key};
use super::{EffectResultInput, HeadlessEngine};
use crate::runtime::{EffectEnvelope, EffectKind};
use serde_json::{json, Value};

pub(super) fn dispatch_install(
    engine: &mut HeadlessEngine,
    transport_url: String,
    force_refresh: Option<bool>,
) -> Vec<EffectEnvelope> {
    let generation = engine.bump_generation("addonGeneration");
    engine.state["addons"]["installing"] = json!(transport_url.clone());
    engine.state["addons"]["error"] = Value::Null;
    vec![engine.effect(
        EffectKind::FetchAddonManifest,
        generation,
        json!({
            "transportUrl": transport_url,
            "forceRefresh": force_refresh.unwrap_or(false)
        }),
    )]
}

pub(super) fn dispatch_refresh(
    engine: &mut HeadlessEngine,
    profile: Option<Value>,
    force_refresh: Option<bool>,
) -> Vec<EffectEnvelope> {
    let generation = engine.bump_generation("addonGeneration");
    let profile_value = profile.unwrap_or_else(|| engine.state["profile"]["active"].clone());
    vec![engine.effect(
        EffectKind::RefreshInstalledAddons,
        generation,
        json!({
            "profile": profile_value,
            "forceRefresh": force_refresh.unwrap_or(true)
        }),
    )]
}

pub(super) fn dispatch_resource(
    engine: &mut HeadlessEngine,
    transport_url: String,
    resource: String,
    content_type: String,
    id: String,
    extra: Option<Value>,
) -> Vec<EffectEnvelope> {
    let generation = engine.bump_generation("addonGeneration");
    engine.state["addons"]["lastResourceRequest"] = json!({
        "transportUrl": transport_url,
        "resource": resource,
        "contentType": content_type,
        "id": id,
        "extra": extra.unwrap_or(Value::Null)
    });
    vec![engine.effect(
        EffectKind::FetchAddonResource,
        generation,
        engine.state["addons"]["lastResourceRequest"].clone(),
    )]
}

pub(super) fn complete(
    engine: &mut HeadlessEngine,
    effect_type: &str,
    generation: u64,
    result: &EffectResultInput,
) -> Vec<EffectEnvelope> {
    match effect_type {
        "fetchAddonManifest" => {
            if generation == current_generation(&engine.state, "addonGeneration") {
                if result.status == "ok" {
                    let manifest = result.value.clone();
                    let id = manifest["transportUrl"]
                        .as_str()
                        .or_else(|| manifest["id"].as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    upsert_by_key(&mut engine.state["addons"]["installed"], "id", &id, manifest);
                    engine.state["addons"]["installing"] = Value::Null;
                    engine.state["addons"]["error"] = Value::Null;
                } else {
                    engine.state["addons"]["installing"] = Value::Null;
                    engine.state["addons"]["error"] = normalize_error(result.error.clone());
                }
            }
        }
        "refreshInstalledAddons" => {
            if generation == current_generation(&engine.state, "addonGeneration") {
                if result.status == "ok" {
                    engine.state["home"]["userAddons"] = result
                        .value
                        .get("addons")
                        .cloned()
                        .unwrap_or_else(|| result.value.clone());
                    engine.state["addons"]["installed"] =
                        engine.state["home"]["userAddons"].clone();
                    engine.state["addons"]["error"] = Value::Null;
                } else {
                    engine.state["addons"]["error"] = normalize_error(result.error.clone());
                }
            }
        }
        "fetchAddonResource" => {
            if generation == current_generation(&engine.state, "addonGeneration") {
                if result.status == "ok" {
                    engine.state["addons"]["lastResourceResult"] = result.value.clone();
                    engine.state["addons"]["error"] = Value::Null;
                } else {
                    engine.state["addons"]["lastResourceResult"] = Value::Null;
                    engine.state["addons"]["error"] = normalize_error(result.error.clone());
                }
            }
        }
        _ => {}
    }
    vec![]
}
