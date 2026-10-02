use super::*;

fn detail_app(operations: Vec<yoctui_model::VariableOperation>) -> App {
    let mut app = App::new(32, 8192);
    app.screen = Screen::Configuration;
    app.focus = FocusTarget::Workspace;
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemuarm".into());
    let identity = yoctui_model::VariableIdentity {
        name: "MACHINE".into(),
        recipe: None,
    };
    app.variable_details.insert(
        identity.clone(),
        yoctui_model::VariableDetail {
            identity,
            effective_value: Some("qemuarm".into()),
            unexpanded_value: None,
            provenance: Some("conf/local.conf:12".into()),
            operations,
            active_overrides: Vec::new(),
        },
    );
    app
}

#[test]
fn demo_config_details_controls_and_exact_provenance_remain_visible_together() {
    let app = detail_app(vec![
        yoctui_model::VariableOperation {
            operation: "set".into(),
            file: Some("meta/conf/bitbake.conf".into()),
            line: Some(1),
            value: Some("qemux86-64".into()),
        },
        yoctui_model::VariableOperation {
            operation: "set".into(),
            file: Some("conf/local.conf".into()),
            line: Some(12),
            value: Some("qemuarm".into()),
        },
    ]);
    for (width, height) in [(100, 25), (110, 26), (160, 30), (200, 50)] {
        let output = rendered_text(&app, width, height);
        for expected in [
            "Alt+c effective: enabled",
            "Alt+u unexpanded: disabled",
            "conf/local.conf:12",
            "meta/conf/bitbake.conf:1",
            "Operations:",
        ] {
            assert!(
                output.contains(expected),
                "{width}x{height} lost {expected}: {output}"
            );
        }
    }
    let full = crate::config_render::config_inspector(&app);
    for expected in [
        "Variable: MACHINE",
        "Scope: global",
        "Effective value: qemuarm",
        "Unexpanded value: unavailable",
        "Active overrides: none reported",
        "set @ conf/local.conf:12 = qemuarm",
    ] {
        assert!(full.contains(expected), "{full}");
    }
    assert_eq!(app.focus, FocusTarget::Workspace);
}

#[test]
fn demo_config_details_empty_operations_and_global_only_scope_are_truthful() {
    let app = detail_app(Vec::new());
    for (width, height) in [(110, 26), (160, 30), (200, 50)] {
        let output = rendered_text(&app, width, height);
        for expected in [
            "Unexpanded value: unavailable",
            "Operations: none reported",
            "s scope: global only",
            "Alt+c effective: enabled",
        ] {
            assert!(
                output.contains(expected),
                "{width}x{height} lost {expected}: {output}"
            );
        }
        assert!(!output.contains("s scope: enabled"), "{output}");
    }
}
