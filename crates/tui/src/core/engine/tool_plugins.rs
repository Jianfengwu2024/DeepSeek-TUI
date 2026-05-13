//! Optional host-side tool plugins for the engine.
//!
//! This layer keeps methodology-specific or optional tool bundles out of the
//! base registry construction path. The engine evaluates a small registry of
//! factories and lets each enabled plugin extend the `ToolRegistryBuilder`.

use super::plugin_metadata::{PluginDefaultState, PluginMetadata, PluginStability};
use super::*;

type ToolPluginEnabledPredicate = fn(&EngineConfig, AppMode) -> bool;
type ToolPluginRegistrar = fn(ToolRegistryBuilder) -> ToolRegistryBuilder;

/// Registry entry for an optional tool plugin bundle.
pub(crate) struct ToolPluginFactory {
    /// Stable descriptive metadata for this plugin bundle.
    pub(crate) metadata: PluginMetadata,
    /// Whether the plugin should extend the registry for this engine + mode.
    pub(crate) enabled: ToolPluginEnabledPredicate,
    /// Registration hook that extends the builder with the plugin's tools.
    pub(crate) register: ToolPluginRegistrar,
}

fn triadmind_tool_plugin_enabled(config: &EngineConfig, mode: AppMode) -> bool {
    mode != AppMode::Plan && config.triadmind_mode.tools_enabled()
}

pub(crate) fn tool_plugin_registry() -> &'static [ToolPluginFactory] {
    static REGISTRY: [ToolPluginFactory; 1] = [ToolPluginFactory {
        metadata: PluginMetadata {
            id: "triadmind_tools",
            description: "TriadMind tool bundle: sync, verify, and rules management.",
            stability: PluginStability::Experimental,
            default_state: PluginDefaultState::Disabled,
        },
        enabled: triadmind_tool_plugin_enabled,
        register: crate::tools::triadmind::register_triadmind_tools,
    }];
    &REGISTRY
}

pub(crate) fn apply_optional_tool_plugins(
    engine: &Engine,
    mode: AppMode,
    mut builder: ToolRegistryBuilder,
) -> ToolRegistryBuilder {
    for factory in tool_plugin_registry() {
        let enabled = (factory.enabled)(&engine.config, mode);
        tracing::debug!(
            plugin = factory.metadata.id,
            stability = ?factory.metadata.stability,
            default_state = ?factory.metadata.default_state,
            enabled,
            mode = ?mode,
            "evaluated tool plugin factory"
        );
        if enabled {
            builder = (factory.register)(builder);
        }
    }
    builder
}
