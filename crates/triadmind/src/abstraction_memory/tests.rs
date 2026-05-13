use std::collections::HashSet;

use serde_json::json;

use super::builder::build_abstraction_memory;
use super::evidence::extract_contract_names;
use super::prompt::build_prompt_context;
use super::recommend::recommend_abstractions;
use super::search::{search_abstraction_memory, tokenize};
use super::types::{
    AbstractionMemoryArtifact, AbstractionMemoryConfig, AbstractionMemoryEntry,
    AbstractionMemoryEntryKind, AbstractionMemorySummary, RecommendationInput, SuggestedUsage,
};

fn write_json_map(value: serde_json::Value) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "triadmind_abstraction_memory_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let map_file = root.join("triad-map.json");
    std::fs::write(&map_file, serde_json::to_string_pretty(&value).unwrap()).unwrap();
    map_file
}

#[test]
fn build_abstraction_memory_tracks_providers_consumers_and_functions() {
    let map_file = write_json_map(json!([
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
        },
        {
            "nodeId": "PaypalPay.execute",
            "sourcePath": "src/payments/paypal.rs",
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
        },
        {
            "nodeId": "PaymentRouter.route",
            "sourcePath": "src/payments/router.rs",
            "fission": {
                "problem": "Route payment through selected strategy",
                "demand": ["PaymentStrategy", "PayInput"],
                "answer": ["PayResult"],
                "evidence": {
                    "abstraction": {
                        "role": "mixed",
                        "signals": ["depends_on_contract"],
                        "implements": [],
                        "dependsOnAbstractions": ["PaymentStrategy"],
                        "abstractFunctions": [],
                        "abstractionSignalCount": 1,
                        "functionContractCount": 0,
                        "concreteSignalCount": 4
                    }
                }
            }
        }
    ]));

    let artifact = build_abstraction_memory(
        &map_file,
        "test",
        &AbstractionMemoryConfig::default(),
        &HashSet::new(),
    )
    .unwrap();

    std::fs::remove_dir_all(map_file.parent().unwrap()).unwrap();

    let contract = artifact
        .entries
        .iter()
        .find(|entry| entry.name == "PaymentStrategy")
        .expect("PaymentStrategy contract entry");

    assert_eq!(artifact.summary.scanned_source_count, 3);
    assert_eq!(
        contract.kind,
        AbstractionMemoryEntryKind::InterfaceOrContract
    );
    assert_eq!(contract.provider_node_ids.len(), 2);
    assert!(
        contract
            .provider_node_ids
            .contains(&"StripePay.execute".to_string())
    );
    assert!(
        contract
            .provider_node_ids
            .contains(&"PaypalPay.execute".to_string())
    );
    assert_eq!(
        contract.consumer_node_ids,
        vec!["PaymentRouter.route".to_string()]
    );

    let abstract_function = artifact
        .entries
        .iter()
        .find(|entry| entry.kind == AbstractionMemoryEntryKind::AbstractFunction)
        .expect("abstract function entry");
    assert_eq!(abstract_function.signatures.len(), 1);
}

#[test]
fn search_and_recommend_use_matched_terms_and_reuse_bias() {
    let artifact = AbstractionMemoryArtifact {
        schema_version: "1.0".into(),
        generated_at: "2026-01-01T00:00:00Z".into(),
        project: "test".into(),
        source_map_file: "triad-map.json".into(),
        summary: AbstractionMemorySummary::default(),
        entries: vec![AbstractionMemoryEntry {
            id: "payment_strategy".into(),
            name: "PaymentStrategy".into(),
            kind: AbstractionMemoryEntryKind::InterfaceOrContract,
            primary_source_path: "src/payments/strategies.rs".into(),
            source_paths: vec!["src/payments/strategies.rs".into()],
            node_ids: Vec::new(),
            provider_node_ids: vec!["StripePay.execute".into(), "PaypalPay.execute".into()],
            consumer_node_ids: vec!["PaymentRouter.route".into()],
            variant_clusters: vec!["payment".into()],
            related_abstractions: Vec::new(),
            signatures: vec!["PaymentStrategy.execute(PayInput) -> PayResult".into()],
            tags: vec!["implements_contract".into()],
            abstraction_ratio: 0.6,
            reusability_score: 0.85,
            why_reusable: "Has 2 providers and 1 consumer".into(),
        }],
    };

    let results = search_abstraction_memory(&artifact, "payment strategy", 5);
    assert_eq!(results.len(), 1);
    assert!(results[0].matched_terms.contains(&"payment".to_string()));

    let recommendations = recommend_abstractions(
        &artifact,
        &RecommendationInput {
            query: "payment strategy reuse".into(),
            focus_node_id: Some("PaymentRouter.route".into()),
            focus_source_path: Some("src/payments/router.rs".into()),
            limit: 3,
        },
    );
    assert_eq!(recommendations.len(), 1);
    assert_eq!(
        recommendations[0].suggested_usage,
        SuggestedUsage::ReuseFirst
    );
    assert!(!recommendations[0].rationale.is_empty());
}

#[test]
fn prompt_context_falls_back_cleanly_when_memory_is_missing() {
    let missing =
        std::env::temp_dir().join(format!("missing_abstraction_memory_{}", std::process::id()));
    let context = build_prompt_context(
        std::path::Path::new("triad-map.json"),
        &missing,
        &RecommendationInput {
            query: "payment strategy".into(),
            focus_node_id: None,
            focus_source_path: None,
            limit: 3,
        },
        &AbstractionMemoryConfig::default(),
    );

    assert!(context.matches.is_empty());
    assert!(!context.summary_lines.is_empty());
}

#[test]
fn tokenize_and_contract_name_extraction_filter_noise() {
    let tokens = tokenize("Payment strategy reuse pattern");
    assert!(tokens.contains(&"payment".to_string()));
    assert!(tokens.contains(&"strategy".to_string()));

    let names = extract_contract_names(&[
        "PaymentStrategy".into(),
        "Option<String>".into(),
        "UserService".into(),
        "Vec<Result<String>>".into(),
        "i32".into(),
    ]);
    assert!(names.contains(&"PaymentStrategy".to_string()));
    assert!(names.contains(&"UserService".to_string()));
    assert!(!names.contains(&"Option".to_string()));
    assert!(!names.contains(&"Vec".to_string()));
}
