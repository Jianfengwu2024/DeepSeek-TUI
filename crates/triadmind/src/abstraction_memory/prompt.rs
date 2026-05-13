use std::path::Path;

use super::recommend::recommend_abstractions;
use super::search::{choose_preferred_reuse, normalize_confidence};
use super::sync::load_abstraction_memory;
use super::types::{
    AbstractionMemoryConfig, AbstractionMemoryEntry, AbstractionMemoryEntryRef,
    AbstractionProtocolActionCandidate, PromptMemoryContext, ProtocolActionSeed,
    RecommendationInput, ReuseActionSeed,
};

pub fn build_prompt_context(
    _map_file: &Path,
    memory_file: &Path,
    input: &RecommendationInput,
    config: &AbstractionMemoryConfig,
) -> PromptMemoryContext {
    let Some(artifact) = load_abstraction_memory(memory_file) else {
        return empty_prompt_context();
    };

    let max_entries = config.max_prompt_entries.max(1);
    let recommendations = recommend_abstractions(
        &artifact,
        &RecommendationInput {
            query: input.query.clone(),
            focus_node_id: input.focus_node_id.clone(),
            focus_source_path: input.focus_source_path.clone(),
            limit: max_entries,
        },
    );

    let matches: Vec<AbstractionMemoryEntry> = recommendations
        .iter()
        .map(|recommendation| recommendation.entry.clone())
        .collect();

    let protocol_action_candidates: Vec<AbstractionProtocolActionCandidate> = recommendations
        .iter()
        .filter_map(|recommendation| {
            choose_preferred_reuse(&recommendation.entry, input.focus_node_id.as_deref()).map(
                |preferred_id| AbstractionProtocolActionCandidate {
                    kind: "reuse_seed".into(),
                    action: ProtocolActionSeed::Reuse(ReuseActionSeed {
                        node_id: preferred_id,
                        reason: format!("Reuse abstraction '{}'", recommendation.entry.name),
                        confidence: normalize_confidence(recommendation.score),
                    }),
                    score: recommendation.score,
                    based_on_entry: AbstractionMemoryEntryRef {
                        id: recommendation.entry.id.clone(),
                        name: recommendation.entry.name.clone(),
                        kind: recommendation.entry.kind,
                    },
                    rationale: recommendation.rationale.clone(),
                },
            )
        })
        .take(max_entries)
        .collect();

    let summary_lines = if artifact.entries.is_empty() {
        vec![
            "No abstraction memory entries were recorded yet.".into(),
            "Proceed with mount-point and impact analysis, then create reusable abstractions only if no existing tool fits."
                .into(),
        ]
    } else {
        let top_recommendation = recommendations
            .first()
            .map(|recommendation| recommendation.entry.name.as_str())
            .or_else(|| artifact.entries.first().map(|entry| entry.name.as_str()))
            .unwrap_or("none");
        vec![format!(
            "Abstraction memory: {} entries ({} contracts, {} functions). Top reusable: '{}'",
            artifact.entries.len(),
            artifact.summary.contract_entry_count,
            artifact.summary.abstract_function_entry_count,
            top_recommendation,
        )]
    };

    PromptMemoryContext {
        summary_lines,
        matches,
        recommendations,
        protocol_action_candidates,
    }
}

pub fn render_prompt_context(context: &PromptMemoryContext) -> String {
    let mut lines = vec!["[Abstraction Memory]".to_string()];
    lines.extend(context.summary_lines.iter().cloned());

    if context.matches.is_empty() {
        lines.push("- No reusable abstraction matched the current demand.".into());
        return lines.join("\n");
    }

    lines.push("- Candidate abstractions:".into());
    for entry in &context.matches {
        lines.push(format!(
            "  - {} [{}] @ {}",
            entry.name,
            entry.kind.as_str(),
            entry.primary_source_path
        ));
    }

    if !context.protocol_action_candidates.is_empty() {
        lines.push("- Suggested protocol seeds:".into());
        for candidate in &context.protocol_action_candidates {
            let node_id = match &candidate.action {
                ProtocolActionSeed::Reuse(seed) => &seed.node_id,
                ProtocolActionSeed::Modify(seed) => &seed.node_id,
            };
            lines.push(format!("  - {} -> {}", candidate.kind, node_id));
        }
    }

    lines.join("\n")
}

fn empty_prompt_context() -> PromptMemoryContext {
    PromptMemoryContext {
        summary_lines: vec!["No abstraction memory entries recorded yet.".into()],
        matches: Vec::new(),
        recommendations: Vec::new(),
        protocol_action_candidates: Vec::new(),
    }
}
