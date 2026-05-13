//! Optional post-edit governance plugins for the engine.
//!
//! Unlike built-in diagnostics such as LSP, governance layers are intended to
//! be host-pluggable and methodology-optional. The engine asks enabled plugins
//! for advisory messages after successful file edits, then injects those
//! messages into the next model request as synthetic user text.

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;

use super::plugin_metadata::{PluginDefaultState, PluginMetadata, PluginStability};
use super::*;

type GovernancePluginBuilder = fn(&EngineConfig) -> Option<Arc<dyn PostEditGovernancePlugin>>;

#[async_trait]
pub(crate) trait PostEditGovernancePlugin: Send + Sync {
    /// Stable plugin identifier for diagnostics and tests.
    fn name(&self) -> &'static str;

    /// Inspect a successful file edit and return any advisory messages that
    /// should be shown to the model on its next reasoning step.
    async fn on_successful_edit(
        &self,
        workspace_root: &Path,
        tool_name: &str,
        tool_input: &serde_json::Value,
    ) -> Vec<String>;
}

/// Registry entry for a host-side governance plugin.
pub(crate) struct GovernancePluginFactory {
    /// Stable descriptive metadata for this plugin.
    pub(crate) metadata: PluginMetadata,
    /// Builder that decides whether the plugin is enabled for this engine.
    pub(crate) build: GovernancePluginBuilder,
}

pub(crate) fn post_edit_governance_registry() -> &'static [GovernancePluginFactory] {
    static REGISTRY: [GovernancePluginFactory; 1] = [GovernancePluginFactory {
        metadata: PluginMetadata {
            id: "triadmind",
            description: "TriadMind architecture governance advisories after successful source edits.",
            stability: PluginStability::Experimental,
            default_state: PluginDefaultState::Disabled,
        },
        build: super::triadmind_hooks::build_triadmind_governance_plugin,
    }];
    &REGISTRY
}

pub(crate) fn build_post_edit_governance_plugins(
    config: &EngineConfig,
) -> Vec<Arc<dyn PostEditGovernancePlugin>> {
    post_edit_governance_registry()
        .iter()
        .filter_map(|factory| {
            let plugin = (factory.build)(config);
            tracing::debug!(
                plugin = factory.metadata.id,
                stability = ?factory.metadata.stability,
                default_state = ?factory.metadata.default_state,
                enabled = plugin.is_some(),
                "evaluated governance plugin factory"
            );
            plugin
        })
        .collect()
}

impl Engine {
    /// Run every enabled post-edit governance plugin after a successful file
    /// mutation. Plugins fail closed by returning no messages; the engine never
    /// blocks the edit path on governance advisories.
    pub(super) async fn run_post_edit_governance_hooks(
        &mut self,
        tool_name: &str,
        tool_input: &serde_json::Value,
    ) {
        if self.post_edit_governance_plugins.is_empty() {
            return;
        }

        let workspace = self.session.workspace.clone();
        for plugin in &self.post_edit_governance_plugins {
            let messages = plugin
                .on_successful_edit(&workspace, tool_name, tool_input)
                .await;
            tracing::debug!(
                plugin = plugin.name(),
                message_count = messages.len(),
                "post-edit governance plugin completed"
            );
            self.pending_governance_messages.extend(messages);
        }
    }

    /// Drain pending governance advisories into the session message stream so
    /// the model sees them before the next API request.
    pub(super) async fn flush_pending_governance_messages(&mut self) {
        let messages = std::mem::take(&mut self.pending_governance_messages);
        for msg in messages {
            self.add_session_message(self.user_text_message_with_turn_metadata(msg))
                .await;
        }
    }
}
