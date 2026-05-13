use serde::{Deserialize, Serialize};

use crate::abstraction_memory::AbstractionMemoryEntryKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectAbsToolkitEntryKind {
    InterfaceOrContract,
    AbstractClass,
    AbstractFunction,
    ContractDependency,
    AbstractionModule,
}

impl ProjectAbsToolkitEntryKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InterfaceOrContract => "interface_or_contract",
            Self::AbstractClass => "abstract_class",
            Self::AbstractFunction => "abstract_function",
            Self::ContractDependency => "contract_dependency",
            Self::AbstractionModule => "abstraction_module",
        }
    }
}

impl From<AbstractionMemoryEntryKind> for ProjectAbsToolkitEntryKind {
    fn from(value: AbstractionMemoryEntryKind) -> Self {
        match value {
            AbstractionMemoryEntryKind::InterfaceOrContract => Self::InterfaceOrContract,
            AbstractionMemoryEntryKind::AbstractClass => Self::AbstractClass,
            AbstractionMemoryEntryKind::AbstractFunction => Self::AbstractFunction,
            AbstractionMemoryEntryKind::ContractDependency => Self::ContractDependency,
            AbstractionMemoryEntryKind::AbstractionModule => Self::AbstractionModule,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProjectAbsToolkitStatus {
    Candidate,
    #[default]
    Promoted,
    Shared,
    Deprecated,
}

impl ProjectAbsToolkitStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Candidate => "candidate",
            Self::Promoted => "promoted",
            Self::Shared => "shared",
            Self::Deprecated => "deprecated",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProjectAbsToolkitReusePolicy {
    #[default]
    ReuseFirst,
    ReuseWithAdaptation,
    ReferenceOnly,
}

impl ProjectAbsToolkitReusePolicy {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ReuseFirst => "reuse_first",
            Self::ReuseWithAdaptation => "reuse_with_adaptation",
            Self::ReferenceOnly => "reference_only",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProjectAbsToolkitStability {
    Experimental,
    #[default]
    Candidate,
    Stable,
    Canonical,
}

impl ProjectAbsToolkitStability {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Experimental => "experimental",
            Self::Candidate => "candidate",
            Self::Stable => "stable",
            Self::Canonical => "canonical",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectAbsToolkitEntry {
    pub id: String,
    pub name: String,
    pub kind: ProjectAbsToolkitEntryKind,
    pub status: ProjectAbsToolkitStatus,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub subcategory: String,
    #[serde(rename = "toolkitRelativeDir", default)]
    pub toolkit_relative_dir: String,
    #[serde(rename = "toolkitDocPath", default)]
    pub toolkit_doc_path: String,
    #[serde(rename = "sourceEntryId")]
    pub source_entry_id: String,
    #[serde(rename = "primarySourcePath")]
    pub primary_source_path: String,
    #[serde(rename = "sourcePaths")]
    pub source_paths: Vec<String>,
    #[serde(rename = "providerNodeIds")]
    pub provider_node_ids: Vec<String>,
    #[serde(rename = "consumerNodeIds")]
    pub consumer_node_ids: Vec<String>,
    #[serde(rename = "relatedAbstractions")]
    pub related_abstractions: Vec<String>,
    pub signatures: Vec<String>,
    pub tags: Vec<String>,
    #[serde(rename = "abstractionRatio")]
    pub abstraction_ratio: f64,
    #[serde(rename = "reusabilityScore")]
    pub reusability_score: f64,
    #[serde(rename = "whyReusable")]
    pub why_reusable: String,
    pub intent: String,
    #[serde(rename = "reusePolicy")]
    pub reuse_policy: ProjectAbsToolkitReusePolicy,
    pub applicability: Vec<String>,
    #[serde(rename = "nonApplicability")]
    pub non_applicability: Vec<String>,
    #[serde(rename = "adaptationRules")]
    pub adaptation_rules: Vec<String>,
    pub examples: Vec<String>,
    pub owner: String,
    pub stability: ProjectAbsToolkitStability,
    pub notes: String,
    #[serde(rename = "promotedAt")]
    pub promoted_at: String,
    #[serde(
        rename = "lastReviewedAt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub last_reviewed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectAbsToolkitArtifact {
    #[serde(rename = "schemaVersion")]
    pub schema_version: String,
    #[serde(rename = "generatedAt")]
    pub generated_at: String,
    pub project: String,
    #[serde(rename = "sourceMemoryFile")]
    pub source_memory_file: String,
    #[serde(rename = "sourceMapFile")]
    pub source_map_file: String,
    pub summary: ProjectAbsToolkitSummary,
    pub entries: Vec<ProjectAbsToolkitEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectAbsToolkitSummary {
    #[serde(rename = "scannedMemoryEntryCount")]
    pub scanned_memory_entry_count: usize,
    #[serde(rename = "promotedEntryCount")]
    pub promoted_entry_count: usize,
    #[serde(rename = "categoryCount", default)]
    pub category_count: usize,
    #[serde(rename = "subcategoryCount", default)]
    pub subcategory_count: usize,
    #[serde(rename = "reuseFirstEntryCount")]
    pub reuse_first_entry_count: usize,
    #[serde(rename = "stableEntryCount")]
    pub stable_entry_count: usize,
    #[serde(rename = "canonicalEntryCount")]
    pub canonical_entry_count: usize,
    #[serde(rename = "experimentalEntryCount")]
    pub experimental_entry_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectAbsToolkitSearchResult {
    pub entry: ProjectAbsToolkitEntry,
    pub score: f64,
    #[serde(rename = "matchedTerms")]
    pub matched_terms: Vec<String>,
}
