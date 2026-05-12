//! Types for the navigator module — pre-implementation architecture impact mapping.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::protocol::UpgradeProtocol;

// ── Run Options ─────────────────────────────────────────────────────

/// Options for running the navigator.
#[derive(Debug, Clone)]
pub struct NavigatorRunOptions {
    /// Optional path to an existing upgrade protocol.
    pub protocol_path: Option<PathBuf>,
    /// LLM provider name (for prompt-based protocol generation).
    pub llm: Option<String>,
}

impl Default for NavigatorRunOptions {
    fn default() -> Self {
        Self {
            protocol_path: None,
            llm: None,
        }
    }
}

// ── Run Result ──────────────────────────────────────────────────────

/// Result of a navigator run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigatorRunResult {
    /// Status: "pending_protocol" or "ready".
    pub status: String,
    /// The feature demand that was analyzed.
    pub demand: String,
    /// Path to the impact map artifact.
    #[serde(rename = "impactMapFile")]
    pub impact_map_file: String,
    /// Path to the impact protocol.
    #[serde(rename = "impactProtocolFile")]
    pub impact_protocol_file: String,
    /// Path to the navigator prompt.
    #[serde(rename = "impactPromptFile")]
    pub impact_prompt_file: String,
    /// Path to the visualizer output.
    #[serde(rename = "impactVisualizerFile")]
    pub impact_visualizer_file: String,
    /// Summary lines for display.
    pub summary: Vec<String>,
}

// ── Impact Map Artifact ─────────────────────────────────────────────

/// The impact map artifact written to disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactMapArtifact {
    #[serde(rename = "schemaVersion")]
    pub schema_version: String,
    #[serde(rename = "generatedAt")]
    pub generated_at: String,
    /// Project name.
    pub project: String,
    /// Feature description.
    pub feature: String,
    /// Path to the impact protocol.
    #[serde(rename = "protocolFile")]
    pub protocol_file: String,
    /// Path to the visualizer output.
    #[serde(rename = "visualizerFile")]
    pub visualizer_file: String,
    /// The generated upgrade protocol.
    pub protocol: UpgradeProtocol,
    /// Preview topology data.
    #[serde(rename = "previewTopology")]
    pub preview_topology: serde_json::Value,
    /// Impact graph data.
    pub graph: serde_json::Value,
    /// Summary statistics.
    pub summary: ImpactGraphSummary,
}

/// Summary of the impact graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactGraphSummary {
    #[serde(rename = "proposedNodeCount")]
    pub proposed_node_count: usize,
    #[serde(rename = "proposedEdgeCount")]
    pub proposed_edge_count: usize,
    #[serde(rename = "totalVisibleNodes")]
    pub total_visible_nodes: usize,
    #[serde(rename = "totalVisibleEdges")]
    pub total_visible_edges: usize,
}
