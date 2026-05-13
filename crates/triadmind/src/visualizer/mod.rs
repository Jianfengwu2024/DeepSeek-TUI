//! # Visualizer — Interactive HTML topology knowledge graph
//!
//! Ported from triadmind-core/visualizer.ts
//!
//! Generates a self-contained HTML file that renders the triad topology
//! as an interactive graph using vis-network (CDN). Shows:
//! - Capability nodes with fission triples
//! - Edges representing dependencies
//! - Color-coded lifecycle states
//! - Filter/search controls
//!
//! @LeftBranch: generate_triad_visualizer, render_html
//! @RightBranch: VisualizerOptions, KnowledgeNode, KnowledgeEdge

pub mod graph;
pub mod render;
pub mod types;

#[cfg(test)]
mod tests;

pub use graph::build_knowledge_graph;
pub use render::{generate_triad_visualizer, render_html};
pub use types::{GraphStats, KnowledgeEdge, KnowledgeNode, VisualizerGraph, VisualizerOptions};
