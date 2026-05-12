//! Core navigator engine — impact map generation and prompt building.

use std::path::Path;

use super::types::{ImpactMapArtifact, ImpactGraphSummary, NavigatorRunOptions, NavigatorRunResult};
use crate::protocol::{TriadNodeDefinition, UpgradeProtocol};
use crate::sync::chrono_now;

/// Run the navigator to generate an architecture impact map.
///
/// Analyzes how a feature demand intersects with the existing triad topology.
/// When a protocol already exists, generates the impact map directly.
/// When no protocol exists, writes a prompt for LLM-guided protocol generation.
pub fn run_navigator(
    project_root: &Path,
    demand: &str,
    options: &NavigatorRunOptions,
) -> Result<NavigatorRunResult, anyhow::Error> {
    let normalized_demand = demand.trim().to_string();
    if normalized_demand.is_empty() {
        anyhow::bail!("Navigator demand must not be empty");
    }

    let triad_dir = project_root.join(".triadmind");
    std::fs::create_dir_all(&triad_dir)?;

    let impact_map_file = triad_dir.join("impact-map.json");
    let impact_protocol_file = triad_dir.join("impact-protocol.json");
    let impact_prompt_file = triad_dir.join("impact-prompt.md");
    let impact_visualizer_file = triad_dir.join("impact-visualizer.html");

    let map_file = triad_dir.join("triad-map.json");
    let nodes: Vec<TriadNodeDefinition> = if map_file.exists() {
        let content = std::fs::read_to_string(&map_file)?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Vec::new()
    };

    // If a protocol path is given and exists, use it directly
    let protocol = if let Some(ref proto_path) = options.protocol_path {
        if proto_path.exists() {
            let content = std::fs::read_to_string(proto_path)?;
            serde_json::from_str(&content).unwrap_or_else(|_| {
                build_protocol_template(&normalized_demand, &nodes)
            })
        } else {
            build_protocol_template(&normalized_demand, &nodes)
        }
    } else {
        build_protocol_template(&normalized_demand, &nodes)
    };

    // Determine status
    let has_real_protocol = options
        .protocol_path
        .as_ref()
        .map(|p| p.exists())
        .unwrap_or(false);

    let status = if has_real_protocol {
        "ready"
    } else {
        "pending_protocol"
    };

    // Generate the impact map artifact
    let artifact = ImpactMapArtifact {
        schema_version: "1.0".into(),
        generated_at: chrono_now(),
        project: "triadmind".into(),
        feature: normalized_demand.clone(),
        protocol_file: impact_protocol_file.to_string_lossy().to_string(),
        visualizer_file: impact_visualizer_file.to_string_lossy().to_string(),
        protocol: protocol.clone(),
        preview_topology: serde_json::json!({"nodes": nodes.len()}),
        graph: serde_json::json!({"nodes": [], "edges": []}),
        summary: ImpactGraphSummary {
            proposed_node_count: protocol.actions.len(),
            proposed_edge_count: 0,
            total_visible_nodes: nodes.len(),
            total_visible_edges: 0,
        },
    };

    // Write artifacts
    std::fs::write(
        &impact_map_file,
        serde_json::to_string_pretty(&artifact)?,
    )?;
    std::fs::write(
        &impact_protocol_file,
        serde_json::to_string_pretty(&protocol)?,
    )?;

    // Build and write the navigator prompt
    if !has_real_protocol {
        let prompt =
            build_navigator_prompt(project_root, &normalized_demand, &nodes, options.llm.as_deref());
        std::fs::write(&impact_prompt_file, &prompt)?;
    }

    Ok(NavigatorRunResult {
        status: status.into(),
        demand: normalized_demand,
        impact_map_file: impact_map_file.to_string_lossy().to_string(),
        impact_protocol_file: impact_protocol_file.to_string_lossy().to_string(),
        impact_prompt_file: impact_prompt_file.to_string_lossy().to_string(),
        impact_visualizer_file: impact_visualizer_file.to_string_lossy().to_string(),
        summary: if has_real_protocol {
            vec![
                format!("Impact map generated for '{}'", artifact.feature),
                format!("{} existing topology nodes analyzed", nodes.len()),
            ]
        } else {
            vec![
                format!(
                    "Navigator prompt written for '{}'",
                    artifact.feature
                ),
                "Run the prompt through an LLM to generate an UpgradeProtocol.".into(),
                format!(
                    "Then place it at {} and re-run the navigator.",
                    impact_protocol_file.display()
                ),
            ]
        },
    })
}

/// Build a navigator prompt for LLM-guided protocol generation.
pub fn build_navigator_prompt(
    project_root: &Path,
    demand: &str,
    existing_nodes: &[TriadNodeDefinition],
    llm_provider: Option<&str>,
) -> String {
    let map_file = project_root
        .join(".triadmind")
        .join("triad-map.json");
    let has_map = map_file.exists();

    let topology_summary = if existing_nodes.is_empty() {
        "(no existing topology — greenfield project)".to_string()
    } else {
        format!(
            "Topology snapshot: {} capability nodes available.\n\
             Existing node ids: {}",
            existing_nodes.len(),
            existing_nodes
                .iter()
                .take(20)
                .map(|n| n.node_id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
    };

    let mut sections: Vec<String> = Vec::new();
    sections.push(format!(
        "[Navigator Task]\n\
         You are the TriadMind Navigator. Analyze the following feature demand\n\
         against the existing architecture topology and produce a strict \n\
         UpgradeProtocol JSON document.\n\n\
         [Feature Demand]\n{}\n\n\
         {}",
        demand, topology_summary
    ));

    if has_map {
        sections.push(format!(
            "[Data Source]\n\
             Triad map available at: {}\n\
             Use this file to understand the existing topology before proposing actions.",
            map_file.display()
        ));
    }

    if let Some(_provider) = llm_provider {
        sections.push(format!(
            "[LLM Provider]\nProvider: {}\n",
            llm_provider.unwrap_or("unknown")
        ));
    }

    sections.push(format!(
        "[Output Format]\n\
         Return a valid UpgradeProtocol JSON with:\n\
         - protocol_version: \"1.0\"\n\
         - user_demand: \"{}\"\n\
         - upgrade_policy.allowed_ops: [\"reuse\", \"modify\", \"create_child\"]\n\
         - upgrade_policy.principle: \"reuse_first\"\n\
         - actions: [] (list of upgrade actions)\n\n\
         Each action must have:\n\
         - op: \"reuse\" | \"modify\" | \"create_child\"\n\
         - node_id: string (referencing existing or new capability node)\n\
         - reason: string (architecture rationale)\n\n\
         [Navigator Rules]\n\
         - This is a dry-run architecture preview, not an apply step.\n\
         - Favor reuse of mature existing nodes before inventing new capability hubs.\n\
         - Use only reuse / modify / create_child actions.\n\
         - Keep changes minimal and topology-aware.\n\
         - Return strict UpgradeProtocol JSON only.",
        demand
    ));

    sections.join("\n\n")
}

/// Build a minimal protocol template for the user to fill in.
fn build_protocol_template(
    demand: &str,
    _existing_nodes: &[TriadNodeDefinition],
) -> UpgradeProtocol {
    UpgradeProtocol {
        protocol_version: "1.0".into(),
        project: String::new(),
        map_source: "triad-map.json".into(),
        user_demand: demand.into(),
        upgrade_policy: crate::protocol::UpgradePolicy {
            allowed_ops: vec!["reuse".into(), "modify".into(), "create_child".into()],
            principle: "reuse_first".into(),
        },
        macro_split: None,
        meso_split: None,
        micro_split: None,
        actions: vec![],
    }
}
