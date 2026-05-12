use std::collections::HashSet;
use std::path::Path;

use super::builder::build_abstraction_memory;
use super::types::{AbstractionMemoryArtifact, AbstractionMemoryConfig, AbstractionMemorySummary};

pub fn sync_abstraction_memory(
    map_file: &Path,
    output_file: &Path,
    project_name: &str,
    config: &AbstractionMemoryConfig,
    stable_source_paths: &HashSet<String>,
) -> Result<AbstractionMemoryArtifact, anyhow::Error> {
    let artifact = build_abstraction_memory(map_file, project_name, config, stable_source_paths)?;

    if let Some(parent) = output_file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&artifact)?;
    std::fs::write(output_file, json)?;

    Ok(artifact)
}

pub fn load_abstraction_memory(file_path: &Path) -> Option<AbstractionMemoryArtifact> {
    let content = std::fs::read_to_string(file_path).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn ensure_abstraction_memory(
    map_file: &Path,
    memory_file: &Path,
    project_name: &str,
    config: &AbstractionMemoryConfig,
    stable_source_paths: &HashSet<String>,
    force: bool,
) -> Result<AbstractionMemoryArtifact, anyhow::Error> {
    if !config.enabled {
        return Ok(empty_artifact(project_name, map_file));
    }

    if force || !memory_file.exists() {
        return sync_abstraction_memory(map_file, memory_file, project_name, config, stable_source_paths);
    }

    match load_abstraction_memory(memory_file) {
        Some(artifact) => Ok(artifact),
        None => sync_abstraction_memory(map_file, memory_file, project_name, config, stable_source_paths),
    }
}

fn empty_artifact(project_name: &str, map_file: &Path) -> AbstractionMemoryArtifact {
    AbstractionMemoryArtifact {
        schema_version: "1.0".into(),
        generated_at: crate::sync::chrono_now(),
        project: project_name.into(),
        source_map_file: map_file.to_string_lossy().to_string(),
        summary: AbstractionMemorySummary::default(),
        entries: Vec::new(),
    }
}
