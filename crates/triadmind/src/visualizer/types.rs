//! Types for the visualizer — knowledge graph nodes, edges, and options.

use serde::{Deserialize, Serialize};

// ── Visualizer Options ──────────────────────────────────────────────

/// Options for visualizer generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizerOptions {
    /// Default view mode.
    #[serde(default = "default_view")]
    pub default_view: String,
    /// Whether to show isolated (unconnected) nodes.
    #[serde(default)]
    pub show_isolated_capabilities: bool,
    /// Maximum number of nodes to render.
    #[serde(default = "default_max_nodes")]
    pub max_render_nodes: usize,
    /// Maximum number of edges to render.
    #[serde(default = "default_max_edges")]
    pub max_render_edges: usize,
}

fn default_view() -> String {
    "architecture".into()
}
fn default_max_nodes() -> usize {
    500
}
fn default_max_edges() -> usize {
    1500
}

impl Default for VisualizerOptions {
    fn default() -> Self {
        Self {
            default_view: default_view(),
            show_isolated_capabilities: false,
            max_render_nodes: default_max_nodes(),
            max_render_edges: default_max_edges(),
        }
    }
}

// ── Knowledge Graph Types ───────────────────────────────────────────

/// A node in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeNode {
    pub id: String,
    pub label: String,
    pub group: String,
    /// Node lifecycle state.
    pub lifecycle: String,
    /// Fission problem statement.
    pub problem: String,
    /// Fission demand (dependencies).
    pub demand: Vec<String>,
    /// Fission answer (outputs).
    pub answer: Vec<String>,
    /// Source file path.
    pub source_path: String,
}

/// An edge in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEdge {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub label: String,
}

/// Complete graph data for rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizerGraph {
    pub nodes: Vec<KnowledgeNode>,
    pub edges: Vec<KnowledgeEdge>,
    pub stats: GraphStats,
}

/// Graph statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStats {
    pub nodes: usize,
    pub edges: usize,
    pub vertices: usize,
}
