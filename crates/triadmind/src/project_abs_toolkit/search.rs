use super::types::{
    ProjectAbsToolkitArtifact, ProjectAbsToolkitEntry, ProjectAbsToolkitSearchResult,
};

pub fn search_project_abs_toolkit(
    artifact: &ProjectAbsToolkitArtifact,
    query: &str,
    limit: usize,
) -> Vec<ProjectAbsToolkitSearchResult> {
    let terms = tokenize(query);
    if terms.is_empty() {
        return Vec::new();
    }

    let mut scored: Vec<(f64, &ProjectAbsToolkitEntry, Vec<String>)> = artifact
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
            |(score, entry, matched_terms)| ProjectAbsToolkitSearchResult {
                entry: entry.clone(),
                score,
                matched_terms,
            },
        )
        .collect()
}

fn score_entry(entry: &ProjectAbsToolkitEntry, terms: &[String]) -> (f64, Vec<String>) {
    let mut score = 0.0;
    let mut matched_terms = Vec::new();

    let search_text = format!(
        "{} {} {} {} {} {} {} {} {} {} {} {} {}",
        entry.name,
        entry.kind.as_str(),
        entry.category,
        entry.subcategory,
        entry.intent,
        entry.tags.join(" "),
        entry.signatures.join(" "),
        entry.applicability.join(" "),
        entry.non_applicability.join(" "),
        entry.adaptation_rules.join(" "),
        entry.primary_source_path,
        entry.toolkit_relative_dir,
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

fn tokenize(text: &str) -> Vec<String> {
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
    haystack.contains(&term.replace('_', ""))
        || haystack.contains(&term.replace('-', ""))
        || haystack.contains(&term.replace(' ', ""))
}
