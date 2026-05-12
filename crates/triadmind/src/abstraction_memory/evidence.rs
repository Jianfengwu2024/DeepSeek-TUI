use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::protocol::TriadNodeDefinition;

use super::types::AbstractionMemoryConfig;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AbstractionEvidence {
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub signals: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub implements: Vec<String>,
    #[serde(rename = "extendsAbstract", default, skip_serializing_if = "Vec::is_empty")]
    pub extends_abstract: Vec<String>,
    #[serde(rename = "dependsOnAbstractions", default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on_abstractions: Vec<String>,
    #[serde(rename = "abstractFunctions", default, skip_serializing_if = "Vec::is_empty")]
    pub abstract_functions: Vec<String>,
    #[serde(rename = "functionContractCount", default)]
    pub function_contract_count: usize,
    #[serde(rename = "variantCluster", default)]
    pub variant_cluster: String,
    #[serde(rename = "abstractionSignalCount", default)]
    pub abstraction_signal_count: usize,
    #[serde(rename = "concreteSignalCount", default)]
    pub concrete_signal_count: usize,
    #[serde(rename = "interfaceCount", default)]
    pub interface_count: usize,
    #[serde(rename = "abstractClassCount", default)]
    pub abstract_class_count: usize,
    #[serde(rename = "concreteClassCount", default)]
    pub concrete_class_count: usize,
    #[serde(rename = "typeAliasCount", default)]
    pub type_alias_count: usize,
    #[serde(rename = "publicMethodCount", default)]
    pub public_method_count: usize,
    #[serde(rename = "topLevelExecutableCount", default)]
    pub top_level_executable_count: usize,
}

impl AbstractionEvidence {
    pub(crate) fn abstraction_ratio(&self) -> f64 {
        let total = self.abstraction_signal_count + self.concrete_signal_count;
        if total == 0 {
            0.0
        } else {
            self.abstraction_signal_count as f64 / total as f64
        }
    }

    pub(crate) fn has_signal_data(&self) -> bool {
        !self.signals.is_empty()
            || !self.implements.is_empty()
            || !self.extends_abstract.is_empty()
            || !self.depends_on_abstractions.is_empty()
            || !self.abstract_functions.is_empty()
            || self.abstraction_signal_count > 0
            || self.function_contract_count > 0
            || self.interface_count > 0
            || self.abstract_class_count > 0
            || self.type_alias_count > 0
            || !self.variant_cluster.is_empty()
    }

    fn finalize_role(&mut self) {
        if !self.role.is_empty() {
            return;
        }

        self.role = if self.abstraction_signal_count > self.concrete_signal_count {
            "abstraction_rich".into()
        } else if self.abstraction_signal_count > 0 {
            "mixed".into()
        } else {
            "concrete".into()
        };
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub(crate) struct TriadNodeRecord {
    #[serde(rename = "nodeId")]
    pub node_id: String,
    #[serde(rename = "sourcePath", default)]
    pub source_path: Option<String>,
    #[serde(default)]
    fission: Option<TriadFissionRecord>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub(crate) struct TriadFissionRecord {
    #[serde(default)]
    problem: String,
    #[serde(default)]
    demand: Vec<String>,
    #[serde(default)]
    answer: Vec<String>,
    #[serde(default)]
    evidence: Option<TriadEvidenceRecord>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct TriadEvidenceRecord {
    #[serde(default)]
    abstraction: Option<AbstractionEvidence>,
    #[serde(flatten)]
    direct_abstraction: AbstractionEvidence,
}

impl TriadEvidenceRecord {
    fn abstraction(&self) -> Option<AbstractionEvidence> {
        if let Some(abstraction) = &self.abstraction {
            return Some(abstraction.clone());
        }
        if self.direct_abstraction.has_signal_data() {
            return Some(self.direct_abstraction.clone());
        }
        None
    }
}

pub(crate) fn load_triad_nodes(map_file: &Path) -> Result<Vec<TriadNodeDefinition>, anyhow::Error> {
    if !map_file.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(map_file)?;
    let trimmed = content.trim().trim_start_matches('\u{FEFF}');
    Ok(serde_json::from_str(trimmed).unwrap_or_default())
}

pub(crate) fn load_triad_node_records(map_file: &Path) -> Result<Vec<TriadNodeRecord>, anyhow::Error> {
    if !map_file.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(map_file)?;
    let trimmed = content.trim().trim_start_matches('\u{FEFF}');
    Ok(serde_json::from_str(trimmed).unwrap_or_default())
}

pub(crate) fn extract_abstraction_evidence(node: &TriadNodeRecord) -> AbstractionEvidence {
    if let Some(fission) = &node.fission {
        if let Some(raw) = &fission.evidence {
            if let Some(mut abstraction) = raw.abstraction() {
                abstraction.finalize_role();
                return abstraction;
            }
        }
    }

    infer_abstraction_evidence(node)
}

fn infer_abstraction_evidence(node: &TriadNodeRecord) -> AbstractionEvidence {
    let mut evidence = AbstractionEvidence::default();
    let Some(fission) = &node.fission else {
        return evidence;
    };

    let problem = fission.problem.to_lowercase();
    let contract_names = extract_contract_names(&fission.demand);

    if !contract_names.is_empty() {
        evidence.depends_on_abstractions = contract_names.clone();
        evidence.signals.push("depends_on_contract".into());
        evidence.abstraction_signal_count += contract_names.len();
    }

    if problem.contains("implement") && contract_names.len() == 1 {
        evidence.implements = contract_names.clone();
        evidence.depends_on_abstractions.clear();
        evidence.signals.push("implements_contract".into());
        evidence.abstraction_signal_count += 1;
    }

    if problem.contains("extend") && contract_names.len() == 1 {
        evidence.extends_abstract = contract_names;
        evidence.signals.push("extends_abstract".into());
        evidence.abstract_class_count += 1;
        evidence.abstraction_signal_count += 1;
    }

    evidence.function_contract_count = evidence.abstract_functions.len();
    evidence.concrete_signal_count = fission.demand.len() + fission.answer.len() + 1;
    evidence.finalize_role();
    evidence
}

pub(crate) fn extract_contract_names(demand: &[String]) -> Vec<String> {
    let mut names = Vec::new();

    for entry in demand {
        for token in contract_tokens(entry) {
            if !names.iter().any(|existing| existing == &token) {
                names.push(token);
            }
        }
    }

    names
}

fn contract_tokens(value: &str) -> Vec<String> {
    let normalized = value
        .replace("dyn ", " ")
        .replace('&', " ")
        .replace('<', " ")
        .replace('>', " ")
        .replace(',', " ")
        .replace('(', " ")
        .replace(')', " ")
        .replace('[', " ")
        .replace(']', " ")
        .replace('{', " ")
        .replace('}', " ")
        .replace(':', " ")
        .replace('|', " ");

    normalized
        .split_whitespace()
        .filter_map(|raw| {
            let candidate = raw
                .rsplit("::")
                .next()
                .unwrap_or(raw)
                .trim_matches(|ch: char| !ch.is_alphanumeric() && ch != '_');

            if candidate.is_empty()
                || is_primitive_type(candidate)
                || is_generic_container(candidate)
            {
                return None;
            }

            candidate
                .chars()
                .next()
                .filter(|ch| ch.is_uppercase())
                .map(|_| candidate.to_string())
        })
        .collect()
}

fn is_primitive_type(value: &str) -> bool {
    matches!(
        value.to_lowercase().as_str(),
        "str"
            | "string"
            | "int"
            | "i32"
            | "i64"
            | "u32"
            | "u64"
            | "usize"
            | "isize"
            | "f32"
            | "f64"
            | "bool"
            | "boolean"
            | "void"
            | "none"
            | "()"
            | "self"
    )
}

fn is_generic_container(value: &str) -> bool {
    matches!(
        value,
        "Vec"
            | "HashMap"
            | "HashSet"
            | "Option"
            | "Result"
            | "String"
            | "Box"
            | "Rc"
            | "Arc"
            | "RefCell"
            | "Mutex"
            | "Promise"
    )
}

pub(crate) fn appears_as_contract(name: &str, nodes: &[TriadNodeRecord]) -> bool {
    let count = nodes
        .iter()
        .filter(|node| {
            let evidence = extract_abstraction_evidence(node);
            evidence.implements.iter().any(|item| item == name)
                || evidence.depends_on_abstractions.iter().any(|item| item == name)
                || node
                    .fission
                    .as_ref()
                    .map(|fission| {
                        extract_contract_names(&fission.demand)
                            .iter()
                            .any(|item| item == name)
                    })
                    .unwrap_or(false)
        })
        .count();

    count >= 2
}

pub(crate) fn is_excluded_source(normalized_path: &str, config: &AbstractionMemoryConfig) -> bool {
    config
        .exclude_source_path_patterns
        .iter()
        .any(|pattern| normalized_path.contains(pattern.as_str()))
        || config
            .exclude_source_paths
            .iter()
            .any(|path| normalized_path == path.as_str())
}

pub(crate) fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
        .trim_start_matches("./")
        .trim_end_matches('/')
        .to_string()
}

pub(crate) fn sorted_set(values: &HashSet<String>) -> Vec<String> {
    let mut items: Vec<String> = values.iter().cloned().collect();
    items.sort();
    items
}

pub(crate) fn sanitize_id(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_alphanumeric() { ch.to_ascii_lowercase() } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}
