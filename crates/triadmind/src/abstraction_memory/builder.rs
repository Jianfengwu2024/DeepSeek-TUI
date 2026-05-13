use std::collections::{HashMap, HashSet};
use std::path::Path;

use super::evidence::{
    appears_as_contract, extract_abstraction_evidence, is_excluded_source, load_triad_node_records,
    normalize_path, sanitize_id, sorted_set,
};
use super::types::{
    AbstractionMemoryArtifact, AbstractionMemoryConfig, AbstractionMemoryEntry,
    AbstractionMemoryEntryKind, AbstractionMemorySummary,
};

pub fn build_abstraction_memory(
    map_file: &Path,
    project_name: &str,
    config: &AbstractionMemoryConfig,
    stable_source_paths: &HashSet<String>,
) -> Result<AbstractionMemoryArtifact, anyhow::Error> {
    let nodes = load_triad_node_records(map_file)?;

    let mut seen_sources: HashSet<String> = HashSet::new();
    let mut excluded_stable = 0usize;
    let mut excluded_configured = 0usize;

    let mut contracts: HashMap<String, ContractAccumulator> = HashMap::new();
    let mut abstract_classes: HashMap<String, ContractAccumulator> = HashMap::new();
    let mut abstract_functions: HashMap<String, FunctionAccumulator> = HashMap::new();

    for node in &nodes {
        let source_path = node.source_path.as_deref().unwrap_or_default();
        if source_path.is_empty() {
            continue;
        }

        let normalized_path = normalize_path(source_path);
        seen_sources.insert(normalized_path.clone());

        if config.exclude_mature_stable_sources && stable_source_paths.contains(&normalized_path) {
            excluded_stable += 1;
            continue;
        }

        if is_excluded_source(&normalized_path, config) {
            excluded_configured += 1;
            continue;
        }

        let evidence = extract_abstraction_evidence(node);
        let ratio = evidence.abstraction_ratio();
        if ratio < config.min_abstraction_ratio && !evidence.has_signal_data() {
            continue;
        }

        let node_id = node.node_id.clone();

        for contract_name in &evidence.implements {
            let accumulator = contracts
                .entry(contract_name.clone())
                .or_insert_with(|| ContractAccumulator::new(contract_name));
            accumulator.source_paths.insert(normalized_path.clone());
            accumulator.provider_node_ids.insert(node_id.clone());
            accumulator.abstraction_ratios.push(ratio);
            accumulator.add_signal_tags(&evidence.signals);
            accumulator.extend_related(&evidence.depends_on_abstractions, contract_name);
            accumulator.add_variant_cluster(&evidence.variant_cluster);
        }

        for contract_name in &evidence.depends_on_abstractions {
            let accumulator = contracts
                .entry(contract_name.clone())
                .or_insert_with(|| ContractAccumulator::new(contract_name));
            accumulator.source_paths.insert(normalized_path.clone());
            accumulator.consumer_node_ids.insert(node_id.clone());
            accumulator.abstraction_ratios.push(ratio);
            accumulator.add_signal_tags(&evidence.signals);
            accumulator.add_variant_cluster(&evidence.variant_cluster);
        }

        for class_name in &evidence.extends_abstract {
            let accumulator = abstract_classes
                .entry(class_name.clone())
                .or_insert_with(|| ContractAccumulator::new(class_name));
            accumulator.source_paths.insert(normalized_path.clone());
            accumulator.provider_node_ids.insert(node_id.clone());
            accumulator.abstraction_ratios.push(ratio);
            accumulator.kind_hints.insert("abstract_class".into());
            accumulator.add_signal_tags(&evidence.signals);
            accumulator.add_variant_cluster(&evidence.variant_cluster);
        }

        for signature in &evidence.abstract_functions {
            let accumulator = abstract_functions
                .entry(signature.clone())
                .or_insert_with(|| FunctionAccumulator::new(signature));
            accumulator.source_paths.insert(normalized_path.clone());
            accumulator.node_ids.insert(node_id.clone());
            accumulator.abstraction_ratios.push(ratio);
            accumulator.tags.extend(evidence.signals.iter().cloned());
            accumulator
                .related_abstractions
                .extend(evidence.depends_on_abstractions.iter().cloned());
            accumulator.add_variant_cluster(&evidence.variant_cluster);
        }
    }

    let mut entries = Vec::new();

    for (name, accumulator) in contracts {
        let should_keep = !accumulator.provider_node_ids.is_empty()
            && (accumulator.consumer_node_ids.len() >= 1 || appears_as_contract(&name, &nodes));
        if should_keep {
            entries.push(build_contract_entry(
                &name,
                AbstractionMemoryEntryKind::InterfaceOrContract,
                accumulator,
            ));
        }
    }

    for (name, accumulator) in abstract_classes {
        if !accumulator.provider_node_ids.is_empty() {
            entries.push(build_contract_entry(
                &name,
                AbstractionMemoryEntryKind::AbstractClass,
                accumulator,
            ));
        }
    }

    for (signature, accumulator) in abstract_functions {
        if accumulator.node_ids.len() >= 2 {
            entries.push(build_function_entry(&signature, accumulator));
        }
    }

    let mut seen_ids = HashSet::new();
    entries.retain(|entry| seen_ids.insert(entry.id.clone()));

    let contract_count = entries
        .iter()
        .filter(|entry| entry.kind == AbstractionMemoryEntryKind::InterfaceOrContract)
        .count();
    let function_count = entries
        .iter()
        .filter(|entry| entry.kind == AbstractionMemoryEntryKind::AbstractFunction)
        .count();
    let module_count = entries
        .iter()
        .filter(|entry| entry.kind == AbstractionMemoryEntryKind::AbstractionModule)
        .count();

    Ok(AbstractionMemoryArtifact {
        schema_version: "1.0".into(),
        generated_at: crate::sync::chrono_now(),
        project: project_name.into(),
        source_map_file: map_file.to_string_lossy().to_string(),
        summary: AbstractionMemorySummary {
            scanned_source_count: seen_sources.len(),
            excluded_stable_source_count: excluded_stable,
            excluded_configured_source_count: excluded_configured,
            remembered_entry_count: entries.len(),
            contract_entry_count: contract_count,
            abstract_function_entry_count: function_count,
            module_entry_count: module_count,
            hotspot_count: 0,
            variant_cluster_count: 0,
        },
        entries,
    })
}

struct ContractAccumulator {
    source_paths: HashSet<String>,
    provider_node_ids: HashSet<String>,
    consumer_node_ids: HashSet<String>,
    variant_clusters: HashSet<String>,
    related_abstractions: HashSet<String>,
    tags: HashSet<String>,
    abstraction_ratios: Vec<f64>,
    kind_hints: HashSet<String>,
}

impl ContractAccumulator {
    fn new(_name: &str) -> Self {
        Self {
            source_paths: HashSet::new(),
            provider_node_ids: HashSet::new(),
            consumer_node_ids: HashSet::new(),
            variant_clusters: HashSet::new(),
            related_abstractions: HashSet::new(),
            tags: HashSet::new(),
            abstraction_ratios: Vec::new(),
            kind_hints: HashSet::new(),
        }
    }

    fn add_signal_tags(&mut self, signals: &[String]) {
        self.tags.extend(signals.iter().cloned());
    }

    fn extend_related(&mut self, abstractions: &[String], current_name: &str) {
        self.related_abstractions.extend(
            abstractions
                .iter()
                .filter(|item| item.as_str() != current_name)
                .cloned(),
        );
    }

    fn add_variant_cluster(&mut self, variant_cluster: &str) {
        if !variant_cluster.is_empty() {
            self.variant_clusters.insert(variant_cluster.to_string());
        }
    }

    fn primary_source_path(&self) -> String {
        self.source_paths.iter().min().cloned().unwrap_or_default()
    }

    fn avg_abstraction_ratio(&self) -> f64 {
        if self.abstraction_ratios.is_empty() {
            0.0
        } else {
            self.abstraction_ratios.iter().sum::<f64>() / self.abstraction_ratios.len() as f64
        }
    }
}

struct FunctionAccumulator {
    signature: String,
    source_paths: HashSet<String>,
    node_ids: HashSet<String>,
    variant_clusters: HashSet<String>,
    related_abstractions: HashSet<String>,
    tags: HashSet<String>,
    abstraction_ratios: Vec<f64>,
}

impl FunctionAccumulator {
    fn new(signature: &str) -> Self {
        Self {
            signature: signature.into(),
            source_paths: HashSet::new(),
            node_ids: HashSet::new(),
            variant_clusters: HashSet::new(),
            related_abstractions: HashSet::new(),
            tags: HashSet::new(),
            abstraction_ratios: Vec::new(),
        }
    }

    fn add_variant_cluster(&mut self, variant_cluster: &str) {
        if !variant_cluster.is_empty() {
            self.variant_clusters.insert(variant_cluster.to_string());
        }
    }

    fn primary_source_path(&self) -> String {
        self.source_paths.iter().min().cloned().unwrap_or_default()
    }

    fn avg_abstraction_ratio(&self) -> f64 {
        if self.abstraction_ratios.is_empty() {
            0.0
        } else {
            self.abstraction_ratios.iter().sum::<f64>() / self.abstraction_ratios.len() as f64
        }
    }

    fn function_name(&self) -> String {
        self.signature
            .split(['(', ':'])
            .next()
            .unwrap_or(&self.signature)
            .trim()
            .to_string()
    }
}

fn build_contract_entry(
    name: &str,
    kind: AbstractionMemoryEntryKind,
    accumulator: ContractAccumulator,
) -> AbstractionMemoryEntry {
    let provider_count = accumulator.provider_node_ids.len();
    let consumer_count = accumulator.consumer_node_ids.len();
    let reusability_score = if provider_count >= 2 && consumer_count >= 1 {
        0.9
    } else if provider_count >= 2 {
        0.75
    } else if provider_count >= 1 && consumer_count >= 1 {
        0.7
    } else {
        0.45
    };

    let why_reusable = if provider_count >= 2 && consumer_count >= 1 {
        format!(
            "Contract '{}' has {} providers and {} consumers",
            name, provider_count, consumer_count
        )
    } else if provider_count >= 2 {
        format!("Contract '{}' has {} providers", name, provider_count)
    } else {
        format!("Contract '{}' is referenced by existing topology", name)
    };

    AbstractionMemoryEntry {
        id: sanitize_id(name),
        name: name.into(),
        kind,
        primary_source_path: accumulator.primary_source_path(),
        source_paths: sorted_set(&accumulator.source_paths),
        node_ids: Vec::new(),
        provider_node_ids: sorted_set(&accumulator.provider_node_ids),
        consumer_node_ids: sorted_set(&accumulator.consumer_node_ids),
        variant_clusters: sorted_set(&accumulator.variant_clusters),
        related_abstractions: sorted_set(&accumulator.related_abstractions),
        signatures: Vec::new(),
        tags: sorted_set(&accumulator.tags),
        abstraction_ratio: accumulator.avg_abstraction_ratio(),
        reusability_score,
        why_reusable,
    }
}

fn build_function_entry(
    signature: &str,
    accumulator: FunctionAccumulator,
) -> AbstractionMemoryEntry {
    let provider_count = accumulator.node_ids.len();
    let reusability_score = if provider_count >= 3 { 0.8 } else { 0.65 };

    AbstractionMemoryEntry {
        id: sanitize_id(signature),
        name: accumulator.function_name(),
        kind: AbstractionMemoryEntryKind::AbstractFunction,
        primary_source_path: accumulator.primary_source_path(),
        source_paths: sorted_set(&accumulator.source_paths),
        node_ids: sorted_set(&accumulator.node_ids),
        provider_node_ids: Vec::new(),
        consumer_node_ids: Vec::new(),
        variant_clusters: sorted_set(&accumulator.variant_clusters),
        related_abstractions: sorted_set(&accumulator.related_abstractions),
        signatures: vec![signature.to_string()],
        tags: sorted_set(&accumulator.tags),
        abstraction_ratio: accumulator.avg_abstraction_ratio(),
        reusability_score,
        why_reusable: format!(
            "Abstract function '{}' appears in {} nodes",
            accumulator.function_name(),
            provider_count
        ),
    }
}
