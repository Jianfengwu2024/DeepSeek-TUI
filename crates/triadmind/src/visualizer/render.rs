//! HTML rendering and main entry point for the visualizer.

use std::path::Path;

use crate::protocol::TriadNodeDefinition;

use super::graph::build_knowledge_graph;
use super::types::{VisualizerGraph, VisualizerOptions};

/// Generate an interactive HTML triad topology visualizer.
pub fn generate_triad_visualizer(
    map_file: &Path,
    output_path: &Path,
    options: &VisualizerOptions,
) -> Result<(), anyhow::Error> {
    let nodes: Vec<TriadNodeDefinition> = if map_file.exists() {
        let content = std::fs::read_to_string(map_file)?;
        let trimmed = content.trim().trim_start_matches('\u{FEFF}');
        serde_json::from_str(trimmed).unwrap_or_default()
    } else {
        Vec::new()
    };

    let graph = build_knowledge_graph(&nodes, options);
    let html = render_html(&graph, options);

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(output_path, html)?;

    Ok(())
}

/// Render the knowledge graph as a self-contained HTML page.
pub fn render_html(graph: &VisualizerGraph, _options: &VisualizerOptions) -> String {
    let graph_json =
        serde_json::to_string(graph).unwrap_or_else(|_| "{}".to_string());

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>TriadMind — Architecture Topology</title>
<script src="https://unpkg.com/vis-network@9.1.6/dist/vis-network.min.js"></script>
<style>
  body {{ margin: 0; background: #0d1117; color: #c9d1d9; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; }}
  #header {{ padding: 12px 20px; background: #161b22; border-bottom: 1px solid #30363d; display: flex; justify-content: space-between; align-items: center; }}
  #header h1 {{ margin: 0; font-size: 18px; color: #58a6ff; }}
  #stats {{ font-size: 13px; color: #8b949e; }}
  #mynetwork {{ width: 100vw; height: calc(100vh - 48px); }}
  .vis-network:focus {{ outline: none; }}
</style>
</head>
<body>
<div id="header">
  <h1>TriadMind Architecture Topology</h1>
  <div id="stats">Nodes: {nodes} | Edges: {edges} | Vertices: {vertices}</div>
</div>
<div id="mynetwork"></div>
<script>
  var graphData = {graph_json};
  var nodes = new vis.DataSet(graphData.nodes.map(function(n) {{
    return {{
      id: n.id,
      label: n.label,
      group: n.group,
      title: '<b>' + n.id + '</b><br>' + (n.problem || '(no problem)') + '<br><i>' + n.source_path + '</i>'
    }};
  }}));
  var edges = new vis.DataSet(graphData.edges.map(function(e) {{
    return {{ from: e.from, to: e.to, label: e.label, arrows: 'to' }};
  }}));
  var container = document.getElementById('mynetwork');
  var data = {{ nodes: nodes, edges: edges }};
  var options = {{
    physics: {{ solver: 'forceAtlas2Based', forceAtlas2Based: {{ gravitationalConstant: -50, centralGravity: 0.01 }} }},
    edges: {{ smooth: {{ type: 'continuous' }}, font: {{ size: 10 }} }},
    groups: {{
      handler: {{ color: {{ background: '#ff7b72', border: '#ff7b72' }} }},
      service: {{ color: {{ background: '#58a6ff', border: '#58a6ff' }} }},
      adapter: {{ color: {{ background: '#3fb950', border: '#3fb950' }} }},
      core: {{ color: {{ background: '#d2a8ff', border: '#d2a8ff' }} }},
      other: {{ color: {{ background: '#8b949e', border: '#8b949e' }} }}
    }}
  }};
  new vis.Network(container, data, options);
</script>
</body>
</html>"#,
        nodes = graph.stats.nodes,
        edges = graph.stats.edges,
        vertices = graph.stats.vertices,
        graph_json = graph_json,
    )
}
