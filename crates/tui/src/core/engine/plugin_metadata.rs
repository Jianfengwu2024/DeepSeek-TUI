//! Shared metadata types for optional engine plugins.

/// Stability marker for optional host-side plugins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum PluginStability {
    Experimental,
    Stable,
}

/// Default host-side enablement state for an optional plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum PluginDefaultState {
    Disabled,
    Enabled,
}

/// Stable metadata describing a host-side plugin extension point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PluginMetadata {
    pub(crate) id: &'static str,
    pub(crate) description: &'static str,
    pub(crate) stability: PluginStability,
    pub(crate) default_state: PluginDefaultState,
}
