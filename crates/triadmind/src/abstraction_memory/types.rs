use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AbstractionMemoryEntryKind {
    InterfaceOrContract,
    AbstractClass,
    AbstractFunction,
    ContractDependency,
    AbstractionModule,
}

impl AbstractionMemoryEntryKind {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbstractionMemoryEntry {
    pub id: String,
    pub name: String,
    pub kind: AbstractionMemoryEntryKind,
    #[serde(rename = "primarySourcePath")]
    pub primary_source_path: String,
    #[serde(rename = "sourcePaths")]
    pub source_paths: Vec<String>,
    #[serde(rename = "nodeIds")]
    pub node_ids: Vec<String>,
    #[serde(rename = "providerNodeIds")]
    pub provider_node_ids: Vec<String>,
    #[serde(rename = "consumerNodeIds")]
    pub consumer_node_ids: Vec<String>,
    #[serde(rename = "variantClusters")]
    pub variant_clusters: Vec<String>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbstractionMemoryArtifact {
    #[serde(rename = "schemaVersion")]
    pub schema_version: String,
    #[serde(rename = "generatedAt")]
    pub generated_at: String,
    pub project: String,
    #[serde(rename = "sourceMapFile")]
    pub source_map_file: String,
    pub summary: AbstractionMemorySummary,
    pub entries: Vec<AbstractionMemoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AbstractionMemorySummary {
    #[serde(rename = "scannedSourceCount")]
    pub scanned_source_count: usize,
    #[serde(rename = "excludedStableSourceCount")]
    pub excluded_stable_source_count: usize,
    #[serde(rename = "excludedConfiguredSourceCount")]
    pub excluded_configured_source_count: usize,
    #[serde(rename = "rememberedEntryCount")]
    pub remembered_entry_count: usize,
    #[serde(rename = "contractEntryCount")]
    pub contract_entry_count: usize,
    #[serde(rename = "abstractFunctionEntryCount")]
    pub abstract_function_entry_count: usize,
    #[serde(rename = "moduleEntryCount")]
    pub module_entry_count: usize,
    #[serde(rename = "hotspotCount")]
    pub hotspot_count: usize,
    #[serde(rename = "variantClusterCount")]
    pub variant_cluster_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbstractionMemorySearchResult {
    pub entry: AbstractionMemoryEntry,
    pub score: f64,
    #[serde(rename = "matchedTerms")]
    pub matched_terms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbstractionMemoryRecommendation {
    pub entry: AbstractionMemoryEntry,
    pub score: f64,
    #[serde(rename = "matchedTerms")]
    pub matched_terms: Vec<String>,
    pub rationale: Vec<String>,
    #[serde(rename = "suggestedUsage")]
    pub suggested_usage: SuggestedUsage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestedUsage {
    ReuseFirst,
    AdaptBeforeCreate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbstractionProtocolActionCandidate {
    pub kind: String,
    #[serde(flatten)]
    pub action: ProtocolActionSeed,
    pub score: f64,
    #[serde(rename = "basedOnEntry")]
    pub based_on_entry: AbstractionMemoryEntryRef,
    pub rationale: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbstractionMemoryEntryRef {
    pub id: String,
    pub name: String,
    pub kind: AbstractionMemoryEntryKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum ProtocolActionSeed {
    #[serde(rename = "reuse")]
    Reuse(ReuseActionSeed),
    #[serde(rename = "modify")]
    Modify(ModifyActionSeed),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReuseActionSeed {
    #[serde(rename = "nodeId")]
    pub node_id: String,
    pub reason: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModifyActionSeed {
    #[serde(rename = "nodeId")]
    pub node_id: String,
    pub reason: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptMemoryContext {
    #[serde(rename = "summaryLines")]
    pub summary_lines: Vec<String>,
    pub matches: Vec<AbstractionMemoryEntry>,
    pub recommendations: Vec<AbstractionMemoryRecommendation>,
    #[serde(rename = "protocolActionCandidates")]
    pub protocol_action_candidates: Vec<AbstractionProtocolActionCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbstractionMemoryConfig {
    pub enabled: bool,
    #[serde(rename = "autoSyncOnPrompt", default = "default_true")]
    pub auto_sync_on_prompt: bool,
    #[serde(rename = "excludeMatureStableSources", default = "default_true")]
    pub exclude_mature_stable_sources: bool,
    #[serde(rename = "excludeSourcePaths", default)]
    pub exclude_source_paths: Vec<String>,
    #[serde(rename = "excludeSourcePathPatterns", default)]
    pub exclude_source_path_patterns: Vec<String>,
    #[serde(rename = "minAbstractionRatio", default = "default_min_ratio")]
    pub min_abstraction_ratio: f64,
    #[serde(rename = "maxPromptEntries", default = "default_max_entries")]
    pub max_prompt_entries: usize,
    #[serde(rename = "maxSearchResults", default = "default_max_search")]
    pub max_search_results: usize,
}

fn default_true() -> bool {
    true
}

fn default_min_ratio() -> f64 {
    0.2
}

fn default_max_entries() -> usize {
    6
}

fn default_max_search() -> usize {
    10
}

impl Default for AbstractionMemoryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_sync_on_prompt: true,
            exclude_mature_stable_sources: true,
            exclude_source_paths: Vec::new(),
            exclude_source_path_patterns: Vec::new(),
            min_abstraction_ratio: default_min_ratio(),
            max_prompt_entries: default_max_entries(),
            max_search_results: default_max_search(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RecommendationInput {
    pub query: String,
    pub focus_node_id: Option<String>,
    pub focus_source_path: Option<String>,
    pub limit: usize,
}
