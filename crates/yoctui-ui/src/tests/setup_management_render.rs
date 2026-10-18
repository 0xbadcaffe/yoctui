use super::*;
use ratatui::{Terminal, backend::TestBackend};
#[test]
fn setup_review_exposes_safe_policies_and_controls_at_small_sizes() {
    let app = App::new(20, 2000);
    let d = SetupOperationDialog {
        operation: SetupOperation::Update,
        profile: yoctui_model::BuildEnvironmentProfile {
            source_dir: "/tmp/setups/demo/layers/core".into(),
            build_dir: "/tmp/setups/demo/build".into(),
            init_script: "/tmp/setups/demo/build/init-build-env".into(),
        },
        tool: "bitbake-setup".into(),
        fragment: String::new(),
        editor: None,
        config_policy: 0,
        backup_conflicts: false,
        offline: false,
        preparing: None,
        error: None,
    };
    for (width, height) in [(80, 24), (100, 30), (160, 48)] {
        let mut t = Terminal::new(TestBackend::new(width, height)).unwrap();
        t.draw(|f| review(f, &app, &d, f.area())).unwrap();
        let text: String = t
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(text.contains("prompt"));
        assert!(text.contains("abort (default)"));
        assert!(text.contains("Enter confirm"));
    }
}

#[test]
fn setup_fragments_keep_last_selection_and_output_end_visible() {
    let mut app = App::new(20, 2000);
    app.bitbake_setup.management.fragments = (0..300)
        .map(|i| yoctui_model::SetupFragment {
            name: format!("core/test/{i:03}"),
            summary: "Test".into(),
            enabled: i % 2 == 0,
            pattern: false,
        })
        .collect();
    app.bitbake_setup.management.selected = 299;
    let mut t = Terminal::new(TestBackend::new(80, 24)).unwrap();
    t.draw(|f| workspace(f, &app, f.area())).unwrap();
    let text: String = t
        .backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect();
    assert!(text.contains("core/test/299"));
    app.bitbake_setup.management.output = Some((0..300).map(|i| format!("output {i}")).collect());
    app.bitbake_setup.management.output_scroll = 299;
    t.draw(|f| workspace(f, &app, f.area())).unwrap();
    let text: String = t
        .backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect();
    assert!(text.contains("output 299"));
}
