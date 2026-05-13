//! `/triadmind` slash command - inspect and manage TriadMind artifacts.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};

use deepseek_triadmind::abstraction_memory::{
    AbstractionMemoryArtifact, AbstractionMemoryEntry, AbstractionMemorySearchResult,
    ensure_abstraction_memory, load_abstraction_memory, search_abstraction_memory,
    sync_abstraction_memory,
};
use deepseek_triadmind::config::{TriadConfig, WorkspacePaths, load_triad_config};
use deepseek_triadmind::project_abs_toolkit::{
    ProjectAbsToolkitArtifact, ProjectAbsToolkitEntry, ProjectAbsToolkitSearchResult,
    ensure_project_abs_toolkit, export_project_abs_toolkit_directory,
    export_project_abs_toolkit_markdown, load_project_abs_toolkit, search_project_abs_toolkit,
    sync_project_abs_toolkit_from_memory,
};
use serde::{Deserialize, Serialize};

use super::CommandResult;
use crate::tui::app::App;

const TRIADMIND_USAGE: &str = "/triadmind <memory|toolkit> [show|search <query>|sync|path|export|reclassify <entry-id> <category> <subcategory>|reclassify-batch <entry-id> <category> <subcategory> ...|whitelist <show|add|remove|clear|path|help>|help]";
const MEMORY_USAGE: &str = "/triadmind memory [show|search <query>|sync|path|help]";
const TOOLKIT_USAGE: &str = "/triadmind toolkit [show|search <query>|sync|path|export|reclassify <entry-id> <category> <subcategory>|reclassify-batch <entry-id> <category> <subcategory> ...|whitelist <show|add|remove|clear|path|help>|help]";
const TOOLKIT_TAXONOMY_FILE_NAME: &str = "project-abs-toolkit-taxonomy.json";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ToolkitTaxonomy {
    #[serde(rename = "schemaVersion")]
    schema_version: String,
    categories: BTreeMap<String, BTreeSet<String>>,
}

#[derive(Debug, Clone)]
struct ReclassOp {
    entry_id: String,
    category: String,
    subcategory: String,
}

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
        "toolkit" => triadmind_toolkit(app, rest),
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

fn triadmind_toolkit(app: &App, arg: Option<&str>) -> CommandResult {
    let context = build_context(&app.workspace);
    let sub = arg.unwrap_or("show").trim();
    let (command, rest) = split_once(sub);

    match command.to_ascii_lowercase().as_str() {
        "" | "show" => match load_or_sync_toolkit(&context, false) {
            Ok(artifact) => CommandResult::message(format_toolkit_overview(&context, &artifact)),
            Err(err) => CommandResult::error(err),
        },
        "path" => CommandResult::message(format!(
            "TriadMind project abstraction toolkit paths:\nJSON: {}\nMarkdown: {}\nDirectory: {}",
            context.paths.project_abs_toolkit_file.display(),
            context.paths.project_abs_toolkit_markdown_file.display(),
            context.paths.project_abs_toolkit_dir.display(),
        )),
        "search" => {
            let Some(query) = rest.map(str::trim).filter(|query| !query.is_empty()) else {
                return CommandResult::error("Usage: /triadmind toolkit search <query>");
            };

            match load_or_sync_toolkit(&context, false) {
                Ok(artifact) => CommandResult::message(format_toolkit_search_results(
                    &context,
                    query,
                    &artifact,
                    &search_project_abs_toolkit(&artifact, query, 10),
                )),
                Err(err) => CommandResult::error(err),
            }
        }
        "sync" => match load_or_sync_toolkit(&context, true) {
            Ok(artifact) => CommandResult::message(format_toolkit_sync_result(&context, &artifact)),
            Err(err) => CommandResult::error(err),
        },
        "export" => match load_or_sync_toolkit(&context, false) {
            Ok(artifact) => match export_project_abs_toolkit_markdown(
                &artifact,
                &context.paths.project_abs_toolkit_markdown_file,
            ) {
                Ok(()) => match export_project_abs_toolkit_directory(
                    &artifact,
                    &context.paths.project_abs_toolkit_dir,
                ) {
                    Ok(()) => CommandResult::message(format!(
                        "TriadMind project abstraction toolkit exported.\n\nMarkdown: {}\nDirectory: {}",
                        context.paths.project_abs_toolkit_markdown_file.display(),
                        context.paths.project_abs_toolkit_dir.display(),
                    )),
                    Err(err) => CommandResult::error(format!(
                        "failed to export project abstraction toolkit directory: {err}"
                    )),
                },
                Err(err) => CommandResult::error(format!(
                    "failed to export project abstraction toolkit markdown: {err}"
                )),
            },
            Err(err) => CommandResult::error(err),
        },
        "reclassify" => {
            let Some(args) = rest.map(str::trim).filter(|value| !value.is_empty()) else {
                return CommandResult::error(
                    "Usage: /triadmind toolkit reclassify <entry-id> <category> <subcategory>",
                );
            };
            match reclassify_toolkit_entry(&context, args) {
                Ok(msg) => CommandResult::message(msg),
                Err(err) => CommandResult::error(err),
            }
        }
        "reclassify-batch" => {
            let Some(args) = rest.map(str::trim).filter(|value| !value.is_empty()) else {
                return CommandResult::error(
                    "Usage: /triadmind toolkit reclassify-batch <entry-id> <category> <subcategory> ...",
                );
            };
            match reclassify_toolkit_entries_batch(&context, args) {
                Ok(msg) => CommandResult::message(msg),
                Err(err) => CommandResult::error(err),
            }
        }
        "whitelist" => match toolkit_whitelist(&context, rest) {
            Ok(msg) => CommandResult::message(msg),
            Err(err) => CommandResult::error(err),
        },
        "help" => CommandResult::message(toolkit_help(&context)),
        _ => CommandResult::error(format!(
            "unknown subcommand `{command}`. Try `/triadmind toolkit help`.\n\n{}",
            toolkit_help(&context)
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
    format!(
        "Inspect or manage TriadMind reusable architecture artifacts.\n\n\
         Usage: {TRIADMIND_USAGE}\n\n\
         Workspace: {}\n\
         Map file: {}\n\
         Memory file: {}\n\
         Toolkit file: {}\n\
         Toolkit markdown: {}\n\
         Toolkit directory: {}\n\
         Abstraction memory enabled: {}\n\n\
         Namespaces:\n\
           /triadmind memory  Raw discovered abstractions remembered from the topology\n\
           /triadmind toolkit Curated project abstractions to share and reuse first\n\n\
         Quick commands:\n\
           {}\n\
           {}",
        context.paths.project_root.display(),
        context.paths.map_file.display(),
        context.paths.abstraction_memory_file.display(),
        context.paths.project_abs_toolkit_file.display(),
        context.paths.project_abs_toolkit_markdown_file.display(),
        context.paths.project_abs_toolkit_dir.display(),
        if context.config.abstraction_memory.enabled {
            "yes"
        } else {
            "no"
        },
        MEMORY_USAGE,
        TOOLKIT_USAGE,
    )
}

fn memory_help(context: &TriadMemoryContext) -> String {
    format!(
        "Inspect or manage TriadMind abstraction memory.\n\n\
         Usage: {MEMORY_USAGE}\n\n\
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

fn toolkit_help(context: &TriadMemoryContext) -> String {
    format!(
        "Inspect or manage the curated TriadMind project abstraction toolkit.\n\n\
         Usage: {TOOLKIT_USAGE}\n\n\
         Workspace: {}\n\
         Memory file: {}\n\
         Toolkit file: {}\n\
         Toolkit markdown: {}\n\
         Toolkit directory: {}\n\n\
         Subcommands:\n\
           /triadmind toolkit show             Show summary and promoted project abstractions\n\
           /triadmind toolkit search <query>   Search promoted abstractions by intent, tags, or signatures\n\
           /triadmind toolkit sync             Promote reusable memory entries into project-abs-toolkit.json\n\
           /triadmind toolkit export           Export project-abs-toolkit.md for teammates\n\
           /triadmind toolkit reclassify <entry-id> <category> <subcategory>\n\
                                                Move an abstraction into another taxonomy bucket\n\
           /triadmind toolkit reclassify-batch <entry-id> <category> <subcategory> ...\n\
                                                Reclassify multiple entries in one command\n\
           /triadmind toolkit whitelist <show|add|remove|clear|path|help>\n\
                                                Manage team taxonomy whitelist for category/subcategory\n\
           /triadmind toolkit path             Print resolved toolkit JSON and markdown paths\n\
           /triadmind toolkit help             Show this help",
        context.paths.project_root.display(),
        context.paths.abstraction_memory_file.display(),
        context.paths.project_abs_toolkit_file.display(),
        context.paths.project_abs_toolkit_markdown_file.display(),
        context.paths.project_abs_toolkit_dir.display(),
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

fn load_or_sync_toolkit(
    context: &TriadMemoryContext,
    force_sync: bool,
) -> Result<ProjectAbsToolkitArtifact, String> {
    if !context.paths.triad_dir.exists() {
        return Err(format!(
            "TriadMind workspace is not initialized in {}. Expected directory {}.",
            context.paths.project_root.display(),
            context.paths.triad_dir.display()
        ));
    }

    if !force_sync && context.paths.project_abs_toolkit_file.exists() {
        if let Some(artifact) = load_project_abs_toolkit(&context.paths.project_abs_toolkit_file) {
            return Ok(artifact);
        }
    }

    if !context.paths.abstraction_memory_file.exists() {
        if !context.config.abstraction_memory.enabled {
            return Err(format!(
                "TriadMind abstraction memory is disabled, so the project abstraction toolkit cannot be promoted automatically. Enable `abstractionMemory.enabled` in {} or provide {} first.",
                context.paths.config_file.display(),
                context.paths.abstraction_memory_file.display()
            ));
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
        .map_err(|err| {
            format!("failed to build abstraction memory for toolkit promotion: {err}")
        })?;
    }

    if force_sync {
        sync_project_abs_toolkit_from_memory(
            &context.paths.abstraction_memory_file,
            &context.paths.project_abs_toolkit_file,
            &context.paths.project_abs_toolkit_markdown_file,
            &context.paths.project_abs_toolkit_dir,
            &context.project_name,
        )
        .map_err(|err| format!("failed to sync project abstraction toolkit: {err}"))
    } else {
        ensure_project_abs_toolkit(
            &context.paths.abstraction_memory_file,
            &context.paths.project_abs_toolkit_file,
            &context.paths.project_abs_toolkit_markdown_file,
            &context.paths.project_abs_toolkit_dir,
            &context.project_name,
            false,
        )
        .map_err(|err| format!("failed to load project abstraction toolkit: {err}"))
    }
}

fn reclassify_toolkit_entry(context: &TriadMemoryContext, args: &str) -> Result<String, String> {
    let op = parse_reclassify_args(args)?;
    reclassify_toolkit_ops(context, &[op], false)
}

fn reclassify_toolkit_entries_batch(
    context: &TriadMemoryContext,
    args: &str,
) -> Result<String, String> {
    let ops = parse_reclassify_batch_args(args)?;
    reclassify_toolkit_ops(context, &ops, true)
}

fn reclassify_toolkit_ops(
    context: &TriadMemoryContext,
    ops: &[ReclassOp],
    batch: bool,
) -> Result<String, String> {
    let taxonomy = load_toolkit_taxonomy(&toolkit_taxonomy_file(context))?.ok_or_else(|| {
        format!(
            "toolkit whitelist not configured. Add entries first with `/triadmind toolkit whitelist add <category> <subcategory>` in {}",
            toolkit_taxonomy_file(context).display()
        )
    })?;
    if taxonomy.categories.is_empty() {
        return Err(format!(
            "toolkit whitelist is empty in {}. Add allowed categories first with `/triadmind toolkit whitelist add <category> <subcategory>`",
            toolkit_taxonomy_file(context).display()
        ));
    }

    for op in ops {
        validate_reclassify_against_whitelist(op, &taxonomy)?;
    }

    let mut artifact = load_or_sync_toolkit(context, false)?;
    let mut changes = Vec::new();

    for op in ops {
        let (updated_entry_id, old_bucket) = {
            let entry = artifact
                .entries
                .iter_mut()
                .find(|item| item.id == op.entry_id || item.id.eq_ignore_ascii_case(&op.entry_id))
                .ok_or_else(|| format!("toolkit entry `{}` not found", op.entry_id))?;
            let old_bucket = format!("{}/{}", entry.category, entry.subcategory);
            entry.category = op.category.clone();
            entry.subcategory = op.subcategory.clone();
            entry.toolkit_relative_dir = format!("{}/{}", op.category, op.subcategory);
            entry.toolkit_doc_path =
                format!("{}/{}.md", entry.toolkit_relative_dir, slugify(&entry.id));
            (entry.id.clone(), old_bucket)
        };
        changes.push((
            updated_entry_id,
            old_bucket,
            op.category.clone(),
            op.subcategory.clone(),
        ));
    }

    refresh_toolkit_summary(&mut artifact);
    persist_toolkit_artifact(context, &artifact)?;

    if batch {
        let mut lines = vec![
            "TriadMind toolkit entries reclassified in batch.".to_string(),
            String::new(),
            format!("Updated entries: {}", changes.len()),
            format!("JSON: {}", context.paths.project_abs_toolkit_file.display()),
            format!(
                "Directory: {}",
                context.paths.project_abs_toolkit_dir.display()
            ),
            String::new(),
            "Changes:".into(),
        ];
        for (id, old_bucket, category, subcategory) in &changes {
            lines.push(format!(
                "- {}: {} -> {}/{}",
                id, old_bucket, category, subcategory
            ));
        }
        return Ok(lines.join("\n"));
    }

    let (id, old_bucket, category, subcategory) = changes
        .first()
        .ok_or_else(|| "no reclassification changes were applied".to_string())?;
    Ok(format!(
        "TriadMind toolkit entry reclassified.\n\nEntry: {}\nFrom: {}\nTo: {}/{}\nJSON: {}\nDirectory: {}",
        id,
        old_bucket,
        category,
        subcategory,
        context.paths.project_abs_toolkit_file.display(),
        context.paths.project_abs_toolkit_dir.display(),
    ))
}

fn refresh_toolkit_summary(artifact: &mut ProjectAbsToolkitArtifact) {
    let category_count = artifact
        .entries
        .iter()
        .map(|entry| entry.category.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let subcategory_count = artifact
        .entries
        .iter()
        .map(|entry| (entry.category.as_str(), entry.subcategory.as_str()))
        .collect::<BTreeSet<_>>()
        .len();
    artifact.summary.promoted_entry_count = artifact.entries.len();
    artifact.summary.category_count = category_count;
    artifact.summary.subcategory_count = subcategory_count;
    artifact.summary.reuse_first_entry_count = artifact
        .entries
        .iter()
        .filter(|entry| entry.reuse_policy.as_str() == "reuse_first")
        .count();
    artifact.summary.stable_entry_count = artifact
        .entries
        .iter()
        .filter(|entry| {
            let stability = entry.stability.as_str();
            stability == "stable" || stability == "canonical"
        })
        .count();
    artifact.summary.canonical_entry_count = artifact
        .entries
        .iter()
        .filter(|entry| entry.stability.as_str() == "canonical")
        .count();
    artifact.summary.experimental_entry_count = artifact
        .entries
        .iter()
        .filter(|entry| entry.stability.as_str() == "experimental")
        .count();
}

fn normalize_taxonomy_component(value: &str) -> Option<String> {
    let slug = slugify(value);
    if slug.is_empty() { None } else { Some(slug) }
}

fn parse_reclassify_args(args: &str) -> Result<ReclassOp, String> {
    let mut parts = args.split_whitespace();
    let entry_id = parts.next().unwrap_or_default().trim();
    let raw_category = parts.next().unwrap_or_default().trim();
    let raw_subcategory = parts.next().unwrap_or_default().trim();

    if entry_id.is_empty() || raw_category.is_empty() || raw_subcategory.is_empty() {
        return Err(
            "Usage: /triadmind toolkit reclassify <entry-id> <category> <subcategory>".into(),
        );
    }
    if parts.next().is_some() {
        return Err(
            "reclassify accepts exactly 3 arguments: <entry-id> <category> <subcategory>".into(),
        );
    }

    let category = normalize_taxonomy_component(raw_category)
        .ok_or_else(|| format!("invalid category `{raw_category}`"))?;
    let subcategory = normalize_taxonomy_component(raw_subcategory)
        .ok_or_else(|| format!("invalid subcategory `{raw_subcategory}`"))?;

    Ok(ReclassOp {
        entry_id: entry_id.to_string(),
        category,
        subcategory,
    })
}

fn parse_reclassify_batch_args(args: &str) -> Result<Vec<ReclassOp>, String> {
    let tokens: Vec<&str> = args.split_whitespace().collect();
    if tokens.is_empty() || tokens.len() % 3 != 0 {
        return Err(
            "Usage: /triadmind toolkit reclassify-batch <entry-id> <category> <subcategory> ..."
                .into(),
        );
    }

    let mut ops = Vec::new();
    for chunk in tokens.chunks_exact(3) {
        let entry_id = chunk[0].trim();
        let raw_category = chunk[1].trim();
        let raw_subcategory = chunk[2].trim();
        if entry_id.is_empty() || raw_category.is_empty() || raw_subcategory.is_empty() {
            return Err(
                "batch reclassify received an empty token; each triplet must be complete".into(),
            );
        }
        let category = normalize_taxonomy_component(raw_category)
            .ok_or_else(|| format!("invalid category `{raw_category}`"))?;
        let subcategory = normalize_taxonomy_component(raw_subcategory)
            .ok_or_else(|| format!("invalid subcategory `{raw_subcategory}`"))?;
        ops.push(ReclassOp {
            entry_id: entry_id.to_string(),
            category,
            subcategory,
        });
    }
    Ok(ops)
}

fn persist_toolkit_artifact(
    context: &TriadMemoryContext,
    artifact: &ProjectAbsToolkitArtifact,
) -> Result<(), String> {
    std::fs::write(
        &context.paths.project_abs_toolkit_file,
        serde_json::to_string_pretty(artifact)
            .map_err(|err| format!("failed to serialize toolkit JSON: {err}"))?,
    )
    .map_err(|err| format!("failed to write toolkit JSON: {err}"))?;

    export_project_abs_toolkit_markdown(artifact, &context.paths.project_abs_toolkit_markdown_file)
        .map_err(|err| format!("failed to export toolkit markdown: {err}"))?;
    export_project_abs_toolkit_directory(artifact, &context.paths.project_abs_toolkit_dir)
        .map_err(|err| format!("failed to export toolkit directory: {err}"))?;
    Ok(())
}

fn toolkit_whitelist(context: &TriadMemoryContext, arg: Option<&str>) -> Result<String, String> {
    let input = arg.unwrap_or("show").trim();
    let (command, rest) = split_once(input);
    let taxonomy_file = toolkit_taxonomy_file(context);

    match command.to_ascii_lowercase().as_str() {
        "" | "show" => {
            let Some(taxonomy) = load_toolkit_taxonomy(&taxonomy_file)? else {
                return Ok(format!(
                    "Toolkit whitelist is not configured yet.\n\nPath: {}\n\nUse `/triadmind toolkit whitelist add <category> <subcategory>` to initialize it.",
                    taxonomy_file.display()
                ));
            };
            Ok(format_toolkit_whitelist(&taxonomy, &taxonomy_file))
        }
        "path" => Ok(format!(
            "Toolkit whitelist path: {}",
            taxonomy_file.display()
        )),
        "add" => {
            let Some(args) = rest.map(str::trim).filter(|item| !item.is_empty()) else {
                return Err(
                    "Usage: /triadmind toolkit whitelist add <category> <subcategory>".into(),
                );
            };
            let mut parts = args.split_whitespace();
            let category = normalize_taxonomy_component(parts.next().unwrap_or_default())
                .ok_or_else(|| "invalid category".to_string())?;
            let subcategory = normalize_taxonomy_component(parts.next().unwrap_or_default())
                .ok_or_else(|| "invalid subcategory".to_string())?;
            if parts.next().is_some() {
                return Err(
                    "whitelist add accepts exactly 2 arguments: <category> <subcategory>".into(),
                );
            }

            let mut taxonomy =
                load_toolkit_taxonomy(&taxonomy_file)?.unwrap_or_else(|| ToolkitTaxonomy {
                    schema_version: "1.0".into(),
                    categories: BTreeMap::new(),
                });
            taxonomy
                .categories
                .entry(category.clone())
                .or_default()
                .insert(subcategory.clone());
            save_toolkit_taxonomy(&taxonomy_file, &taxonomy)?;
            Ok(format!(
                "Toolkit whitelist updated.\n\nAdded: {}/{}\nPath: {}",
                category,
                subcategory,
                taxonomy_file.display()
            ))
        }
        "remove" => {
            let Some(args) = rest.map(str::trim).filter(|item| !item.is_empty()) else {
                return Err(
                    "Usage: /triadmind toolkit whitelist remove <category> <subcategory>".into(),
                );
            };
            let mut parts = args.split_whitespace();
            let category = normalize_taxonomy_component(parts.next().unwrap_or_default())
                .ok_or_else(|| "invalid category".to_string())?;
            let subcategory = normalize_taxonomy_component(parts.next().unwrap_or_default())
                .ok_or_else(|| "invalid subcategory".to_string())?;
            if parts.next().is_some() {
                return Err(
                    "whitelist remove accepts exactly 2 arguments: <category> <subcategory>".into(),
                );
            }
            let mut taxonomy = load_toolkit_taxonomy(&taxonomy_file)?
                .ok_or_else(|| "toolkit whitelist is not configured yet".to_string())?;
            if let Some(subs) = taxonomy.categories.get_mut(&category) {
                subs.remove(&subcategory);
                if subs.is_empty() {
                    taxonomy.categories.remove(&category);
                }
            }
            save_toolkit_taxonomy(&taxonomy_file, &taxonomy)?;
            Ok(format!(
                "Toolkit whitelist updated.\n\nRemoved: {}/{}\nPath: {}",
                category,
                subcategory,
                taxonomy_file.display()
            ))
        }
        "clear" => {
            let taxonomy = ToolkitTaxonomy {
                schema_version: "1.0".into(),
                categories: BTreeMap::new(),
            };
            save_toolkit_taxonomy(&taxonomy_file, &taxonomy)?;
            Ok(format!(
                "Toolkit whitelist cleared.\n\nPath: {}",
                taxonomy_file.display()
            ))
        }
        "help" => Ok(format!(
            "Manage toolkit taxonomy whitelist.\n\nUsage: /triadmind toolkit whitelist <show|add|remove|clear|path|help>\n\nCommands:\n  /triadmind toolkit whitelist show\n  /triadmind toolkit whitelist add <category> <subcategory>\n  /triadmind toolkit whitelist remove <category> <subcategory>\n  /triadmind toolkit whitelist clear\n  /triadmind toolkit whitelist path"
        )),
        _ => Err(format!(
            "unknown whitelist subcommand `{command}`. Try `/triadmind toolkit whitelist help`."
        )),
    }
}

fn load_toolkit_taxonomy(path: &Path) -> Result<Option<ToolkitTaxonomy>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read toolkit whitelist: {err}"))?;
    let mut taxonomy: ToolkitTaxonomy = serde_json::from_str(&content)
        .map_err(|err| format!("failed to parse toolkit whitelist JSON: {err}"))?;
    if taxonomy.schema_version.trim().is_empty() {
        taxonomy.schema_version = "1.0".into();
    }
    Ok(Some(taxonomy))
}

fn save_toolkit_taxonomy(path: &Path, taxonomy: &ToolkitTaxonomy) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create whitelist directory: {err}"))?;
    }
    std::fs::write(
        path,
        serde_json::to_string_pretty(taxonomy)
            .map_err(|err| format!("failed to serialize whitelist JSON: {err}"))?,
    )
    .map_err(|err| format!("failed to write toolkit whitelist: {err}"))
}

fn toolkit_taxonomy_file(context: &TriadMemoryContext) -> PathBuf {
    context.paths.triad_dir.join(TOOLKIT_TAXONOMY_FILE_NAME)
}

fn format_toolkit_whitelist(taxonomy: &ToolkitTaxonomy, path: &Path) -> String {
    let category_count = taxonomy.categories.len();
    let subcategory_count = taxonomy
        .categories
        .values()
        .map(BTreeSet::len)
        .sum::<usize>();

    let mut lines = vec![
        "TriadMind toolkit whitelist".to_string(),
        String::new(),
        format!("- Path: {}", path.display()),
        format!("- Categories: {}", category_count),
        format!("- Subcategories: {}", subcategory_count),
        String::new(),
        "Allowed taxonomy:".into(),
    ];
    if taxonomy.categories.is_empty() {
        lines.push("- (empty)".into());
        return lines.join("\n");
    }
    for (category, subcategories) in &taxonomy.categories {
        lines.push(format!(
            "- {} -> {}",
            category,
            subcategories.iter().cloned().collect::<Vec<_>>().join(", ")
        ));
    }
    lines.join("\n")
}

fn validate_reclassify_against_whitelist(
    op: &ReclassOp,
    taxonomy: &ToolkitTaxonomy,
) -> Result<(), String> {
    let Some(subcategories) = taxonomy.categories.get(&op.category) else {
        return Err(format!(
            "category `{}` is not in toolkit whitelist. Add it with `/triadmind toolkit whitelist add {} <subcategory>` first.",
            op.category, op.category
        ));
    };
    if !subcategories.contains(&op.subcategory) {
        return Err(format!(
            "subcategory `{}/{}` is not allowed by toolkit whitelist.",
            op.category, op.subcategory
        ));
    }
    Ok(())
}

fn slugify(value: &str) -> String {
    let mut out = String::new();
    let mut prev_sep = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_sep = false;
        } else if !prev_sep {
            out.push('_');
            prev_sep = true;
        }
    }
    out.trim_matches('_').to_string()
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

fn format_toolkit_sync_result(
    context: &TriadMemoryContext,
    artifact: &ProjectAbsToolkitArtifact,
) -> String {
    format!(
        "TriadMind project abstraction toolkit synced.\n\n\
         JSON: {}\n\
         Markdown: {}\n\
         Directory: {}\n\
         Promoted entries: {}\n\
         Categories: {}\n\
         Subcategories: {}\n\
         Reuse-first entries: {}\n\
         Stable entries: {}",
        context.paths.project_abs_toolkit_file.display(),
        context.paths.project_abs_toolkit_markdown_file.display(),
        context.paths.project_abs_toolkit_dir.display(),
        artifact.summary.promoted_entry_count,
        artifact.summary.category_count,
        artifact.summary.subcategory_count,
        artifact.summary.reuse_first_entry_count,
        artifact.summary.stable_entry_count,
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

fn format_toolkit_overview(
    context: &TriadMemoryContext,
    artifact: &ProjectAbsToolkitArtifact,
) -> String {
    let mut output = format!(
        "TriadMind project abstraction toolkit\n\n\
         Project: {}\n\
         Generated: {}\n\
         JSON: {}\n\
         Markdown: {}\n\
         Directory: {}\n\
         Scanned memory entries: {}\n\
         Promoted entries: {}\n\
         Categories: {}\n\
         Subcategories: {}\n\
         Reuse-first entries: {}\n\
         Stable entries: {}\n\
         Canonical entries: {}",
        artifact.project,
        artifact.generated_at,
        context.paths.project_abs_toolkit_file.display(),
        context.paths.project_abs_toolkit_markdown_file.display(),
        context.paths.project_abs_toolkit_dir.display(),
        artifact.summary.scanned_memory_entry_count,
        artifact.summary.promoted_entry_count,
        artifact.summary.category_count,
        artifact.summary.subcategory_count,
        artifact.summary.reuse_first_entry_count,
        artifact.summary.stable_entry_count,
        artifact.summary.canonical_entry_count,
    );

    if artifact.entries.is_empty() {
        output.push_str("\n\nNo project abstractions have been promoted yet.");
        return output;
    }

    output.push_str("\n\nTop project abstractions:");
    for (index, entry) in artifact.entries.iter().take(8).enumerate() {
        output.push_str(&format_toolkit_entry_line(index + 1, entry));
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

fn format_toolkit_search_results(
    context: &TriadMemoryContext,
    query: &str,
    artifact: &ProjectAbsToolkitArtifact,
    results: &[ProjectAbsToolkitSearchResult],
) -> String {
    let mut output = format!(
        "TriadMind project abstraction toolkit search\n\n\
         Query: {}\n\
         Path: {}\n\
         Total promoted entries: {}",
        query,
        context.paths.project_abs_toolkit_file.display(),
        artifact.summary.promoted_entry_count,
    );

    if results.is_empty() {
        output.push_str("\n\nNo matching project abstractions found.");
        return output;
    }

    output.push_str("\n\nMatches:");
    for (index, result) in results.iter().enumerate() {
        output.push_str(&format_toolkit_search_line(index + 1, result));
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

fn format_toolkit_entry_line(index: usize, entry: &ProjectAbsToolkitEntry) -> String {
    let signatures = if entry.signatures.is_empty() {
        String::new()
    } else {
        format!("\n   signature: {}", entry.signatures[0])
    };

    format!(
        "\n\n{index}. {} [{}] policy={} stability={} reuse={:.2}\n   source: {}\n   intent: {}{}",
        entry.name,
        entry.kind.as_str(),
        entry.reuse_policy.as_str(),
        entry.stability.as_str(),
        entry.reusability_score,
        entry.primary_source_path,
        format!(
            "{}\n   toolkit: {}/{}",
            entry.intent, entry.category, entry.subcategory
        ),
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

fn format_toolkit_search_line(index: usize, result: &ProjectAbsToolkitSearchResult) -> String {
    let matched_terms = if result.matched_terms.is_empty() {
        "none".to_string()
    } else {
        result.matched_terms.join(", ")
    };

    format!(
        "\n\n{index}. {} [{}] score={:.2}\n   matched: {}\n   policy: {}\n   source: {}\n   intent: {}",
        result.entry.name,
        result.entry.kind.as_str(),
        result.score,
        matched_terms,
        result.entry.reuse_policy.as_str(),
        result.entry.primary_source_path,
        format!(
            "{}\n   toolkit: {}/{}",
            result.entry.intent, result.entry.category, result.entry.subcategory
        ),
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
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs;

    use serde_json::json;
    use tempfile::TempDir;

    use super::*;
    use crate::config::Config;
    use crate::tui::app::{App, TuiOptions};
    use deepseek_triadmind::abstraction_memory::{
        AbstractionMemoryEntryKind, AbstractionMemorySummary,
    };
    use deepseek_triadmind::project_abs_toolkit::{
        ProjectAbsToolkitEntryKind, ProjectAbsToolkitReusePolicy, ProjectAbsToolkitStability,
        ProjectAbsToolkitStatus, ProjectAbsToolkitSummary,
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

    fn sample_toolkit_artifact() -> ProjectAbsToolkitArtifact {
        ProjectAbsToolkitArtifact {
            schema_version: "1.0".into(),
            generated_at: "2026-01-01T00:00:00Z".into(),
            project: "sample".into(),
            source_memory_file: ".triadmind/abstraction-memory.json".into(),
            source_map_file: ".triadmind/triad-map.json".into(),
            summary: ProjectAbsToolkitSummary {
                scanned_memory_entry_count: 1,
                promoted_entry_count: 1,
                category_count: 1,
                subcategory_count: 1,
                reuse_first_entry_count: 1,
                stable_entry_count: 1,
                canonical_entry_count: 0,
                experimental_entry_count: 0,
            },
            entries: vec![ProjectAbsToolkitEntry {
                id: "payment_strategy".into(),
                name: "PaymentStrategy".into(),
                kind: ProjectAbsToolkitEntryKind::InterfaceOrContract,
                status: ProjectAbsToolkitStatus::Promoted,
                category: "payments".into(),
                subcategory: "contracts".into(),
                toolkit_relative_dir: "payments/contracts".into(),
                toolkit_doc_path: "payments/contracts/payment_strategy.md".into(),
                source_entry_id: "payment_strategy".into(),
                primary_source_path: "src/payments/strategies.rs".into(),
                source_paths: vec!["src/payments/strategies.rs".into()],
                provider_node_ids: vec!["StripePay.execute".into()],
                consumer_node_ids: vec!["PaymentRouter.route".into()],
                related_abstractions: vec![],
                signatures: vec!["PaymentStrategy.execute(PayInput) -> PayResult".into()],
                tags: vec!["implements_contract".into()],
                abstraction_ratio: 0.60,
                reusability_score: 0.85,
                why_reusable: "Has multiple providers and at least one consumer".into(),
                intent: "Use PaymentStrategy before adding a new payment abstraction.".into(),
                reuse_policy: ProjectAbsToolkitReusePolicy::ReuseFirst,
                applicability: vec!["payment routing".into()],
                non_applicability: vec!["unrelated domains".into()],
                adaptation_rules: vec!["Keep the execute contract stable".into()],
                examples: vec!["See src/payments/strategies.rs".into()],
                owner: "sample".into(),
                stability: ProjectAbsToolkitStability::Stable,
                notes: "promoted from abstraction memory".into(),
                promoted_at: "2026-01-01T00:00:00Z".into(),
                last_reviewed_at: None,
            }],
        }
    }

    fn write_toolkit_artifact(tmpdir: &TempDir, artifact: &ProjectAbsToolkitArtifact) {
        let triad_dir = tmpdir.path().join(".triadmind");
        fs::create_dir_all(&triad_dir).expect("triad dir");
        fs::write(
            triad_dir.join("project-abs-toolkit.json"),
            serde_json::to_string_pretty(artifact).expect("serialize toolkit"),
        )
        .expect("write toolkit");
    }

    fn write_toolkit_whitelist(tmpdir: &TempDir, rules: &[(&str, &str)]) {
        let triad_dir = tmpdir.path().join(".triadmind");
        fs::create_dir_all(&triad_dir).expect("triad dir");
        let mut categories: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for (category, subcategory) in rules {
            categories
                .entry((*category).to_string())
                .or_default()
                .insert((*subcategory).to_string());
        }
        let taxonomy = ToolkitTaxonomy {
            schema_version: "1.0".into(),
            categories,
        };
        fs::write(
            triad_dir.join(TOOLKIT_TAXONOMY_FILE_NAME),
            serde_json::to_string_pretty(&taxonomy).expect("serialize whitelist"),
        )
        .expect("write whitelist");
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
        assert!(msg.contains(MEMORY_USAGE));
        assert!(msg.contains("/triadmind memory sync"));
        assert!(msg.contains("Memory file:"));
    }

    #[test]
    fn triadmind_help_lists_toolkit_namespace() {
        let tmpdir = TempDir::new().expect("tempdir");
        let mut app = create_test_app(&tmpdir);
        let result = triadmind(&mut app, Some("help"));
        let msg = result.message.expect("help text");
        assert!(msg.contains(TRIADMIND_USAGE));
        assert!(msg.contains("/triadmind toolkit"));
        assert!(msg.contains("Toolkit markdown:"));
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

    #[test]
    fn triadmind_toolkit_show_reads_existing_artifact() {
        let tmpdir = TempDir::new().expect("tempdir");
        write_toolkit_artifact(&tmpdir, &sample_toolkit_artifact());
        let mut app = create_test_app(&tmpdir);
        let result = triadmind(&mut app, Some("toolkit show"));
        let msg = result.message.expect("show text");
        assert!(msg.contains("project abstraction toolkit"));
        assert!(msg.contains("Promoted entries: 1"));
        assert!(msg.contains("PaymentStrategy"));
    }

    #[test]
    fn triadmind_toolkit_search_returns_ranked_matches() {
        let tmpdir = TempDir::new().expect("tempdir");
        write_toolkit_artifact(&tmpdir, &sample_toolkit_artifact());
        let mut app = create_test_app(&tmpdir);
        let result = triadmind(&mut app, Some("toolkit search payment"));
        let msg = result.message.expect("search text");
        assert!(msg.contains("Query: payment"));
        assert!(msg.contains("PaymentStrategy"));
        assert!(msg.contains("policy: reuse_first"));
    }

    #[test]
    fn triadmind_toolkit_reclassify_updates_taxonomy_and_exports() {
        let tmpdir = TempDir::new().expect("tempdir");
        write_toolkit_artifact(&tmpdir, &sample_toolkit_artifact());
        write_toolkit_whitelist(
            &tmpdir,
            &[("platform", "billing"), ("payments", "contracts")],
        );
        let mut app = create_test_app(&tmpdir);

        let result = triadmind(
            &mut app,
            Some("toolkit reclassify payment_strategy platform billing"),
        );
        let msg = result.message.expect("reclassify text");
        assert!(msg.contains("toolkit entry reclassified"));
        assert!(msg.contains("To: platform/billing"));

        let content = fs::read_to_string(
            tmpdir
                .path()
                .join(".triadmind")
                .join("project-abs-toolkit.json"),
        )
        .expect("read toolkit");
        let artifact: ProjectAbsToolkitArtifact =
            serde_json::from_str(&content).expect("parse toolkit json");
        let entry = artifact
            .entries
            .iter()
            .find(|item| item.id == "payment_strategy")
            .expect("payment_strategy entry");
        assert_eq!(entry.category, "platform");
        assert_eq!(entry.subcategory, "billing");
        assert_eq!(
            entry.toolkit_doc_path,
            "platform/billing/payment_strategy.md"
        );
        assert!(
            tmpdir
                .path()
                .join(".triadmind")
                .join("project-abs-toolkit")
                .join("platform")
                .join("billing")
                .join("payment_strategy.md")
                .exists()
        );
    }

    #[test]
    fn triadmind_toolkit_reclassify_batch_updates_multiple_entries() {
        let tmpdir = TempDir::new().expect("tempdir");
        let mut artifact = sample_toolkit_artifact();
        artifact.entries.push(ProjectAbsToolkitEntry {
            id: "refund_strategy".into(),
            name: "RefundStrategy".into(),
            kind: ProjectAbsToolkitEntryKind::InterfaceOrContract,
            status: ProjectAbsToolkitStatus::Promoted,
            category: "payments".into(),
            subcategory: "contracts".into(),
            toolkit_relative_dir: "payments/contracts".into(),
            toolkit_doc_path: "payments/contracts/refund_strategy.md".into(),
            source_entry_id: "refund_strategy".into(),
            primary_source_path: "src/payments/refund.rs".into(),
            source_paths: vec!["src/payments/refund.rs".into()],
            provider_node_ids: vec!["RefundPay.execute".into()],
            consumer_node_ids: vec!["PaymentRouter.refund".into()],
            related_abstractions: vec![],
            signatures: vec!["RefundStrategy.execute(RefundInput) -> RefundResult".into()],
            tags: vec!["refund".into()],
            abstraction_ratio: 0.58,
            reusability_score: 0.81,
            why_reusable: "shared refund contract".into(),
            intent: "Reuse RefundStrategy before adding new refund contract".into(),
            reuse_policy: ProjectAbsToolkitReusePolicy::ReuseFirst,
            applicability: vec!["refund".into()],
            non_applicability: vec![],
            adaptation_rules: vec![],
            examples: vec![],
            owner: "sample".into(),
            stability: ProjectAbsToolkitStability::Stable,
            notes: "seed".into(),
            promoted_at: "2026-01-01T00:00:00Z".into(),
            last_reviewed_at: None,
        });
        write_toolkit_artifact(&tmpdir, &artifact);
        write_toolkit_whitelist(
            &tmpdir,
            &[("platform", "billing"), ("platform", "checkout")],
        );

        let mut app = create_test_app(&tmpdir);
        let result = triadmind(
            &mut app,
            Some(
                "toolkit reclassify-batch payment_strategy platform billing refund_strategy platform checkout",
            ),
        );
        let msg = result.message.expect("batch text");
        assert!(msg.contains("reclassified in batch"));
        assert!(msg.contains("Updated entries: 2"));

        let content = fs::read_to_string(
            tmpdir
                .path()
                .join(".triadmind")
                .join("project-abs-toolkit.json"),
        )
        .expect("read toolkit");
        let updated: ProjectAbsToolkitArtifact =
            serde_json::from_str(&content).expect("parse toolkit json");
        assert!(updated.entries.iter().any(|entry| {
            entry.id == "payment_strategy"
                && entry.category == "platform"
                && entry.subcategory == "billing"
        }));
        assert!(updated.entries.iter().any(|entry| {
            entry.id == "refund_strategy"
                && entry.category == "platform"
                && entry.subcategory == "checkout"
        }));
    }

    #[test]
    fn triadmind_toolkit_whitelist_add_show_and_remove() {
        let tmpdir = TempDir::new().expect("tempdir");
        let mut app = create_test_app(&tmpdir);

        let add_result = triadmind(&mut app, Some("toolkit whitelist add platform billing"));
        let add_msg = add_result.message.expect("add text");
        assert!(add_msg.contains("Toolkit whitelist updated"));

        let show_result = triadmind(&mut app, Some("toolkit whitelist show"));
        let show_msg = show_result.message.expect("show text");
        assert!(show_msg.contains("platform -> billing"));

        let remove_result = triadmind(&mut app, Some("toolkit whitelist remove platform billing"));
        let remove_msg = remove_result.message.expect("remove text");
        assert!(remove_msg.contains("Removed: platform/billing"));
    }

    #[test]
    fn triadmind_toolkit_sync_builds_artifact_from_memory() {
        let tmpdir = TempDir::new().expect("tempdir");
        write_artifact(&tmpdir, &sample_artifact());
        let mut app = create_test_app(&tmpdir);
        let result = triadmind(&mut app, Some("toolkit sync"));
        let msg = result.message.expect("sync text");
        assert!(msg.contains("project abstraction toolkit synced"));
        assert!(
            tmpdir
                .path()
                .join(".triadmind")
                .join("project-abs-toolkit.json")
                .exists()
        );
        assert!(
            tmpdir
                .path()
                .join(".triadmind")
                .join("project-abs-toolkit.md")
                .exists()
        );
        assert!(
            tmpdir
                .path()
                .join(".triadmind")
                .join("project-abs-toolkit")
                .join("README.md")
                .exists()
        );
    }

    #[test]
    fn triadmind_toolkit_export_writes_markdown() {
        let tmpdir = TempDir::new().expect("tempdir");
        write_toolkit_artifact(&tmpdir, &sample_toolkit_artifact());
        let mut app = create_test_app(&tmpdir);
        let result = triadmind(&mut app, Some("toolkit export"));
        let msg = result.message.expect("export text");
        assert!(msg.contains("project abstraction toolkit exported"));
        assert!(
            tmpdir
                .path()
                .join(".triadmind")
                .join("project-abs-toolkit.md")
                .exists()
        );
        assert!(
            tmpdir
                .path()
                .join(".triadmind")
                .join("project-abs-toolkit")
                .join("README.md")
                .exists()
        );
    }
}
