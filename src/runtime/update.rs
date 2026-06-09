use crate::runtime::effects::Effects;
use crate::runtime::msg::Msg;

/// Mirrors stremio-core's `Update` trait. The engine calls `update` when a
/// new `Msg` arrives; the model mutates its own state and returns a set of
/// side-effect requests for the platform to execute.
pub trait Update {
    fn update(&mut self, msg: &Msg) -> Effects;
}
