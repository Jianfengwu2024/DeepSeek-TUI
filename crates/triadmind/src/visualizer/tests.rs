#[cfg(test)]
mod tests {
    use crate::protocol::TriadNodeDefinition;
    use crate::visualizer::graph::{build_knowledge_graph, categorize_node, is_generic_type};
    use crate::visualizer::render::render_html;
    use crate::visualizer::types::VisualizerOptions;

    fn make_node(id: &str, problem: &str, demand: &[&str], answer: &[&str]) -> TriadNodeDefinition {
        TriadNodeDefinition {
            node_id: id.into(),
            category: Some("core".into()),
            source_path: Some("src/main.rs".into()),
            lifecycle: None,
            fission: Some(crate::protocol::TriadFission {
                problem: problem.into(),
                demand: demand.iter().map(|s| s.to_string()).collect(),
                answer: answer.iter().map(|s| s.to_string()).collect(),
            }),
        }
    }

    #[test]
    fn test_build_knowledge_graph() {
        let nodes = vec![
            make_node(
                "Handler.run",
                "handle request",
                &["Service.process"],
                &["Response"],
            ),
            make_node("Service.process", "process data", &["String"], &["Result"]),
        ];
        let opts = VisualizerOptions::default();
        let graph = build_knowledge_graph(&nodes, &opts);
        assert_eq!(graph.stats.nodes, 2);
        assert!(graph.edges.len() >= 1);
    }

    #[test]
    fn test_categorize_node() {
        assert_eq!(categorize_node("HttpHandler.run", "handle"), "handler");
        assert_eq!(categorize_node("UserService.create", "service"), "service");
        assert_eq!(
            categorize_node("DatabaseAdapter.connect", "adapter"),
            "adapter"
        );
        assert_eq!(categorize_node("Core.engine", "core engine"), "core");
        assert_eq!(categorize_node("Utils.helper", "helper"), "other");
    }

    #[test]
    fn test_is_generic_type() {
        assert!(is_generic_type("String"));
        assert!(is_generic_type("i32"));
        assert!(is_generic_type("bool"));
        assert!(!is_generic_type("UserService"));
        assert!(!is_generic_type("MyCustomType"));
    }

    #[test]
    fn test_render_html_contains_graph_data() {
        let nodes = vec![make_node("Test.run", "test", &[], &[])];
        let mut opts = VisualizerOptions::default();
        opts.show_isolated_capabilities = true;
        let graph = build_knowledge_graph(&nodes, &opts);
        let html = render_html(&graph, &opts);
        assert!(html.contains("TriadMind"));
        assert!(html.contains("Test.run"));
        assert!(html.contains("vis-network"));
    }
}
