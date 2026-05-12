//! TriadMind post-edit governance plugin.
//!
//! This module owns TriadMind-specific file filtering, sync/verify logic, and
//! diagnostic formatting. The engine only sees it through the generic
//! post-edit governance plugin interface from `governance.rs`.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use crate::core::engine::EngineConfig;

/// Check whether a tool call edited files that are source files warranting
/// TriadMind sync. Reuses the LSP hook's path extraction logic.
pub(super) fn triadmind_relevant_paths(
    tool_name: &str,
    tool_input: &serde_json::Value,
    workspace_root: &PathBuf,
) -> Vec<PathBuf> {
    let all_paths = super::lsp_hooks::edited_paths_for_tool(tool_name, tool_input);
    all_paths
        .into_iter()
        .filter(|p| {
            let abs = if p.is_absolute() {
                p.clone()
            } else {
                workspace_root.join(p)
            };
            let rel = abs
                .strip_prefix(workspace_root)
                .unwrap_or(&abs)
                .to_string_lossy()
                .replace('\\', "/");
            deepseek_triadmind::sync::is_source_file(&rel)
        })
        .collect()
}

pub(super) struct TriadMindGovernancePlugin;

pub(super) fn build_triadmind_governance_plugin(
    config: &EngineConfig,
) -> Option<Arc<dyn super::governance::PostEditGovernancePlugin>> {
    if config.triadmind_mode.post_edit_enabled() {
        Some(Arc::new(TriadMindGovernancePlugin))
    } else {
        None
    }
}

#[async_trait]
impl super::governance::PostEditGovernancePlugin for TriadMindGovernancePlugin {
    fn name(&self) -> &'static str {
        "triadmind"
    }

    async fn on_successful_edit(
        &self,
        workspace_root: &std::path::Path,
        tool_name: &str,
        tool_input: &serde_json::Value,
    ) -> Vec<String> {
        let workspace = workspace_root.to_path_buf();
        let source_paths = triadmind_relevant_paths(tool_name, tool_input, &workspace);
        if source_paths.is_empty() {
            return Vec::new();
        }

        let paths = deepseek_triadmind::config::WorkspacePaths::new(&workspace);
        match deepseek_triadmind::sync::sync_triad_map(&paths, false) {
            Ok(sync_result) => {
                if !sync_result.changed {
                    return Vec::new();
                }

                let map_path = paths.map_file;
                if !map_path.exists() {
                    return Vec::new();
                }

                match std::fs::read_to_string(&map_path) {
                    Ok(content) => {
                        let trimmed = content.trim().trim_start_matches('\u{FEFF}');
                        if let Ok(nodes) = serde_json::from_str::<
                            Vec<deepseek_triadmind::protocol::TriadNodeDefinition>,
                        >(trimmed)
                        {
                            let report = deepseek_triadmind::verify::run_topology_verify(
                                &map_path.to_string_lossy(),
                                &workspace.to_string_lossy(),
                                &nodes,
                                &Default::default(),
                            );
                            if !report.passed {
                                return vec![format_triadmind_diagnostic(&report)];
                            }
                        }
                    }
                    Err(_e) => {
                        // Map file read error - skip silently.
                    }
                }
            }
            Err(_e) => {
                // Sync error - skip silently.
            }
        }

        Vec::new()
    }
}

/// Format a verify report into a human-readable diagnostic message for the model.
fn format_triadmind_diagnostic(report: &deepseek_triadmind::verify::VerifyReport) -> String {
    let mut lines = vec![
        "TriadMind Architecture Check".to_string(),
        format!(
            "  Nodes: {} | Execute-like: {:.1}% | Ghost: {:.1}% | Empty: {}",
            report.metrics.triad_nodes,
            report.metrics.execute_like_ratio * 100.0,
            report.metrics.ghost_ratio * 100.0,
            report.metrics.empty_vertices,
        ),
    ];

    let failures: Vec<_> = report.checks.iter().filter(|c| c.status == "fail").collect();
    if !failures.is_empty() {
        lines.push(String::new());
        lines.push("  Issues:".to_string());
        for check in failures {
            lines.push(format!(
                "    - {}: {} (threshold: {}, actual: {})",
                check.key, check.detail, check.expected, check.actual
            ));
        }
        lines.push(String::new());
        lines.push(
            "  Recommendation: Run `triadmind sync --force` and review the topology map."
                .to_string(),
        );
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::engine::governance;
    use deepseek_triadmind::verify::{
        VerifyCheckResult, VerifyMetrics, VerifyReport, VerifyThresholds,
    };

    #[test]
    fn test_triadmind_relevant_paths_filters_non_source() {
        let paths = triadmind_relevant_paths(
            "write_file",
            &serde_json::json!({"path": "README.md"}),
            &PathBuf::from("/project"),
        );
        assert!(paths.is_empty());
    }

    #[test]
    fn test_triadmind_relevant_paths_accepts_rust() {
        let paths = triadmind_relevant_paths(
            "edit_file",
            &serde_json::json!({"path": "src/main.rs"}),
            &PathBuf::from("/project"),
        );
        assert!(!paths.is_empty());
    }

    #[test]
    fn test_format_diagnostic_with_failures() {
        let report = VerifyReport {
            generated_at: "2026-01-01T00:00:00Z".into(),
            project_root: "/test".into(),
            strict: false,
            thresholds: VerifyThresholds::default(),
            map_file: "triad-map.json".into(),
            passed: false,
            metrics: VerifyMetrics {
                triad_nodes: 50,
                execute_like_ratio: 0.3,
                ghost_ratio: 0.5,
                empty_vertices: 3,
                ..Default::default()
            },
            checks: vec![
                VerifyCheckResult {
                    key: "execute_like_ratio".into(),
                    status: "fail".into(),
                    expected: "<= 0.1".into(),
                    actual: "0.3".into(),
                    detail: "Too many execute-like nodes".into(),
                },
                VerifyCheckResult {
                    key: "ghost_ratio".into(),
                    status: "pass".into(),
                    expected: "<= 0.4".into(),
                    actual: "0.5".into(),
                    detail: "Ghost ratio within limits".into(),
                },
            ],
        };

        let msg = format_triadmind_diagnostic(&report);
        assert!(msg.contains("TriadMind Architecture Check"));
        assert!(msg.contains("execute_like_ratio"));
        assert!(msg.contains("Recommendation"));
    }

    #[test]
    fn test_format_diagnostic_all_pass() {
        let report = VerifyReport {
            generated_at: "2026-01-01T00:00:00Z".into(),
            project_root: "/test".into(),
            strict: false,
            thresholds: VerifyThresholds::default(),
            map_file: "triad-map.json".into(),
            passed: true,
            metrics: VerifyMetrics::default(),
            checks: vec![VerifyCheckResult {
                key: "execute_like_ratio".into(),
                status: "pass".into(),
                expected: "<= 0.1".into(),
                actual: "0.05".into(),
                detail: "All good".into(),
            }],
        };

        let msg = format_triadmind_diagnostic(&report);
        assert!(msg.contains("TriadMind Architecture Check"));
        assert!(!msg.contains("Issues:"));
    }

    #[tokio::test]
    async fn plugin_reports_stable_name() {
        let plugin = TriadMindGovernancePlugin;
        assert_eq!(
            <TriadMindGovernancePlugin as governance::PostEditGovernancePlugin>::name(&plugin),
            "triadmind"
        );
    }
}
