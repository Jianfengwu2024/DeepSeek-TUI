//! Knowledge graph construction from triad topology nodes.

use std::collections::HashSet;

use crate::protocol::TriadNodeDefinition;

use super::types::{GraphStats, KnowledgeEdge, KnowledgeNode, VisualizerGraph, VisualizerOptions};

/// Build a knowledge graph from triad node definitions.
pub fn build_knowledge_graph(
    nodes: &[TriadNodeDefinition],
    options: &VisualizerOptions,
) -> VisualizerGraph {
    // First pass: collect all node IDs
    let all_node_ids: HashSet<String> = nodes.iter().map(|n| n.node_id.clone()).collect();

    let mut graph_nodes: Vec<KnowledgeNode> = Vec::new();
    let mut graph_edges: Vec<KnowledgeEdge> = Vec::new();

    for node in nodes.iter().take(options.max_render_nodes) {
        let fission = match &node.fission {
            Some(f) => f,
            None => continue,
        };

        let group = categorize_node(&node.node_id, &fission.problem);
        let label = format_label(&node.node_id, &fission.problem);

        if !options.show_isolated_capabilities
            && fission.demand.is_empty()
            && fission.answer.is_empty()
        {
            continue;
        }

        graph_nodes.push(KnowledgeNode {
            id: node.node_id.clone(),
            label,
            group,
            lifecycle: node
                .lifecycle
                .as_ref()
                .map(|l| format!("{:?}", l).to_lowercase())
                .unwrap_or_default(),
            problem: fission.problem.clone(),
            demand: fission.demand.clone(),
            answer: fission.answer.clone(),
            source_path: node.source_path.as_deref().unwrap_or("").to_string(),
        });

        // Build edges for demand dependencies
        for demand_item in &fission.demand {
            if is_generic_type(demand_item) {
                continue;
            }
            if all_node_ids.contains(demand_item.as_str()) {
                graph_edges.push(KnowledgeEdge {
                    from: demand_item.clone(),
                    to: node.node_id.clone(),
                    label: "provides".into(),
                });
            }
        }

        // Build edges for answer outputs
        for answer_item in &fission.answer {
            if is_generic_type(answer_item) {
                continue;
            }
            if all_node_ids.contains(answer_item.as_str()) {
                graph_edges.push(KnowledgeEdge {
                    from: node.node_id.clone(),
                    to: answer_item.clone(),
                    label: "produces".into(),
                });
            }
        }
    }

    let stats = GraphStats {
        nodes: graph_nodes.len(),
        edges: graph_edges.len(),
        vertices: graph_nodes.len() + graph_edges.len(),
    };

    VisualizerGraph {
        nodes: graph_nodes,
        edges: graph_edges,
        stats,
    }
}

/// Categorize a node based on its id and problem statement.
pub(crate) fn categorize_node(node_id: &str, problem: &str) -> String {
    let combined = format!("{} {}", node_id.to_lowercase(), problem.to_lowercase());

    if combined.contains("handle")
        || combined.contains("handler")
        || combined.contains("controller")
        || combined.contains("route")
    {
        "handler".into()
    } else if combined.contains("service")
        || combined.contains("usecase")
        || combined.contains("use_case")
    {
        "service".into()
    } else if combined.contains("adapter")
        || combined.contains("db")
        || combined.contains("repo")
        || combined.contains("storage")
        || combined.contains("gateway")
    {
        "adapter".into()
    } else if combined.contains("core")
        || combined.contains("domain")
        || combined.contains("engine")
    {
        "core".into()
    } else {
        "other".into()
    }
}

/// Format a human-readable label for a node.
pub(crate) fn format_label(node_id: &str, problem: &str) -> String {
    let parts: Vec<&str> = node_id.split('.').collect();
    let short_name = parts.last().copied().unwrap_or(node_id);
    if problem.len() > 40 {
        format!("{}\n{}…", short_name, &problem[..40])
    } else if problem.is_empty() {
        short_name.to_string()
    } else {
        format!("{}\n{}", short_name, problem)
    }
}

/// Check if a type name is a generic/low-value contract.
pub(crate) fn is_generic_type(type_name: &str) -> bool {
    let generics = [
        "str", "string", "int", "i32", "i64", "u32", "u64", "f32", "f64", "usize", "isize",
        "number", "bool", "boolean", "float", "void", "()", "any", "unknown", "object", "array",
        "list", "option", "result", "vec", "hashmap", "json", "request", "response", "promise",
        "future",
    ];
    let lower = type_name.to_lowercase();
    generics.contains(&lower.as_str())
}
