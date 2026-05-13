//! `/triadmind` slash command - inspect and manage TriadMind artifacts.

use std::collections::HashSet;
use std::path::PathBuf;

use deepseek_triadmind::abstraction_memory::{
    AbstractionMemoryArtifact, AbstractionMemoryEntry, AbstractionMemorySearchResult,
    ensure_abstraction_memory, load_abstraction_memory, search_abstraction_memory,
    sync_abstraction_memory,
};
use deepseek_triadmind::config::{TriadConfig, WorkspacePaths, load_triad_config};

use super::CommandResult;
use crate::tui::app::App;

const TRIADMIND_USAGE: &str = "/triadmind memory [show|search <query>|sync|path|help]";

struct TriadMemoryContext {
    paths: WorkspacePaths,
    config: TriadConfig,
    stable_source_paths: HashSet<String>,
    project_name: String,
}

pub fn triadmind(app: &mut App, arg: Option<&str>) -> CommandResult {
    let input = arg.unwrap_or("help").trim();
    let (command, rest) = split_once(input);

    match command.to_ascii_lowercase().as_str() {
        "" | "help" => CommandResult::message(triadmind_help(&app.workspace)),
        "memory" => triadmind_memory(app, rest),
        _ => CommandResult::error(format!(
            "unknown triadmind subcommand `{command}`. Try `/triadmind help`.\n\n{}",
            triadmind_help(&app.workspace)
        )),
    }
}

fn triadmind_memory(app: &App, arg: Option<&str>) -> CommandResult {
    let context = build_context(&app.workspace);
    let sub = arg.unwrap_or("show").trim();
    let (command, rest) = split_once(sub);

    match command.to_ascii_lowercase().as_str() {
        "" | "show" => match load_or_sync_artifact(&context, false) {
            Ok(artifact) => CommandResult::message(format_artifact_overview(&context, &artifact)),
            Err(err) => CommandResult::error(err),
        },
        "path" => CommandResult::message(format!(
            "TriadMind abstraction memory path: {}",
            context.paths.abstraction_memory_file.display()
        )),
        "search" => {
            let Some(query) = rest.map(str::trim).filter(|query| !query.is_empty()) else {
                return CommandResult::error("Usage: /triadmind memory search <query>");
            };

            match load_or_sync_artifact(&context, false) {
                Ok(artifact) => CommandResult::message(format_search_results(
                    &context,
                    query,
                    &artifact,
                    &search_abstraction_memory(
                        &artifact,
                        query,
                        context.config.abstraction_memory.max_search_results,
                    ),
                )),
                Err(err) => CommandResult::error(err),
            }
        }
        "sync" => match load_or_sync_artifact(&context, true) {
            Ok(artifact) => CommandResult::message(format_sync_result(&context, &artifact)),
            Err(err) => CommandResult::error(err),
        },
        "help" => CommandResult::message(memory_help(&context)),
        _ => CommandResult::error(format!(
            "unknown subcommand `{command}`. Try `/triadmind memory help`.\n\n{}",
            memory_help(&context)
        )),
    }
}

fn build_context(workspace: &PathBuf) -> TriadMemoryContext {
    let paths = WorkspacePaths::new(workspace.clone());
    let config = load_triad_config(&paths);
    let project_name = workspace
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("project")
        .to_string();

    TriadMemoryContext {
        stable_source_paths: collect_stable_source_paths(&config),
        paths,
        config,
        project_name,
    }
}

fn triadmind_help(workspace: &std::path::Path) -> String {
    let context = build_context(&workspace.to_path_buf());
    memory_help(&context)
}

fn memory_help(context: &TriadMemoryContext) -> String {
    format!(
        "Inspect or manage TriadMind abstraction memory.\n\n\
         Usage: {TRIADMIND_USAGE}\n\n\
         Workspace: {}\n\
         Map file: {}\n\
         Memory file: {}\n\
         Enabled: {}\n\n\
         Subcommands:\n\
           /triadmind memory show              Show summary and top reusable abstractions\n\
           /triadmind memory search <query>    Search remembered abstractions by name, tags, or signatures\n\
           /triadmind memory sync              Rebuild abstraction-memory.json from triad-map.json\n\
           /triadmind memory path              Print the resolved abstraction memory path\n\
           /triadmind memory help              Show this help",
        context.paths.project_root.display(),
        context.paths.map_file.display(),
        context.paths.abstraction_memory_file.display(),
        if context.config.abstraction_memory.enabled {
            "yes"
        } else {
            "no"
        }
    )
}

fn collect_stable_source_paths(config: &TriadConfig) -> HashSet<String> {
    config
        .topology_risk
        .mature_stable_source_paths
        .iter()
        .map(|path| {
            path.replace('\\', "/")
                .trim_start_matches("./")
                .trim_end_matches('/')
                .to_string()
        })
        .collect()
}

fn load_or_sync_artifact(
    context: &TriadMemoryContext,
    force_sync: bool,
) -> Result<AbstractionMemoryArtifact, String> {
    if !context.paths.triad_dir.exists() {
        return Err(format!(
            "TriadMind workspace is not initialized in {}. Expected directory {}.",
            context.paths.project_root.display(),
            context.paths.triad_dir.display()
        ));
    }

    if force_sync {
        if !context.config.abstraction_memory.enabled {
            return Err(disabled_message(&context.paths));
        }
        if !context.paths.map_file.exists() {
            return Err(missing_map_message(&context.paths));
        }
        return sync_abstraction_memory(
            &context.paths.map_file,
            &context.paths.abstraction_memory_file,
            &context.project_name,
            &context.config.abstraction_memory,
            &context.stable_source_paths,
        )
        .map_err(|err| format!("failed to sync abstraction memory: {err}"));
    }

    if context.paths.abstraction_memory_file.exists() {
        if let Some(artifact) = load_abstraction_memory(&context.paths.abstraction_memory_file) {
            return Ok(artifact);
        }
    }

    if !context.config.abstraction_memory.enabled {
        return Err(disabled_message(&context.paths));
    }
    if !context.paths.map_file.exists() {
        return Err(missing_map_message(&context.paths));
    }

    ensure_abstraction_memory(
        &context.paths.map_file,
        &context.paths.abstraction_memory_file,
        &context.project_name,
        &context.config.abstraction_memory,
        &context.stable_source_paths,
        false,
    )
    .map_err(|err| format!("failed to load abstraction memory: {err}"))
}

fn disabled_message(paths: &WorkspacePaths) -> String {
    format!(
        "TriadMind abstraction memory is disabled. Enable `abstractionMemory.enabled` in {} and retry.",
        paths.config_file.display()
    )
}

fn missing_map_message(paths: &WorkspacePaths) -> String {
    format!(
        "TriadMind topology map not found at {}. Generate `triad-map.json` first, then retry.",
        paths.map_file.display()
    )
}

fn format_sync_result(
    context: &TriadMemoryContext,
    artifact: &AbstractionMemoryArtifact,
) -> String {
    format!(
        "TriadMind abstraction memory synced.\n\n\
         Path: {}\n\
         Remembered entries: {}\n\
         Scanned sources: {}\n\
         Excluded stable sources: {}\n\
         Excluded configured sources: {}",
        context.paths.abstraction_memory_file.display(),
        artifact.summary.remembered_entry_count,
        artifact.summary.scanned_source_count,
        artifact.summary.excluded_stable_source_count,
        artifact.summary.excluded_configured_source_count,
    )
}

fn format_artifact_overview(
    context: &TriadMemoryContext,
    artifact: &AbstractionMemoryArtifact,
) -> String {
    let mut output = format!(
        "TriadMind abstraction memory\n\n\
         Project: {}\n\
         Generated: {}\n\
         Path: {}\n\
         Remembered entries: {}\n\
         Scanned sources: {}\n\
         Excluded stable sources: {}\n\
         Excluded configured sources: {}\n\
         Contract entries: {}\n\
         Abstract function entries: {}\n\
         Module entries: {}\n\
         Hotspots: {}\n\
         Variant clusters: {}",
        artifact.project,
        artifact.generated_at,
        context.paths.abstraction_memory_file.display(),
        artifact.summary.remembered_entry_count,
        artifact.summary.scanned_source_count,
        artifact.summary.excluded_stable_source_count,
        artifact.summary.excluded_configured_source_count,
        artifact.summary.contract_entry_count,
        artifact.summary.abstract_function_entry_count,
        artifact.summary.module_entry_count,
        artifact.summary.hotspot_count,
        artifact.summary.variant_cluster_count,
    );

    if artifact.entries.is_empty() {
        output.push_str("\n\nNo abstraction entries remembered yet.");
        return output;
    }

    output.push_str("\n\nTop remembered abstractions:");
    for (index, entry) in artifact.entries.iter().take(8).enumerate() {
        output.push_str(&format_entry_line(index + 1, entry));
    }

    output
}

fn format_search_results(
    context: &TriadMemoryContext,
    query: &str,
    artifact: &AbstractionMemoryArtifact,
    results: &[AbstractionMemorySearchResult],
) -> String {
    let mut output = format!(
        "TriadMind abstraction memory search\n\n\
         Query: {}\n\
         Path: {}\n\
         Total remembered entries: {}",
        query,
        context.paths.abstraction_memory_file.display(),
        artifact.summary.remembered_entry_count,
    );

    if results.is_empty() {
        output.push_str("\n\nNo matching abstractions found.");
        return output;
    }

    output.push_str("\n\nMatches:");
    for (index, result) in results.iter().enumerate() {
        output.push_str(&format_search_line(index + 1, result));
    }

    output
}

fn format_entry_line(index: usize, entry: &AbstractionMemoryEntry) -> String {
    let signatures = if entry.signatures.is_empty() {
        String::new()
    } else {
        format!("\n   signature: {}", entry.signatures[0])
    };

    format!(
        "\n\n{index}. {} [{}] reuse={:.2} abstraction={:.2}\n   source: {}\n   why: {}{}",
        entry.name,
        entry.kind.as_str(),
        entry.reusability_score,
        entry.abstraction_ratio,
        entry.primary_source_path,
        entry.why_reusable,
        signatures,
    )
}

fn format_search_line(index: usize, result: &AbstractionMemorySearchResult) -> String {
    let matched_terms = if result.matched_terms.is_empty() {
        "none".to_string()
    } else {
        result.matched_terms.join(", ")
    };

    format!(
        "\n\n{index}. {} [{}] score={:.2}\n   matched: {}\n   source: {}\n   why: {}",
        result.entry.name,
        result.entry.kind.as_str(),
        result.score,
        matched_terms,
        result.entry.primary_source_path,
        result.entry.why_reusable,
    )
}

fn split_once(input: &str) -> (&str, Option<&str>) {
    match input.find(char::is_whitespace) {
        Some(index) => (&input[..index], Some(input[index..].trim())),
        None => (input, None),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::json;
    use tempfile::TempDir;

    use super::*;
    use crate::config::Config;
    use crate::tui::app::{App, TuiOptions};
    use deepseek_triadmind::abstraction_memory::{
        AbstractionMemoryEntryKind, AbstractionMemorySummary,
    };

    fn create_test_app(tmpdir: &TempDir) -> App {
        let options = TuiOptions {
            model: "deepseek-v4-pro".to_string(),
            workspace: tmpdir.path().to_path_buf(),
            config_path: None,
            config_profile: None,
            allow_shell: false,
            use_alt_screen: true,
            use_mouse_capture: false,
            use_bracketed_paste: true,
            max_subagents: 1,
            skills_dir: tmpdir.path().join("skills"),
            memory_path: tmpdir.path().join("memory.md"),
            notes_path: tmpdir.path().join("notes.txt"),
            mcp_config_path: tmpdir.path().join("mcp.json"),
            use_memory: false,
            start_in_agent_mode: false,
            skip_onboarding: true,
            yolo: false,
            resume_session_id: None,
            initial_input: None,
        };
        App::new(options, &Config::default())
    }

    fn sample_artifact() -> AbstractionMemoryArtifact {
        AbstractionMemoryArtifact {
            schema_version: "1.0".into(),
            generated_at: "2026-01-01T00:00:00Z".into(),
            project: "sample".into(),
            source_map_file: ".triadmind/triad-map.json".into(),
            summary: AbstractionMemorySummary {
                scanned_source_count: 3,
                excluded_stable_source_count: 1,
                excluded_configured_source_count: 0,
                remembered_entry_count: 1,
                contract_entry_count: 1,
                abstract_function_entry_count: 1,
                module_entry_count: 0,
                hotspot_count: 1,
                variant_cluster_count: 1,
            },
            entries: vec![AbstractionMemoryEntry {
                id: "payment_strategy".into(),
                name: "PaymentStrategy".into(),
                kind: AbstractionMemoryEntryKind::InterfaceOrContract,
                primary_source_path: "src/payments/strategies.rs".into(),
                source_paths: vec!["src/payments/strategies.rs".into()],
                node_ids: vec!["StripePay.execute".into()],
                provider_node_ids: vec!["StripePay.execute".into()],
                consumer_node_ids: vec!["PaymentRouter.route".into()],
                variant_clusters: vec!["payment".into()],
                related_abstractions: Vec::new(),
                signatures: vec!["PaymentStrategy.execute(PayInput) -> PayResult".into()],
                tags: vec!["implements_contract".into()],
                abstraction_ratio: 0.60,
                reusability_score: 0.85,
                why_reusable: "Has multiple providers and at least one consumer".into(),
            }],
        }
    }

    fn write_artifact(tmpdir: &TempDir, artifact: &AbstractionMemoryArtifact) {
        let triad_dir = tmpdir.path().join(".triadmind");
        fs::create_dir_all(&triad_dir).expect("triad dir");
        fs::write(
            triad_dir.join("abstraction-memory.json"),
            serde_json::to_string_pretty(artifact).expect("serialize artifact"),
        )
        .expect("write artifact");
    }

    fn write_map(tmpdir: &TempDir) {
        let triad_dir = tmpdir.path().join(".triadmind");
        fs::create_dir_all(&triad_dir).expect("triad dir");
        fs::write(
            triad_dir.join("triad-map.json"),
            serde_json::to_string_pretty(&json!([
                {
                    "nodeId": "StripePay.execute",
                    "sourcePath": "src/payments/stripe.rs",
                    "fission": {
                        "problem": "Implement payment strategy",
                        "demand": ["PayInput"],
                        "answer": ["PayResult"],
                        "evidence": {
                            "abstraction": {
                                "role": "mixed",
                                "signals": ["implements_contract"],
                                "implements": ["PaymentStrategy"],
                                "dependsOnAbstractions": [],
                                "abstractFunctions": ["PaymentStrategy.execute(PayInput) -> PayResult"],
                                "abstractionSignalCount": 2,
                                "functionContractCount": 1,
                                "concreteSignalCount": 3
                            }
                        }
                    }
                }
            ]))
            .expect("serialize map"),
        )
        .expect("write map");
    }

    #[test]
    fn triadmind_memory_help_lists_subcommands() {
        let tmpdir = TempDir::new().expect("tempdir");
        let mut app = create_test_app(&tmpdir);
        let result = triadmind(&mut app, Some("memory help"));
        let msg = result.message.expect("help text");
        assert!(msg.contains(TRIADMIND_USAGE));
        assert!(msg.contains("/triadmind memory sync"));
        assert!(msg.contains("Memory file:"));
    }

    #[test]
    fn triadmind_memory_path_returns_resolved_file() {
        let tmpdir = TempDir::new().expect("tempdir");
        let mut app = create_test_app(&tmpdir);
        let result = triadmind(&mut app, Some("memory path"));
        let msg = result.message.expect("path text");
        assert!(msg.contains(".triadmind"));
        assert!(msg.contains("abstraction-memory.json"));
    }

    #[test]
    fn triadmind_memory_show_reads_existing_artifact() {
        let tmpdir = TempDir::new().expect("tempdir");
        write_artifact(&tmpdir, &sample_artifact());
        let mut app = create_test_app(&tmpdir);
        let result = triadmind(&mut app, Some("memory show"));
        let msg = result.message.expect("show text");
        assert!(msg.contains("Remembered entries: 1"));
        assert!(msg.contains("PaymentStrategy"));
        assert!(msg.contains("Top remembered abstractions"));
    }

    #[test]
    fn triadmind_memory_search_returns_ranked_matches() {
        let tmpdir = TempDir::new().expect("tempdir");
        write_artifact(&tmpdir, &sample_artifact());
        let mut app = create_test_app(&tmpdir);
        let result = triadmind(&mut app, Some("memory search payment strategy"));
        let msg = result.message.expect("search text");
        assert!(msg.contains("Query: payment strategy"));
        assert!(msg.contains("PaymentStrategy"));
        assert!(msg.contains("matched:"));
    }

    #[test]
    fn triadmind_memory_sync_errors_cleanly_without_map() {
        let tmpdir = TempDir::new().expect("tempdir");
        fs::create_dir_all(tmpdir.path().join(".triadmind")).expect("triad dir");
        let mut app = create_test_app(&tmpdir);
        let result = triadmind(&mut app, Some("memory sync"));
        let msg = result.message.expect("error text");
        assert!(msg.contains("triad-map.json"));
        assert!(result.is_error);
    }

    #[test]
    fn triadmind_memory_sync_builds_artifact_from_map() {
        let tmpdir = TempDir::new().expect("tempdir");
        write_map(&tmpdir);
        let mut app = create_test_app(&tmpdir);
        let result = triadmind(&mut app, Some("memory sync"));
        let msg = result.message.expect("sync text");
        assert!(msg.contains("TriadMind abstraction memory synced"));
        assert!(
            tmpdir
                .path()
                .join(".triadmind")
                .join("abstraction-memory.json")
                .exists()
        );
    }
}
