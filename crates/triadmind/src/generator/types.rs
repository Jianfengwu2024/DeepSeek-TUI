//! Types for the code generator — options, generated files, and results.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Options controlling code generation behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratorOptions {
    /// Target language for generated code.
    pub language: String,
    /// Whether to actually write files (false = dry run).
    #[serde(default = "default_true")]
    pub write_files: bool,
    /// Base directory for generated files.
    #[serde(default)]
    pub output_dir: Option<PathBuf>,
}

fn default_true() -> bool {
    true
}

impl Default for GeneratorOptions {
    fn default() -> Self {
        Self {
            language: "rust".into(),
            write_files: true,
            output_dir: None,
        }
    }
}

/// A single generated source file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedFile {
    /// Relative file path.
    pub path: String,
    /// Generated source code content.
    pub content: String,
    /// Whether this is a new file (true) or a modification (false).
    pub is_new: bool,
    /// The action that triggered this generation.
    #[serde(rename = "sourceAction")]
    pub source_action: String,
}

/// Result of a code generation run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationResult {
    /// Project root directory.
    #[serde(rename = "projectRoot")]
    pub project_root: String,
    /// Protocol version used.
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    /// Generated or modified files.
    pub files: Vec<GeneratedFile>,
    /// Number of reuse actions (no code changes).
    #[serde(rename = "reuseCount")]
    pub reuse_count: usize,
    /// Number of modify actions.
    #[serde(rename = "modifyCount")]
    pub modify_count: usize,
    /// Number of create_child actions.
    #[serde(rename = "createCount")]
    pub create_count: usize,
}
