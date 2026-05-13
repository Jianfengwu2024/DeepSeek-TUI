//! # Project Abstraction Toolkit
//!
//! Curated, shareable project-level abstractions promoted from
//! `abstraction_memory`.

pub mod search;
pub mod sync;
pub mod types;

pub use search::search_project_abs_toolkit;
pub use sync::{
    ensure_project_abs_toolkit, export_project_abs_toolkit_directory,
    export_project_abs_toolkit_markdown, load_project_abs_toolkit,
    render_project_abs_toolkit_markdown, sync_project_abs_toolkit_from_memory,
};
pub use types::{
    ProjectAbsToolkitArtifact, ProjectAbsToolkitEntry, ProjectAbsToolkitEntryKind,
    ProjectAbsToolkitReusePolicy, ProjectAbsToolkitSearchResult, ProjectAbsToolkitStability,
    ProjectAbsToolkitStatus, ProjectAbsToolkitSummary,
};
