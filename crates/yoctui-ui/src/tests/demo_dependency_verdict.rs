use super::*;
use crate::dependency_render::dependency_why_built;

fn chain_app(length: usize) -> App {
    let root = DependencyNodeId::recipe("node-0");
    let nodes = (1..=length)
        .map(|index| yoctui_model::DependencyNode {
            id: DependencyNodeId::recipe(format!("node-{index}")),
            provider: Some(format!("/layers/{}/recipe.bb", "long-source/".repeat(40)).into()),
            log: Some(format!("/build/{}/log.do_compile", "long-build/".repeat(40)).into()),
        })
        .collect();
    let edges = (1..=length)
        .map(|index| yoctui_model::DependencyEdge {
            from: DependencyNodeId::recipe(format!("node-{}", index - 1)),
            to: DependencyNodeId::recipe(format!("node-{index}")),
            kind: DependencyEdgeKind::Build,
        })
        .collect();
    let (graph, _) = DependencyGraph::normalize(root.clone(), nodes, edges, 100, 100);
    let mut app = App::new(10, 1_000);
    app.inspector_visible = true;
    app.screen = Screen::Dependencies;
    app.focus = FocusTarget::Inspector;
    app.dependency_graph_selection = Some(root);
    app.dependency_graph = DependencyGraphState::Partial {
        graph,
        limitations: vec!["runtime edges unavailable".into()],
    };
    app
}

#[test]
fn demo_dependency_verdict_precedes_long_metadata_and_preserves_exact_details() {
    let mut app = chain_app(2);
    app.dependency_graph_selection = Some(DependencyNodeId::recipe("node-2"));
    let details = dependency_inspector(&app);
    let expected = "Why built:\nnode-0\n  --build--> node-1\n  --build--> node-2";
    assert!(details.contains(expected));
    assert!(details.find(expected).unwrap() < details.find("Provider:").unwrap());
    assert!(details.contains(&format!("/layers/{}/recipe.bb", "long-source/".repeat(40))));
    assert!(details.contains(&format!(
        "/build/{}/log.do_compile",
        "long-build/".repeat(40)
    )));
    assert!(details.contains("Reverse / incoming:\nbuild: node-1"));
    assert!(details.contains("Dependencies / outgoing:\nnone reported"));
    assert!(details.contains("Limitations:\n- runtime edges unavailable"));
    assert!(details.contains("Alt+l opens task log"));
    assert!(!details.contains(" L opens"));
    for width in [80, 100, 160] {
        let text = rendered_text(&app, width, 24);
        assert!(text.contains("Why built:"), "width {width}");
        assert!(text.contains("--build--> node-2"), "width {width}");
    }
    assert_eq!(app.focus, FocusTarget::Inspector);
    assert_eq!(
        app.dependency_graph_selection,
        Some(DependencyNodeId::recipe("node-2"))
    );
}

#[test]
fn demo_dependency_verdict_root_unreachable_and_limit_remain_visible_narrow() {
    let mut app = chain_app(66);
    for (selection, verdict) in [
        ("node-0", "root selected"),
        ("node-66", "path limit reached"),
    ] {
        app.dependency_graph_selection = Some(DependencyNodeId::recipe(selection));
        for width in [80, 100, 160] {
            assert!(
                rendered_text(&app, width, 24).contains(verdict),
                "{selection} at {width}"
            );
        }
    }
    // Only exhausted searches may claim unreachable, not the bounded deep graph.
    let mut orphan_app = chain_app(2);
    if let DependencyGraphState::Partial { graph, .. } = &mut orphan_app.dependency_graph {
        graph.nodes.push(yoctui_model::DependencyNode::identity(
            DependencyNodeId::recipe("orphan"),
        ));
    }
    orphan_app.dependency_graph_selection = Some(DependencyNodeId::recipe("orphan"));
    for width in [80, 100, 160] {
        assert!(
            rendered_text(&orphan_app, width, 24).contains("unreachable from root"),
            "width {width}"
        );
    }
    // Long successful paths retain all ordered identities without lifting quotas.
    app.dependency_graph_selection = Some(DependencyNodeId::recipe("node-64"));
    let DependencyGraphState::Partial { graph, .. } = &app.dependency_graph else {
        panic!()
    };
    let path = dependency_why_built(graph, app.dependency_graph_selection.as_ref().unwrap());
    assert!(path.starts_with("node-0\n  --build--> node-1"));
    assert!(path.ends_with("--build--> node-64"));
    assert_eq!(path.lines().count(), 65);
    assert_eq!(
        graph.why_built(&DependencyNodeId::recipe("node-66"), 64, 4_096),
        DependencyPathResult::LimitReached
    );
    assert!(dependency_inspector(&app).contains(&path));
}
