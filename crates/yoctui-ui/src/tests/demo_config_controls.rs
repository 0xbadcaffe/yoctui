use super::*;

fn config_app() -> App {
    let mut app = App::new(32, 8192);
    app.screen = Screen::Configuration;
    app.focus = FocusTarget::Workspace;
    app.workspace
        .variables
        .insert("MACHINE".into(), "romulus".into());
    app
}

fn assert_control_summary(app: &App, effective: &str, unexpanded: &str, detail_state: &str) {
    for (width, height) in [(80, 24), (90, 24), (100, 28), (140, 32), (160, 50)] {
        let output = rendered_text_at(app, width, height, literal_now());
        for anchor in [
            effective,
            unexpanded,
            "Alt+e edit:",
            "c compare:",
            "o source:",
            "s scope:",
            detail_state,
        ] {
            assert!(
                output.contains(anchor),
                "{width}x{height} lost {anchor}: {output}"
            );
        }
        assert!(!output.contains("C effective:"), "{output}");
        assert!(!output.contains("U unexpanded:"), "{output}");
        assert!(!output.contains("E edit:"), "{output}");
    }
}

#[test]
fn config_controls_prioritize_exact_missing_loading_and_failed_states() {
    let mut app = config_app();
    assert_control_summary(
        &app,
        "Alt+c effective: disabled",
        "Alt+u unexpanded: disabled",
        "Detail not loaded",
    );
    let identity = yoctui_model::VariableIdentity {
        name: "MACHINE".into(),
        recipe: None,
    };
    app.variable_detail_loading.insert(identity.clone());
    assert_control_summary(
        &app,
        "Alt+c effective: disabled",
        "Alt+u unexpanded: disabled",
        "Loading authoritative detail",
    );
    app.variable_detail_loading.clear();
    app.variable_detail_errors
        .insert(identity, "native metadata unavailable".into());
    assert_control_summary(
        &app,
        "Alt+c effective: disabled",
        "Alt+u unexpanded: disabled",
        "Detail unavailable: native metadata",
    );
    let wide = rendered_text_at(&app, 240, 70, literal_now());
    assert!(wide.contains("native metadata unavailable"), "{wide}");
}

#[test]
fn config_loaded_partial_copy_availability_remains_truthful_with_long_details() {
    let mut app = config_app();
    let identity = yoctui_model::VariableIdentity {
        name: "MACHINE".into(),
        recipe: None,
    };
    app.variable_details.insert(
        identity.clone(),
        yoctui_model::VariableDetail {
            identity,
            effective_value: Some("romulus".into()),
            unexpanded_value: None,
            provenance: Some(format!("/source/{}/machine.conf", "nested/".repeat(60))),
            operations: Vec::new(),
            active_overrides: Vec::new(),
        },
    );
    assert_control_summary(
        &app,
        "Alt+c effective: enabled",
        "Alt+u unexpanded: disabled",
        "authoritative detail loaded",
    );
    let detailed = crate::config_render::config_inspector(&app);
    assert!(detailed.contains("Effective value: romulus"));
    assert!(detailed.contains("Unexpanded value: unavailable"));
    assert!(detailed.contains("/machine.conf"));
    assert!(detailed.contains("Alt+u unexpanded: disabled"));
    assert!(
        detailed.contains("The unexpanded value for MACHINE is unavailable."),
        "{detailed}"
    );
}
