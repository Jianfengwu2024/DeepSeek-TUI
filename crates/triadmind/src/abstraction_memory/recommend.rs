use super::search::search_abstraction_memory;
use super::types::{
    AbstractionMemoryArtifact, AbstractionMemoryRecommendation, RecommendationInput, SuggestedUsage,
};

pub fn recommend_abstractions(
    artifact: &AbstractionMemoryArtifact,
    input: &RecommendationInput,
) -> Vec<AbstractionMemoryRecommendation> {
    let results =
        search_abstraction_memory(artifact, &input.query, input.limit.saturating_mul(2).max(6));
    let focus_node_id = input.focus_node_id.as_deref().unwrap_or_default();
    let focus_source_path = input.focus_source_path.as_deref().unwrap_or_default();

    results
        .into_iter()
        .take(input.limit.max(1))
        .map(|result| {
            let is_provider = !focus_node_id.is_empty()
                && result
                    .entry
                    .provider_node_ids
                    .iter()
                    .any(|node_id| node_id == focus_node_id);
            let is_consumer = !focus_node_id.is_empty()
                && result
                    .entry
                    .consumer_node_ids
                    .iter()
                    .any(|node_id| node_id == focus_node_id);
            let same_source = !focus_source_path.is_empty()
                && result.entry.source_paths.iter().any(|path| {
                    path.contains(focus_source_path) || focus_source_path.contains(path.as_str())
                });

            let mut score = result.score;
            if is_provider {
                score *= 1.15;
            }
            if is_consumer {
                score *= 1.10;
            }
            if same_source {
                score *= 1.05;
            }

            let suggested_usage = if result.entry.reusability_score >= 0.7
                && (is_provider || is_consumer || result.entry.provider_node_ids.len() >= 2)
            {
                SuggestedUsage::ReuseFirst
            } else {
                SuggestedUsage::AdaptBeforeCreate
            };

            let mut rationale = vec![result.entry.why_reusable.clone()];
            if is_provider {
                rationale.push(format!(
                    "Focus node '{}' already provides this abstraction",
                    focus_node_id
                ));
            } else if is_consumer {
                rationale.push(format!(
                    "Focus node '{}' already consumes this abstraction",
                    focus_node_id
                ));
            }
            if same_source {
                rationale.push(format!(
                    "Source path '{}' is close to the current focus",
                    result.entry.primary_source_path
                ));
            }
            if result.entry.provider_node_ids.len() >= 2 {
                rationale.push(format!(
                    "{} existing providers suggest reuse before creating a new abstraction",
                    result.entry.provider_node_ids.len()
                ));
            }

            AbstractionMemoryRecommendation {
                entry: result.entry,
                score,
                matched_terms: result.matched_terms,
                rationale,
                suggested_usage,
            }
        })
        .collect()
}
