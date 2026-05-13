//! # Generator — UpgradeProtocol → source code scaffolding
//!
//! Ported from triadmind-core/generator.ts
//!
//! Translates an UpgradeProtocol into concrete code changes:
//! - `reuse` actions: verify existing nodes
//! - `modify` actions: generate code diffs for existing files
//! - `create_child` actions: generate new source files with skeleton code
//!
//! Currently supports Rust and TypeScript code generation.
//!
//! @LeftBranch: apply_protocol, generate_code_for_action
//! @RightBranch: GeneratorOptions, GeneratedFile, GenerationResult

pub mod engine;
pub mod scaffold;
pub mod types;

#[cfg(test)]
mod tests;

pub use engine::apply_protocol;
pub use scaffold::{generate_rust_function, generate_ts_function};
pub use types::{GeneratedFile, GenerationResult, GeneratorOptions};
