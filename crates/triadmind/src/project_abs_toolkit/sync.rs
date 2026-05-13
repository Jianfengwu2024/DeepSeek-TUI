use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use anyhow::anyhow;

use crate::abstraction_memory::{AbstractionMemoryEntry, load_abstraction_memory};

use super::types::{
    ProjectAbsToolkitArtifact, ProjectAbsToolkitEntry, ProjectAbsToolkitEntryKind,
    ProjectAbsToolkitReusePolicy, ProjectAbsToolkitStability, ProjectAbsToolkitStatus,
    ProjectAbsToolkitSummary,
};

pub fn sync_project_abs_toolkit_from_memory(
    memory_file: &Path,
    toolkit_file: &Path,
    markdown_file: &Path,
    toolkit_dir: &Path,
    project_name: &str,
) -> Result<ProjectAbsToolkitArtifact, anyhow::Error> {
    let memory = load_abstraction_memory(memory_file).ok_or_else(|| {
        anyhow!(
            "abstraction memory not found at {}",
            memory_file.to_string_lossy()
        )
    })?;

    let existing = load_project_abs_toolkit(toolkit_file);
    let artifact =
        build_project_abs_toolkit_artifact(&memory, existing.as_ref(), project_name, memory_file);

    if let Some(parent) = toolkit_file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if let Some(parent) = markdown_file.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(toolkit_file, serde_json::to_string_pretty(&artifact)?)?;
    export_project_abs_toolkit_markdown(&artifact, markdown_file)?;
    export_project_abs_toolkit_directory(&artifact, toolkit_dir)?;

    Ok(artifact)
}

pub fn ensure_project_abs_toolkit(
    memory_file: &Path,
    toolkit_file: &Path,
    markdown_file: &Path,
    toolkit_dir: &Path,
    project_name: &str,
    force: bool,
) -> Result<ProjectAbsToolkitArtifact, anyhow::Error> {
    if !force && toolkit_file.exists() {
        if let Some(artifact) = load_project_abs_toolkit(toolkit_file) {
            return Ok(artifact);
        }
    }

    sync_project_abs_toolkit_from_memory(
        memory_file,
        toolkit_file,
        markdown_file,
        toolkit_dir,
        project_name,
    )
}

pub fn load_project_abs_toolkit(file_path: &Path) -> Option<ProjectAbsToolkitArtifact> {
    let content = std::fs::read_to_string(file_path).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn render_project_abs_toolkit_markdown(artifact: &ProjectAbsToolkitArtifact) -> String {
    let mut lines =
        vec![
        format!("# {} Project Abstraction Toolkit", artifact.project),
        String::new(),
        format!("- Generated: {}", artifact.generated_at),
        format!("- Source memory: {}", artifact.source_memory_file),
        format!("- Promoted entries: {}", artifact.summary.promoted_entry_count),
        format!("- Categories: {}", artifact.summary.category_count),
        format!("- Subcategories: {}", artifact.summary.subcategory_count),
        format!(
            "- Reuse-first entries: {}",
            artifact.summary.reuse_first_entry_count
        ),
        format!("- Stable entries: {}", artifact.summary.stable_entry_count),
        String::new(),
        "## Reuse Discipline".into(),
        String::new(),
        "1. Search the toolkit before creating a new abstract class or function.".into(),
        "2. Prefer `reuse_first` entries when a capability is close enough to the current demand."
            .into(),
        "3. If no toolkit entry fits, explain why before introducing a new abstraction.".into(),
        String::new(),
        "## Toolkit Entries".into(),
    ];

    if artifact.entries.is_empty() {
        lines.push(String::new());
        lines.push("No promoted abstractions yet.".into());
        return lines.join("\n");
    }

    for (index, entry) in artifact.entries.iter().enumerate() {
        lines.push(String::new());
        lines.push(format!(
            "### {}. {} [{}]",
            index + 1,
            entry.name,
            entry.kind.as_str()
        ));
        lines.push(String::new());
        lines.push(format!("- Status: {}", entry.status.as_str()));
        lines.push(format!("- Reuse policy: {}", entry.reuse_policy.as_str()));
        lines.push(format!("- Stability: {}", entry.stability.as_str()));
        lines.push(format!(
            "- Toolkit path: {}/{}",
            entry.category, entry.subcategory
        ));
        lines.push(format!("- Source: {}", entry.primary_source_path));
        lines.push(format!(
            "- Score: reuse={:.2}, abstraction={:.2}",
            entry.reusability_score, entry.abstraction_ratio
        ));
        lines.push(format!("- Intent: {}", entry.intent));
        lines.push(format!("- Why reusable: {}", entry.why_reusable));
        if !entry.applicability.is_empty() {
            lines.push(format!(
                "- Applicable when: {}",
                entry.applicability.join("; ")
            ));
        }
        if !entry.non_applicability.is_empty() {
            lines.push(format!(
                "- Avoid when: {}",
                entry.non_applicability.join("; ")
            ));
        }
        if !entry.adaptation_rules.is_empty() {
            lines.push(format!(
                "- Adaptation rules: {}",
                entry.adaptation_rules.join("; ")
            ));
        }
        if !entry.signatures.is_empty() {
            lines.push(format!("- Signature: {}", entry.signatures[0]));
        }
        if !entry.examples.is_empty() {
            lines.push(format!("- Example: {}", entry.examples.join("; ")));
        }
    }

    lines.join("\n")
}

pub fn export_project_abs_toolkit_markdown(
    artifact: &ProjectAbsToolkitArtifact,
    markdown_file: &Path,
) -> Result<(), anyhow::Error> {
    if let Some(parent) = markdown_file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(markdown_file, render_project_abs_toolkit_markdown(artifact))?;
    Ok(())
}

pub fn export_project_abs_toolkit_directory(
    artifact: &ProjectAbsToolkitArtifact,
    toolkit_dir: &Path,
) -> Result<(), anyhow::Error> {
    if toolkit_dir.exists() {
        std::fs::remove_dir_all(toolkit_dir)?;
    }
    std::fs::create_dir_all(toolkit_dir)?;

    std::fs::write(
        toolkit_dir.join("README.md"),
        render_project_abs_toolkit_markdown(artifact),
    )?;

    let grouped = group_entries_by_category(artifact);
    for (category, subgroups) in grouped {
        let category_dir = toolkit_dir.join(&category);
        std::fs::create_dir_all(&category_dir)?;
        std::fs::write(
            category_dir.join("README.md"),
            render_category_markdown(artifact, &category, &subgroups),
        )?;

        for (subcategory, entries) in subgroups {
            let subcategory_dir = category_dir.join(&subcategory);
            std::fs::create_dir_all(&subcategory_dir)?;
            std::fs::write(
                subcategory_dir.join("README.md"),
                render_subcategory_markdown(artifact, &category, &subcategory, &entries),
            )?;

            for entry in entries {
                std::fs::write(
                    toolkit_dir.join(&entry.toolkit_doc_path),
                    render_entry_markdown(entry),
                )?;
            }
        }
    }

    Ok(())
}

fn build_project_abs_toolkit_artifact(
    memory: &crate::abstraction_memory::AbstractionMemoryArtifact,
    existing: Option<&ProjectAbsToolkitArtifact>,
    project_name: &str,
    memory_file: &Path,
) -> ProjectAbsToolkitArtifact {
    let existing_map: HashMap<&str, &ProjectAbsToolkitEntry> = existing
        .map(|artifact| {
            artifact
                .entries
                .iter()
                .map(|entry| (entry.id.as_str(), entry))
                .collect()
        })
        .unwrap_or_default();

    let mut entries: Vec<ProjectAbsToolkitEntry> = memory
        .entries
        .iter()
        .filter(|entry| should_promote(entry))
        .map(|entry| {
            build_toolkit_entry(
                entry,
                existing_map.get(entry.id.as_str()).copied(),
                project_name,
            )
        })
        .collect();

    entries.sort_by(|left, right| {
        right
            .reusability_score
            .partial_cmp(&left.reusability_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.name.cmp(&right.name))
    });

    let summary = build_summary(memory.entries.len(), &entries);

    ProjectAbsToolkitArtifact {
        schema_version: "1.0".into(),
        generated_at: crate::sync::chrono_now(),
        project: project_name.into(),
        source_memory_file: memory_file.to_string_lossy().to_string(),
        source_map_file: memory.source_map_file.clone(),
        summary,
        entries,
    }
}

fn build_summary(
    scanned_memory_entry_count: usize,
    entries: &[ProjectAbsToolkitEntry],
) -> ProjectAbsToolkitSummary {
    let category_count = entries
        .iter()
        .map(|entry| entry.category.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let subcategory_count = entries
        .iter()
        .map(|entry| (entry.category.as_str(), entry.subcategory.as_str()))
        .collect::<BTreeSet<_>>()
        .len();

    ProjectAbsToolkitSummary {
        scanned_memory_entry_count,
        promoted_entry_count: entries.len(),
        category_count,
        subcategory_count,
        reuse_first_entry_count: entries
            .iter()
            .filter(|entry| entry.reuse_policy == ProjectAbsToolkitReusePolicy::ReuseFirst)
            .count(),
        stable_entry_count: entries
            .iter()
            .filter(|entry| {
                matches!(
                    entry.stability,
                    ProjectAbsToolkitStability::Stable | ProjectAbsToolkitStability::Canonical
                )
            })
            .count(),
        canonical_entry_count: entries
            .iter()
            .filter(|entry| entry.stability == ProjectAbsToolkitStability::Canonical)
            .count(),
        experimental_entry_count: entries
            .iter()
            .filter(|entry| entry.stability == ProjectAbsToolkitStability::Experimental)
            .count(),
    }
}

fn should_promote(entry: &AbstractionMemoryEntry) -> bool {
    if entry.reusability_score < 0.70 {
        return false;
    }

    let topology_footprint =
        entry.provider_node_ids.len() + entry.consumer_node_ids.len() + entry.node_ids.len();

    topology_footprint >= 2
        || entry.source_paths.len() >= 2
        || matches!(
            entry.kind,
            crate::abstraction_memory::AbstractionMemoryEntryKind::InterfaceOrContract
                | crate::abstraction_memory::AbstractionMemoryEntryKind::AbstractClass
        )
}

fn build_toolkit_entry(
    entry: &AbstractionMemoryEntry,
    existing: Option<&ProjectAbsToolkitEntry>,
    project_name: &str,
) -> ProjectAbsToolkitEntry {
    let default_reuse_policy = derive_reuse_policy(entry);
    let default_stability = derive_stability(entry);
    let (default_category, default_subcategory) = derive_toolkit_taxonomy(entry);
    let category = existing
        .map(|item| item.category.clone())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(default_category);
    let subcategory = existing
        .map(|item| item.subcategory.clone())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(default_subcategory);
    let toolkit_relative_dir = format!("{}/{}", category, subcategory);
    let toolkit_doc_path = format!(
        "{}/{}.md",
        toolkit_relative_dir,
        slugify_component(&entry.id)
    );
    let promoted_at = existing
        .map(|item| item.promoted_at.clone())
        .unwrap_or_else(crate::sync::chrono_now);

    ProjectAbsToolkitEntry {
        id: entry.id.clone(),
        name: entry.name.clone(),
        kind: ProjectAbsToolkitEntryKind::from(entry.kind),
        status: existing
            .map(|item| item.status)
            .unwrap_or(ProjectAbsToolkitStatus::Promoted),
        category,
        subcategory,
        toolkit_relative_dir,
        toolkit_doc_path,
        source_entry_id: entry.id.clone(),
        primary_source_path: entry.primary_source_path.clone(),
        source_paths: entry.source_paths.clone(),
        provider_node_ids: entry.provider_node_ids.clone(),
        consumer_node_ids: entry.consumer_node_ids.clone(),
        related_abstractions: entry.related_abstractions.clone(),
        signatures: entry.signatures.clone(),
        tags: entry.tags.clone(),
        abstraction_ratio: entry.abstraction_ratio,
        reusability_score: entry.reusability_score,
        why_reusable: entry.why_reusable.clone(),
        intent: existing
            .map(|item| item.intent.clone())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| default_intent(entry)),
        reuse_policy: existing
            .map(|item| item.reuse_policy)
            .unwrap_or(default_reuse_policy),
        applicability: existing
            .map(|item| item.applicability.clone())
            .filter(|items| !items.is_empty())
            .unwrap_or_else(|| default_applicability(entry)),
        non_applicability: existing
            .map(|item| item.non_applicability.clone())
            .filter(|items| !items.is_empty())
            .unwrap_or_else(|| default_non_applicability(entry)),
        adaptation_rules: existing
            .map(|item| item.adaptation_rules.clone())
            .filter(|items| !items.is_empty())
            .unwrap_or_else(|| default_adaptation_rules(entry)),
        examples: existing
            .map(|item| item.examples.clone())
            .filter(|items| !items.is_empty())
            .unwrap_or_else(|| default_examples(entry)),
        owner: existing
            .map(|item| item.owner.clone())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| project_name.to_string()),
        stability: existing
            .map(|item| item.stability)
            .unwrap_or(default_stability),
        notes: existing
            .map(|item| item.notes.clone())
            .unwrap_or_else(|| default_notes(entry)),
        promoted_at,
        last_reviewed_at: existing.and_then(|item| item.last_reviewed_at.clone()),
    }
}

fn derive_reuse_policy(entry: &AbstractionMemoryEntry) -> ProjectAbsToolkitReusePolicy {
    if entry.reusability_score >= 0.82
        && (!entry.provider_node_ids.is_empty() || !entry.consumer_node_ids.is_empty())
    {
        ProjectAbsToolkitReusePolicy::ReuseFirst
    } else if entry.reusability_score >= 0.70 {
        ProjectAbsToolkitReusePolicy::ReuseWithAdaptation
    } else {
        ProjectAbsToolkitReusePolicy::ReferenceOnly
    }
}

fn derive_stability(entry: &AbstractionMemoryEntry) -> ProjectAbsToolkitStability {
    let providers = entry.provider_node_ids.len();
    let consumers = entry.consumer_node_ids.len();

    if entry.reusability_score >= 0.90 && providers >= 2 && consumers >= 1 {
        ProjectAbsToolkitStability::Canonical
    } else if entry.reusability_score >= 0.82 && (providers >= 1 || consumers >= 1) {
        ProjectAbsToolkitStability::Stable
    } else if entry.reusability_score >= 0.74 {
        ProjectAbsToolkitStability::Candidate
    } else {
        ProjectAbsToolkitStability::Experimental
    }
}

fn default_intent(entry: &AbstractionMemoryEntry) -> String {
    match entry.kind {
        crate::abstraction_memory::AbstractionMemoryEntryKind::InterfaceOrContract => format!(
            "Use '{}' as the first mount point when a new capability needs the same contract surface.",
            entry.name
        ),
        crate::abstraction_memory::AbstractionMemoryEntryKind::AbstractClass => format!(
            "Extend '{}' when new behavior should stay inside the existing abstract skeleton.",
            entry.name
        ),
        crate::abstraction_memory::AbstractionMemoryEntryKind::AbstractFunction => format!(
            "Reuse '{}' before introducing a parallel helper or utility path.",
            entry.name
        ),
        _ => format!(
            "Check '{}' before creating a new abstraction with overlapping responsibility.",
            entry.name
        ),
    }
}

fn default_applicability(entry: &AbstractionMemoryEntry) -> Vec<String> {
    let mut items = vec![format!(
        "When the demand overlaps with '{}' and the current node can reuse its existing contract.",
        entry.name
    )];
    if !entry.tags.is_empty() {
        items.push(format!(
            "When the feature matches tags: {}",
            entry.tags.join(", ")
        ));
    }
    if !entry.consumer_node_ids.is_empty() {
        items.push(format!(
            "When nearby consumers already depend on it: {}",
            entry.consumer_node_ids.join(", ")
        ));
    }
    items
}

fn default_non_applicability(entry: &AbstractionMemoryEntry) -> Vec<String> {
    let mut items = vec![
        "When the new demand requires a different contract boundary or incompatible lifecycle."
            .into(),
    ];
    if entry.reusability_score < 0.82 {
        items.push(
            "When only a small fragment is reusable and adaptation would create a misleading abstraction."
                .into(),
        );
    }
    items
}

fn default_adaptation_rules(entry: &AbstractionMemoryEntry) -> Vec<String> {
    let mut items = vec![
        "Preserve the existing contract shape before adding extension-specific behavior.".into(),
        "Prefer mounting new behavior through existing provider or consumer seams.".into(),
    ];
    if !entry.signatures.is_empty() {
        items.push(format!(
            "Keep the primary signature aligned with '{}'.",
            entry.signatures[0]
        ));
    }
    items
}

fn default_examples(entry: &AbstractionMemoryEntry) -> Vec<String> {
    let mut items = vec![format!(
        "See {} for the current project implementation anchor.",
        entry.primary_source_path
    )];
    if let Some(signature) = entry.signatures.first() {
        items.push(format!("Current shared signature: {}", signature));
    }
    items
}

fn default_notes(entry: &AbstractionMemoryEntry) -> String {
    format!(
        "Promoted from abstraction memory because {}",
        entry.why_reusable.to_lowercase()
    )
}

fn derive_toolkit_taxonomy(entry: &AbstractionMemoryEntry) -> (String, String) {
    let normalized = entry.primary_source_path.replace('\\', "/");
    let mut parts: Vec<&str> = normalized
        .split('/')
        .filter(|item| !item.trim().is_empty())
        .collect();
    if !parts.is_empty() {
        parts.pop();
    }

    let mut category = String::new();
    let mut subcategory = String::new();

    if let Some(index) = parts.iter().position(|segment| *segment == "crates") {
        if let Some(crate_name) = parts.get(index + 1) {
            category = slugify_component(crate_name);
        }
    }

    if category.is_empty() {
        for segment in &parts {
            let candidate = segment.to_ascii_lowercase();
            if matches!(
                candidate.as_str(),
                "src" | "app" | "apps" | "lib" | "libs" | "packages" | "package"
            ) {
                continue;
            }
            category = slugify_component(segment);
            break;
        }
    }

    if category.is_empty() {
        category = "general".into();
    }

    let mut hit_category = false;
    for segment in &parts {
        let normalized_segment = slugify_component(segment);
        if normalized_segment.is_empty() {
            continue;
        }
        if !hit_category {
            if normalized_segment == category {
                hit_category = true;
            }
            continue;
        }
        if matches!(
            normalized_segment.as_str(),
            "src" | "app" | "apps" | "lib" | "libs"
        ) {
            continue;
        }
        subcategory = normalized_segment;
        break;
    }

    if subcategory.is_empty() {
        subcategory = kind_bucket(entry.kind).to_string();
    }

    (category, subcategory)
}

fn kind_bucket(kind: crate::abstraction_memory::AbstractionMemoryEntryKind) -> &'static str {
    match kind {
        crate::abstraction_memory::AbstractionMemoryEntryKind::InterfaceOrContract => "contracts",
        crate::abstraction_memory::AbstractionMemoryEntryKind::AbstractClass => "abstract_classes",
        crate::abstraction_memory::AbstractionMemoryEntryKind::AbstractFunction => {
            "abstract_functions"
        }
        crate::abstraction_memory::AbstractionMemoryEntryKind::ContractDependency => {
            "contract_dependencies"
        }
        crate::abstraction_memory::AbstractionMemoryEntryKind::AbstractionModule => "modules",
    }
}

fn slugify_component(input: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash {
            out.push('_');
            prev_dash = true;
        }
    }
    out.trim_matches('_').to_string()
}

fn group_entries_by_category<'a>(
    artifact: &'a ProjectAbsToolkitArtifact,
) -> BTreeMap<String, BTreeMap<String, Vec<&'a ProjectAbsToolkitEntry>>> {
    let mut grouped: BTreeMap<String, BTreeMap<String, Vec<&ProjectAbsToolkitEntry>>> =
        BTreeMap::new();
    for entry in &artifact.entries {
        grouped
            .entry(entry.category.clone())
            .or_default()
            .entry(entry.subcategory.clone())
            .or_default()
            .push(entry);
    }

    for subgroups in grouped.values_mut() {
        for entries in subgroups.values_mut() {
            entries.sort_by(|left, right| left.name.cmp(&right.name));
        }
    }

    grouped
}

fn render_category_markdown(
    artifact: &ProjectAbsToolkitArtifact,
    category: &str,
    subgroups: &BTreeMap<String, Vec<&ProjectAbsToolkitEntry>>,
) -> String {
    let count = subgroups.values().map(Vec::len).sum::<usize>();
    let mut lines = vec![
        format!("# Category: {}", category),
        String::new(),
        format!("- Project: {}", artifact.project),
        format!("- Total entries: {}", count),
        format!("- Subcategories: {}", subgroups.len()),
        String::new(),
        "## Subcategories".into(),
    ];

    for (subcategory, entries) in subgroups {
        lines.push(String::new());
        lines.push(format!("### {} ({})", subcategory, entries.len()));
        for entry in entries {
            lines.push(format!(
                "- {} [{}] -> `{}`",
                entry.name,
                entry.kind.as_str(),
                entry.toolkit_doc_path
            ));
        }
    }

    lines.join("\n")
}

fn render_subcategory_markdown(
    artifact: &ProjectAbsToolkitArtifact,
    category: &str,
    subcategory: &str,
    entries: &[&ProjectAbsToolkitEntry],
) -> String {
    let mut lines = vec![
        format!("# {}/{}", category, subcategory),
        String::new(),
        format!("- Project: {}", artifact.project),
        format!("- Entries: {}", entries.len()),
        String::new(),
        "## Entries".into(),
    ];

    for entry in entries {
        lines.push(format!(
            "- {} [{}] policy={} stability={} -> `{}`",
            entry.name,
            entry.kind.as_str(),
            entry.reuse_policy.as_str(),
            entry.stability.as_str(),
            entry.toolkit_doc_path
        ));
    }

    lines.join("\n")
}

fn render_entry_markdown(entry: &ProjectAbsToolkitEntry) -> String {
    let mut lines = vec![
        format!("# {} [{}]", entry.name, entry.kind.as_str()),
        String::new(),
        format!("- Category: {}", entry.category),
        format!("- Subcategory: {}", entry.subcategory),
        format!("- Reuse policy: {}", entry.reuse_policy.as_str()),
        format!("- Stability: {}", entry.stability.as_str()),
        format!("- Source: {}", entry.primary_source_path),
        format!("- Reusability score: {:.2}", entry.reusability_score),
        String::new(),
        "## Intent".into(),
        entry.intent.clone(),
        String::new(),
        "## Reuse Rationale".into(),
        entry.why_reusable.clone(),
    ];

    if !entry.signatures.is_empty() {
        lines.push(String::new());
        lines.push("## Signatures".into());
        for signature in &entry.signatures {
            lines.push(format!("- {}", signature));
        }
    }

    if !entry.adaptation_rules.is_empty() {
        lines.push(String::new());
        lines.push("## Adaptation Rules".into());
        for rule in &entry.adaptation_rules {
            lines.push(format!("- {}", rule));
        }
    }

    if !entry.examples.is_empty() {
        lines.push(String::new());
        lines.push("## Examples".into());
        for example in &entry.examples {
            lines.push(format!("- {}", example));
        }
    }

    lines.push(
        "\nReview this entry before introducing a new abstraction in this subcategory.".into(),
    );
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use crate::abstraction_memory::{
        AbstractionMemoryArtifact, AbstractionMemoryEntry, AbstractionMemoryEntryKind,
        AbstractionMemorySummary,
    };

    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("triadmind-toolkit-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn sample_memory() -> AbstractionMemoryArtifact {
        AbstractionMemoryArtifact {
            schema_version: "1.0".into(),
            generated_at: "2026-01-01T00:00:00Z".into(),
            project: "sample".into(),
            source_map_file: ".triadmind/triad-map.json".into(),
            summary: AbstractionMemorySummary {
                remembered_entry_count: 2,
                contract_entry_count: 1,
                abstract_function_entry_count: 1,
                ..AbstractionMemorySummary::default()
            },
            entries: vec![
                AbstractionMemoryEntry {
                    id: "payment_strategy".into(),
                    name: "PaymentStrategy".into(),
                    kind: AbstractionMemoryEntryKind::InterfaceOrContract,
                    primary_source_path: "src/payments/strategy.rs".into(),
                    source_paths: vec![
                        "src/payments/strategy.rs".into(),
                        "src/payments/router.rs".into(),
                    ],
                    node_ids: vec![],
                    provider_node_ids: vec!["StripePay.execute".into(), "MockPay.execute".into()],
                    consumer_node_ids: vec!["PaymentRouter.route".into()],
                    variant_clusters: vec!["payments".into()],
                    related_abstractions: vec!["RefundStrategy".into()],
                    signatures: vec!["PaymentStrategy.execute(PayInput) -> PayResult".into()],
                    tags: vec!["payments".into(), "strategy".into()],
                    abstraction_ratio: 0.62,
                    reusability_score: 0.91,
                    why_reusable: "Contract 'PaymentStrategy' has 2 providers and 1 consumers"
                        .into(),
                },
                AbstractionMemoryEntry {
                    id: "format_result".into(),
                    name: "format_result".into(),
                    kind: AbstractionMemoryEntryKind::AbstractFunction,
                    primary_source_path: "src/shared/result.ts".into(),
                    source_paths: vec!["src/shared/result.ts".into()],
                    node_ids: vec![
                        "ResultPresenter.format_result".into(),
                        "UiBanner.format_result".into(),
                    ],
                    provider_node_ids: vec![],
                    consumer_node_ids: vec![],
                    variant_clusters: vec![],
                    related_abstractions: vec![],
                    signatures: vec!["format_result(ResultLike) -> String".into()],
                    tags: vec!["format".into()],
                    abstraction_ratio: 0.48,
                    reusability_score: 0.73,
                    why_reusable: "Appears across multiple nodes".into(),
                },
            ],
        }
    }

    #[test]
    fn sync_project_abs_toolkit_promotes_reusable_entries() {
        let dir = temp_dir("sync");
        let triad_dir = dir.join(".triadmind");
        fs::create_dir_all(&triad_dir).expect("triad dir");
        let memory_file = triad_dir.join("abstraction-memory.json");
        let toolkit_file = triad_dir.join("project-abs-toolkit.json");
        let markdown_file = triad_dir.join("project-abs-toolkit.md");
        let toolkit_dir = triad_dir.join("project-abs-toolkit");
        fs::write(
            &memory_file,
            serde_json::to_string_pretty(&sample_memory()).expect("serialize"),
        )
        .expect("write memory");

        let artifact = sync_project_abs_toolkit_from_memory(
            &memory_file,
            &toolkit_file,
            &markdown_file,
            &toolkit_dir,
            "sample",
        )
        .expect("sync toolkit");

        assert_eq!(artifact.summary.promoted_entry_count, 2);
        assert!(toolkit_file.exists());
        assert!(markdown_file.exists());
        assert!(toolkit_dir.join("README.md").exists());
        assert_eq!(artifact.entries[0].name, "PaymentStrategy");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn search_project_abs_toolkit_matches_intent_and_tags() {
        let memory = sample_memory();
        let artifact = build_project_abs_toolkit_artifact(
            &memory,
            None,
            "sample",
            Path::new(".triadmind/abstraction-memory.json"),
        );
        let results = crate::project_abs_toolkit::search_project_abs_toolkit(
            &artifact,
            "payment strategy",
            5,
        );
        assert!(!results.is_empty());
        assert_eq!(results[0].entry.name, "PaymentStrategy");
    }

    #[test]
    fn sync_preserves_manual_toolkit_notes() {
        let memory = sample_memory();
        let existing = ProjectAbsToolkitArtifact {
            schema_version: "1.0".into(),
            generated_at: "2026-01-02T00:00:00Z".into(),
            project: "sample".into(),
            source_memory_file: ".triadmind/abstraction-memory.json".into(),
            source_map_file: ".triadmind/triad-map.json".into(),
            summary: ProjectAbsToolkitSummary::default(),
            entries: vec![ProjectAbsToolkitEntry {
                id: "payment_strategy".into(),
                name: "PaymentStrategy".into(),
                kind: ProjectAbsToolkitEntryKind::InterfaceOrContract,
                status: ProjectAbsToolkitStatus::Shared,
                category: "payments".into(),
                subcategory: "contracts".into(),
                toolkit_relative_dir: "payments/contracts".into(),
                toolkit_doc_path: "payments/contracts/payment_strategy.md".into(),
                source_entry_id: "payment_strategy".into(),
                primary_source_path: "src/payments/strategy.rs".into(),
                source_paths: vec!["src/payments/strategy.rs".into()],
                provider_node_ids: vec!["StripePay.execute".into()],
                consumer_node_ids: vec!["PaymentRouter.route".into()],
                related_abstractions: vec![],
                signatures: vec!["PaymentStrategy.execute(PayInput) -> PayResult".into()],
                tags: vec!["payments".into()],
                abstraction_ratio: 0.60,
                reusability_score: 0.88,
                why_reusable: "existing".into(),
                intent: "manual intent".into(),
                reuse_policy: ProjectAbsToolkitReusePolicy::ReuseFirst,
                applicability: vec!["manual applicability".into()],
                non_applicability: vec!["manual exclusion".into()],
                adaptation_rules: vec!["manual rule".into()],
                examples: vec!["manual example".into()],
                owner: "team".into(),
                stability: ProjectAbsToolkitStability::Canonical,
                notes: "manual notes".into(),
                promoted_at: "2026-01-02T00:00:00Z".into(),
                last_reviewed_at: Some("2026-01-03T00:00:00Z".into()),
            }],
        };

        let artifact = build_project_abs_toolkit_artifact(
            &memory,
            Some(&existing),
            "sample",
            Path::new(".triadmind/abstraction-memory.json"),
        );

        let entry = artifact
            .entries
            .iter()
            .find(|item| item.id == "payment_strategy")
            .expect("payment_strategy entry");
        assert_eq!(entry.status, ProjectAbsToolkitStatus::Shared);
        assert_eq!(entry.intent, "manual intent");
        assert_eq!(entry.notes, "manual notes");
        assert_eq!(entry.owner, "team");
    }
}
