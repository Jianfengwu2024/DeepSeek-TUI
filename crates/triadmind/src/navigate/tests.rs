#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::navigate::engine::{build_navigator_prompt, run_navigator};
    use crate::navigate::types::NavigatorRunOptions;
    use crate::protocol::{TriadNodeDefinition, UpgradeProtocol};

    #[test]
    fn test_navigator_rejects_empty_demand() {
        let tmp = std::env::temp_dir().join("triadmind_nav_test");
        let _ = std::fs::create_dir_all(&tmp);
        let opts = NavigatorRunOptions::default();
        let result = run_navigator(&tmp, "", &opts);
        let _ = std::fs::remove_dir_all(&tmp);
        assert!(result.is_err());
    }

    #[test]
    fn test_navigator_pending_without_protocol() {
        let tmp = std::env::temp_dir().join("triadmind_nav_pending");
        let _ = std::fs::create_dir_all(tmp.join(".triadmind"));
        let opts = NavigatorRunOptions::default();
        let result = run_navigator(&tmp, "add login feature", &opts);
        let _ = std::fs::remove_dir_all(&tmp);
        assert!(result.is_ok());
        let r = result.unwrap();
        assert_eq!(r.status, "pending_protocol");
        assert!(r.summary.iter().any(|s| s.contains("template")));
    }

    #[test]
    fn test_build_navigator_prompt() {
        let nodes: Vec<TriadNodeDefinition> = vec![];
        let prompt = build_navigator_prompt(
            Path::new("/test"),
            "add feature X",
            &nodes,
            None,
        );
        assert!(prompt.contains("add feature X"));
        assert!(prompt.contains("Navigator"));
        assert!(prompt.contains("UpgradeProtocol"));
    }

    #[test]
    fn test_navigator_with_existing_protocol() {
        let tmp = std::env::temp_dir().join("triadmind_nav_ready");
        let _ = std::fs::create_dir_all(tmp.join(".triadmind"));

        let protocol = UpgradeProtocol {
            protocol_version: "1.0".into(),
            project: "test".into(),
            map_source: "triad-map.json".into(),
            user_demand: "test feature".into(),
            upgrade_policy: crate::protocol::UpgradePolicy {
                allowed_ops: vec!["reuse".into()],
                principle: "reuse_first".into(),
            },
            macro_split: None,
            meso_split: None,
            micro_split: None,
            actions: vec![],
        };
        let protocol_path = tmp.join(".triadmind").join("impact-protocol.json");
        std::fs::write(
            &protocol_path,
            serde_json::to_string_pretty(&protocol).unwrap(),
        )
        .unwrap();

        let opts = NavigatorRunOptions {
            protocol_path: Some(protocol_path),
            llm: None,
        };
        let result = run_navigator(&tmp, "test feature", &opts);
        let _ = std::fs::remove_dir_all(&tmp);
        assert!(result.is_ok());
        let r = result.unwrap();
        assert_eq!(r.status, "ready");
        assert!(r.summary.iter().any(|s| s.contains("Impact map generated")));
    }
}
