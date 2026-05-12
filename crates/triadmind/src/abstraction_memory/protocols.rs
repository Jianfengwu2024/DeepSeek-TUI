use std::collections::HashMap;
use std::path::Path;

use crate::protocol::TriadNodeDefinition;

use super::evidence::load_triad_nodes;
use super::recommend::recommend_abstractions;
use super::search::{choose_preferred_reuse, normalize_confidence};
use super::sync::load_abstraction_memory;
use super::types::{
    AbstractionMemoryEntryRef, AbstractionProtocolActionCandidate, ModifyActionSeed,
    ProtocolActionSeed, RecommendationInput, ReuseActionSeed,
};

pub fn build_abstraction_protocol_action_candidates(
    map_file: &Path,
    memory_file: &Path,
    input: &RecommendationInput,
) -> Result<Vec<AbstractionProtocolActionCandidate>, anyhow::Error> {
    let Some(artifact) = load_abstraction_memory(memory_file) else {
        return Ok(Vec::new());
    };

    let recommendations = recommend_abstractions(&artifact, input);
    let nodes = load_triad_nodes(map_file)?;
    let node_map: HashMap<&str, &TriadNodeDefinition> = nodes
        .iter()
        .map(|node| (node.node_id.as_str(), node))
        .collect();

    let mut candidates = Vec::new();

    for recommendation in &recommendations {
        if let Some(preferred_id) =
            choose_preferred_reuse(&recommendation.entry, input.focus_node_id.as_deref())
        {
            candidates.push(AbstractionProtocolActionCandidate {
                kind: "reuse_seed".into(),
                action: ProtocolActionSeed::Reuse(ReuseActionSeed {
                    node_id: preferred_id,
                    reason: format!(
                        "Reuse abstraction '{}' ({})",
                        recommendation.entry.name,
                        recommendation.entry.kind.as_str()
                    ),
                    confidence: normalize_confidence(recommendation.score),
                }),
                score: recommendation.score,
                based_on_entry: AbstractionMemoryEntryRef {
                    id: recommendation.entry.id.clone(),
                    name: recommendation.entry.name.clone(),
                    kind: recommendation.entry.kind,
                },
                rationale: recommendation.rationale.clone(),
            });
        }

        for consumer_id in &recommendation.entry.consumer_node_ids {
            if node_map.contains_key(consumer_id.as_str()) {
                candidates.push(AbstractionProtocolActionCandidate {
                    kind: "modify_seed".into(),
                    action: ProtocolActionSeed::Modify(ModifyActionSeed {
                        node_id: consumer_id.clone(),
                        reason: format!(
                            "Adapt consumer '{}' to use abstraction '{}'",
                            consumer_id, recommendation.entry.name
                        ),
                        confidence: normalize_confidence(recommendation.score * 0.9),
                    }),
                    score: recommendation.score * 0.9,
                    based_on_entry: AbstractionMemoryEntryRef {
                        id: recommendation.entry.id.clone(),
                        name: recommendation.entry.name.clone(),
                        kind: recommendation.entry.kind,
                    },
                    rationale: recommendation.rationale.clone(),
                });
            }
        }
    }

    Ok(candidates)
}
