#[cfg(test)]
mod tests {
    use crate::generator::engine::apply_protocol;
    use crate::generator::scaffold::{generate_rust_function, generate_ts_function};
    use crate::generator::types::GeneratorOptions;
    use crate::protocol::{ProtocolAction, TriadNodeDefinition, TriadOp, UpgradeProtocol};

    #[test]
    fn test_apply_protocol_empty() {
        let protocol = UpgradeProtocol {
            protocol_version: "1.0".into(),
            project: "test".into(),
            map_source: "triad-map.json".into(),
            user_demand: "test".into(),
            upgrade_policy: crate::protocol::UpgradePolicy {
                allowed_ops: vec!["create_child".into()],
                principle: "reuse_first".into(),
            },
            macro_split: None,
            meso_split: None,
            micro_split: None,
            actions: vec![],
        };
        let tmp = std::env::temp_dir().join("triadmind_gen_empty");
        let _ = std::fs::create_dir_all(&tmp);
        let opts = GeneratorOptions::default();
        let result = apply_protocol(&tmp, &protocol, &opts);
        let _ = std::fs::remove_dir_all(&tmp);
        assert!(result.is_ok());
        let r = result.unwrap();
        assert_eq!(r.files.len(), 0);
    }

    #[test]
    fn test_generate_rust_function() {
        let fission = crate::protocol::TriadFission {
            problem: "Process incoming data".into(),
            demand: vec!["String".into(), "Config".into()],
            answer: vec!["Result".into()],
        };
        let code = generate_rust_function("Service.process", &fission);
        assert!(code.contains("/// Process incoming data"));
        assert!(code.contains("pub fn process"));
        assert!(code.contains("arg0: String"));
        assert!(code.contains("arg1: Config"));
        assert!(code.contains("-> Result"));
    }

    #[test]
    fn test_generate_ts_function() {
        let fission = crate::protocol::TriadFission {
            problem: "Handle user login".into(),
            demand: vec!["string".into(), "string".into()],
            answer: vec!["boolean".into()],
        };
        let code = generate_ts_function("AuthService.login", &fission);
        assert!(code.contains("* Handle user login"));
        assert!(code.contains("export function login"));
        assert!(code.contains("arg0: string"));
        assert!(code.contains(": boolean"));
    }

    #[test]
    fn test_apply_protocol_with_actions() {
        let protocol = UpgradeProtocol {
            protocol_version: "1.0".into(),
            project: "test".into(),
            map_source: "triad-map.json".into(),
            user_demand: "test".into(),
            upgrade_policy: crate::protocol::UpgradePolicy {
                allowed_ops: vec!["create_child".into()],
                principle: "reuse_first".into(),
            },
            macro_split: None,
            meso_split: None,
            micro_split: None,
            actions: vec![
                ProtocolAction {
                    op: TriadOp::Reuse,
                    node_id: Some("Existing.run".into()),
                    parent_node_id: None,
                    node: None,
                    fission: None,
                    reuse: vec![],
                    reason: None,
                    confidence: None,
                },
                ProtocolAction {
                    op: TriadOp::CreateChild,
                    node_id: None,
                    parent_node_id: Some("Existing.run".into()),
                    node: Some(TriadNodeDefinition {
                        node_id: "NewFeature.process".into(),
                        category: Some("core".into()),
                        source_path: None,
                        lifecycle: None,
                        fission: Some(crate::protocol::TriadFission {
                            problem: "New feature".into(),
                            demand: vec!["Input".into()],
                            answer: vec!["Output".into()],
                        }),
                    }),
                    fission: None,
                    reuse: vec![],
                    reason: None,
                    confidence: None,
                },
            ],
        };

        let tmp = std::env::temp_dir().join("triadmind_gen_actions");
        let _ = std::fs::create_dir_all(&tmp);
        let opts = GeneratorOptions {
            write_files: false,
            ..Default::default()
        };
        let result = apply_protocol(&tmp, &protocol, &opts);
        let _ = std::fs::remove_dir_all(&tmp);
        assert!(result.is_ok());
        let r = result.unwrap();
        assert_eq!(r.reuse_count, 1);
        assert_eq!(r.create_count, 1);
        assert_eq!(r.files.len(), 1);
    }
}