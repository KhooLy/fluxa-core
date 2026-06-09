use crate::types::addon::AddonManifest;

/// Mirrors stremio-core's `AddonTransport` trait.
/// Rust should never implement this by doing HTTP directly — the platform (Kotlin/JS) executes HTTP and passes results back via completeEffect.
pub trait AddonTransport {
    fn manifest(&self) -> Option<&AddonManifest>;

    fn resource(
        &self,
        resource: &str,
        type_: &str,
        id: &str,
        extra: Option<&str>,
    ) -> Result<String, String>;
}
