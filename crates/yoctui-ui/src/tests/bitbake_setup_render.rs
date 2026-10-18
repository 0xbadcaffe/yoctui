use super::*;
use ratatui::{Terminal, backend::TestBackend};

#[test]
fn setup_wizard_and_review_fit_small_terminals() {
    let app = App::new(20, 2000);
    let mut wizard = BitBakeSetupWizard {
        values: ["bitbake-setup", "/tmp/bitbake-builds", "demo", "", ""].map(str::to_owned),
        field: 0,
        editor: None,
        root_only: false,
        review: false,
        preparing: None,
        error: None,
    };
    for (width, height) in [(80, 24), (100, 30), (160, 48)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| super::wizard(frame, &app, &wizard, frame.area()))
            .unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(text.contains("Registry"));
        assert!(text.contains("s review/save"));
        wizard.review = true;
        terminal
            .draw(|frame| super::wizard(frame, &app, &wizard, frame.area()))
            .unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(text.contains("Enter confirm"));
        assert!(text.contains("No image build"));
        wizard.review = false;
    }
}

#[test]
fn setup_catalogue_keeps_final_selection_visible() {
    let mut app = App::new(20, 2000);
    app.bitbake_setup.entries = (0..200)
        .map(|i| yoctui_model::BitBakeSetupEntry {
            name: format!("setup-{i:03}"),
            profile: yoctui_model::BuildEnvironmentProfile {
                source_dir: "/src".into(),
                build_dir: format!("/build/{i}").into(),
                init_script: "/src/oe-init-build-env".into(),
            },
        })
        .collect();
    app.bitbake_setup.selected = 199;
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal
        .draw(|frame| catalogue(frame, &app, frame.area()))
        .unwrap();
    let text: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect();
    assert!(text.contains("> setup-199"));
    assert!(text.contains("r refresh"));
}
