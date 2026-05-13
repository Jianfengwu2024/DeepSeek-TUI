use super::types::{
    AbstractionMemoryArtifact, AbstractionMemoryEntry, AbstractionMemorySearchResult,
};

pub fn search_abstraction_memory(
    artifact: &AbstractionMemoryArtifact,
    query: &str,
    limit: usize,
) -> Vec<AbstractionMemorySearchResult> {
    let terms = tokenize(query);
    if terms.is_empty() {
        return Vec::new();
    }

    let mut scored: Vec<(f64, &AbstractionMemoryEntry, Vec<String>)> = artifact
        .entries
        .iter()
        .filter_map(|entry| {
            let (score, matched_terms) = score_entry(entry, &terms);
            if score > 0.0 {
                Some((score, entry, matched_terms))
            } else {
                None
            }
        })
        .collect();

    scored.sort_by(|left, right| {
        right
            .0
            .partial_cmp(&left.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    scored
        .into_iter()
        .take(limit.clamp(1, 50))
        .map(
            |(score, entry, matched_terms)| AbstractionMemorySearchResult {
                entry: entry.clone(),
                score,
                matched_terms,
            },
        )
        .collect()
}

fn score_entry(entry: &AbstractionMemoryEntry, terms: &[String]) -> (f64, Vec<String>) {
    let mut score = 0.0;
    let mut matched_terms = Vec::new();

    let search_text = format!(
        "{} {} {} {} {} {}",
        entry.name,
        entry.kind.as_str(),
        entry.tags.join(" "),
        entry.signatures.join(" "),
        entry.source_paths.join(" "),
        entry.why_reusable,
    )
    .to_lowercase();

    for term in terms {
        if search_text.contains(term.as_str()) {
            score += 1.0;
            matched_terms.push(term.clone());
        } else if fuzzy_contains(&search_text, term) {
            score += 0.5;
            matched_terms.push(format!("~{}", term));
        }
    }

    score *= 1.0 + entry.reusability_score;
    (score, matched_terms)
}

pub(crate) fn tokenize(text: &str) -> Vec<String> {
    let mut terms: Vec<String> = text
        .to_lowercase()
        .split(|ch: char| !ch.is_alphanumeric() && ch != '_')
        .map(str::trim)
        .filter(|term| term.len() >= 2)
        .map(ToString::to_string)
        .collect();

    terms.sort();
    terms.dedup();
    terms
}

fn fuzzy_contains(haystack: &str, term: &str) -> bool {
    haystack.contains(&term.replace('_', "")) || haystack.contains(&term.replace('-', ""))
}

pub(crate) fn choose_preferred_reuse(
    entry: &AbstractionMemoryEntry,
    focus_node_id: Option<&str>,
) -> Option<String> {
    if let Some(focus) = focus_node_id {
        if entry.provider_node_ids.iter().any(|item| item == focus) {
            return Some(focus.to_string());
        }
    }

    entry
        .provider_node_ids
        .first()
        .cloned()
        .or_else(|| entry.node_ids.first().cloned())
}

pub(crate) fn normalize_confidence(score: f64) -> f64 {
    (score / 40.0).clamp(0.35, 0.92)
}
