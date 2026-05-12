//! # Navigate — Pre-implementation architecture impact mapping
//!
//! Ported from triadmind-core/navigator.ts
//!
//! Before writing code, generates an "impact map" showing how a proposed
//! feature intersects with the existing topology. Produces:
//! - An impact protocol (UpgradeProtocol draft)
//! - A navigator prompt for LLM-guided protocol generation
//! - Impact visualizer data
//!
//! @LeftBranch: run_navigator, build_navigator_prompt
//! @RightBranch: NavigatorRunOptions, NavigatorRunResult, ImpactMapArtifact

pub mod types;
pub mod engine;

#[cfg(test)]
mod tests;

pub use types::{
    ImpactGraphSummary, ImpactMapArtifact, NavigatorRunOptions, NavigatorRunResult,
};
pub use engine::{build_navigator_prompt, run_navigator};
