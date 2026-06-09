pub mod action;
pub mod event;
pub mod internal;

pub use action::Action;
pub use event::Event;
pub use internal::Internal;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "payload", rename_all = "camelCase")]
pub enum Msg {
    Action(Action),
    Internal(Internal),
    Event(Event),
}
