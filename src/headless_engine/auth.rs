use super::helpers::{current_generation, normalize_error};
use super::{EffectResultInput, HeadlessEngine};
use crate::runtime::{EffectEnvelope, EffectKind};
use serde_json::{json, Value};

pub(super) fn dispatch_flow(
    engine: &mut HeadlessEngine,
    provider: String,
    mode: String,
) -> Vec<EffectEnvelope> {
    let generation = engine.bump_generation("authGeneration");
    engine.state["auth"] = json!({
        "provider": provider,
        "mode": mode,
        "isLoading": true,
        "error": Value::Null,
        "generation": generation
    });
    vec![engine.effect(
        EffectKind::RunAuthFlow,
        generation,
        json!({
            "provider": engine.state["auth"]["provider"].clone(),
            "mode": engine.state["auth"]["mode"].clone()
        }),
    )]
}

pub(super) fn dispatch_exchange(
    engine: &mut HeadlessEngine,
    provider: String,
    code: String,
    code_verifier: Option<String>,
    profile: Option<Value>,
) -> Vec<EffectEnvelope> {
    let generation = engine.bump_generation("authGeneration");
    engine.state["auth"] = json!({
        "provider": provider,
        "mode": "exchange",
        "isLoading": true,
        "error": Value::Null,
        "generation": generation
    });
    vec![engine.effect(
        EffectKind::ExchangeAuthCode,
        generation,
        json!({
            "provider": engine.state["auth"]["provider"].clone(),
            "code": code,
            "codeVerifier": code_verifier,
            "profile": profile.unwrap_or_else(|| engine.state["profile"]["active"].clone())
        }),
    )]
}

pub(super) fn dispatch_token_refresh(
    engine: &mut HeadlessEngine,
    provider: String,
    profile: Value,
) -> Vec<EffectEnvelope> {
    let generation = engine.bump_generation("authGeneration");
    engine.state["auth"] = json!({
        "provider": provider,
        "mode": "refresh",
        "isLoading": true,
        "error": Value::Null,
        "generation": generation
    });
    vec![engine.effect(
        EffectKind::RefreshAuthToken,
        generation,
        json!({
            "provider": engine.state["auth"]["provider"].clone(),
            "profile": profile
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
        "runAuthFlow" => {
            if generation == current_generation(&engine.state, "authGeneration") {
                engine.state["auth"]["isLoading"] = json!(false);
                if result.status == "ok" {
                    engine.state["auth"]["result"] = result.value.clone();
                    engine.state["auth"]["error"] = Value::Null;
                } else {
                    engine.state["auth"]["error"] = normalize_error(result.error.clone());
                }
            }
        }
        "exchangeAuthCode" | "refreshAuthToken" => {
            if generation == current_generation(&engine.state, "authGeneration") {
                engine.state["auth"]["isLoading"] = json!(false);
                if result.status == "ok" {
                    engine.state["auth"]["result"] = result.value.clone();
                    let updated_profile =
                        engine.state["auth"]["result"].get("profile").cloned();
                    if let Some(updated_profile) = updated_profile {
                        engine.state["profile"]["active"] = updated_profile.clone();
                        engine.state["home"]["activeProfile"] = updated_profile;
                    }
                    engine.state["auth"]["error"] = Value::Null;
                } else {
                    engine.state["auth"]["error"] = normalize_error(result.error.clone());
                }
            }
        }
        _ => {}
    }
    vec![]
}
