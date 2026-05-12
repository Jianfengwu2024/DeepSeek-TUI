//! # Abstraction Memory
//!
//! Reusable abstraction recall for the agent. Ported from `triadmind-core`
//! and adapted for the native Rust integration.

pub mod builder;
pub mod evidence;
pub mod prompt;
pub mod protocols;
pub mod recommend;
pub mod search;
pub mod sync;
pub mod types;

#[cfg(test)]
mod tests;

pub use builder::build_abstraction_memory;
pub use evidence::AbstractionEvidence;
pub use prompt::{build_prompt_context, render_prompt_context};
pub use protocols::build_abstraction_protocol_action_candidates;
pub use recommend::recommend_abstractions;
pub use search::search_abstraction_memory;
pub use sync::{ensure_abstraction_memory, load_abstraction_memory, sync_abstraction_memory};
pub use types::{
    AbstractionMemoryArtifact, AbstractionMemoryConfig, AbstractionMemoryEntry,
    AbstractionMemoryEntryKind, AbstractionMemoryEntryRef, AbstractionMemoryRecommendation,
    AbstractionMemorySearchResult, AbstractionMemorySummary, AbstractionProtocolActionCandidate,
    ModifyActionSeed, PromptMemoryContext, ProtocolActionSeed, RecommendationInput,
    ReuseActionSeed, SuggestedUsage,
};
