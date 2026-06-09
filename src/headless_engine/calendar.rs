use super::helpers::{active_profile_id, current_generation, normalize_error};
use super::{EffectResultInput, HeadlessEngine};
use crate::runtime::{EffectEnvelope, EffectKind};
use serde_json::{json, Value};

pub(super) fn dispatch(
    engine: &mut HeadlessEngine,
    profile: Option<Value>,
    year: i32,
    month: i32,
    planned_items: Option<Value>,
) -> Vec<EffectEnvelope> {
    let generation = engine.bump_generation("calendarGeneration");
    let profile_value = profile.unwrap_or_else(|| engine.state["profile"]["active"].clone());
    engine.state["calendar"] = json!({
        "year": year,
        "month": month,
        "isLoading": true,
        "items": [],
        "error": Value::Null,
        "generation": generation
    });
    vec![engine.effect(
        EffectKind::ReadCalendarMonth,
        generation,
        json!({
            "profileId": active_profile_id(&engine.state, &profile_value),
            "profile": profile_value,
            "year": year,
            "month": month.clamp(1, 12),
            "plannedItems": planned_items.unwrap_or_else(|| json!([]))
        }),
    )]
}

pub(super) fn complete(
    engine: &mut HeadlessEngine,
    generation: u64,
    result: &EffectResultInput,
    effect: &Value,
) -> Vec<EffectEnvelope> {
    if generation != current_generation(&engine.state, "calendarGeneration") {
        return vec![];
    }
    engine.state["calendar"]["isLoading"] = json!(false);
    if result.status == "ok" {
        let items = result.value.get("items").cloned().unwrap_or_else(|| result.value.clone());
        let local_items = result.value.get("localItems").cloned().unwrap_or_else(|| json!([]));
        let external_items = result.value.get("externalItems").cloned().unwrap_or_else(|| json!([]));
        engine.state["calendar"]["items"] = items.clone();
        engine.state["calendar"]["localItems"] = local_items;
        engine.state["calendar"]["externalItems"] = external_items.clone();
        engine.state["calendar"]["error"] = Value::Null;
        let mut follow_up = vec![
            engine.effect(EffectKind::UpdateCalendarWidget, generation, json!({
                "profile": effect["payload"]["profile"].clone(),
                "items": items.clone()
            })),
            engine.effect(EffectKind::NotifyReleasedEpisodes, generation, json!({
                "profile": effect["payload"]["profile"].clone(),
                "items": items
            })),
        ];
        if external_items.as_array().is_some_and(|i| !i.is_empty()) {
            follow_up.push(engine.effect(EffectKind::ReplaceExternalContinueWatching, generation, json!({
                "profileId": effect["payload"]["profileId"].clone(),
                "items": external_items
            })));
        }
        follow_up
    } else {
        engine.state["calendar"]["error"] = normalize_error(result.error.clone());
        vec![]
    }
}
