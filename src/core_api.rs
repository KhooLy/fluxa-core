use crate::{headless_engine, offline_download, player_policy, stream_policy};

pub struct FluxaCore;

// desktop calls these directly rather than through ffi::core_invoke, so each
// one needs its own panic guard rather than inheriting core_invoke's.
fn guard<T>(default: T, f: impl FnOnce() -> T) -> T {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).unwrap_or(default)
}

impl FluxaCore {
    pub fn create_headless_engine(initial_json: &str) -> u64 {
        guard(0, || headless_engine::create_headless_engine(initial_json))
    }

    pub fn headless_engine_snapshot_json(handle: u64) -> Option<String> {
        guard(None, || headless_engine::headless_engine_snapshot_json(handle))
    }

    pub fn headless_engine_dispatch_json(handle: u64, action_json: &str) -> Option<String> {
        guard(None, || headless_engine::headless_engine_dispatch_json(handle, action_json))
    }

    pub fn headless_engine_complete_effect_json(handle: u64, result_json: &str) -> Option<String> {
        guard(None, || headless_engine::headless_engine_complete_effect_json(handle, result_json))
    }

    pub fn stream_playback_info_json(stream_json: &str) -> Option<String> {
        guard(None, || stream_policy::stream_playback_info_json(stream_json))
    }

    pub fn torrent_runtime_info_json(request_json: &str) -> Option<String> {
        guard(None, || stream_policy::torrent_runtime_info_json(request_json))
    }

    pub fn player_buffer_targets_json(request_json: &str) -> Option<String> {
        guard(None, || player_policy::player_buffer_targets_json(request_json))
    }

    pub fn offline_download_plan_json(request_json: &str) -> Option<String> {
        guard(None, || offline_download::offline_download_plan_json(request_json))
    }
}

#[cfg(test)]
mod tests {
    use super::guard;

    #[test]
    fn guard_recovers_from_a_panic_instead_of_propagating_it() {
        let result = guard(42, || std::panic::panic_any("boom"));
        assert_eq!(result, 42);
    }
}
