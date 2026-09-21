use super::*;

#[test]
fn compatibility_ui_model_wire_lifecycle_reconciles_filter_search_and_selection() {
    let first = compatibility_workspace_authority(1).normalize().unwrap();
    let mut app = yoctui_model::App::new(16, 4096);
    let mut client = DaemonClientSnapshot::default();
    client.replace_app(&mut app, compatibility_workspace_daemon_snapshot(&first));

    let projection = app
        .compatibility_ui
        .project(&app.workspace_compatibility, app.daemon.status);
    assert_eq!(projection.total_capabilities, 4);
    assert_eq!(projection.summary.available, 3);
    assert_eq!(projection.summary.unavailable, 1);
    let _ = compatibility_workspace_action(
        &mut app,
        yoctui_model::Action::SetCompatibilityFilter(
            yoctui_model::CompatibilityUiFilter::Unavailable,
        ),
    );
    assert_eq!(
        app.compatibility_ui.selected(),
        Some(yoctui_model::CapabilityId::DevtoolUpgrade)
    );
    let _ =
        compatibility_workspace_action(&mut app, yoctui_model::Action::BeginCompatibilitySearch);
    for character in "upgrade".chars() {
        let _ = compatibility_workspace_action(
            &mut app,
            yoctui_model::Action::AppendCompatibilityQuery(character),
        );
    }

    let second = compatibility_workspace_authority(2).normalize().unwrap();
    client.replace_app(&mut app, compatibility_workspace_daemon_snapshot(&second));
    assert_eq!(
        app.compatibility_ui.selected(),
        Some(yoctui_model::CapabilityId::DevtoolUpgrade)
    );
    assert_eq!(app.compatibility_ui.query, "upgrade");

    client.disconnect_app(&mut app);
    let projection = app
        .compatibility_ui
        .project(&app.workspace_compatibility, app.daemon.status);
    assert!(matches!(
        projection.authority,
        yoctui_model::CompatibilityUiAuthorityStatus::Unavailable { .. }
    ));
    assert!(projection.rows.is_empty());
    assert_eq!(app.compatibility_ui.selected(), None);
    assert_eq!(
        app.compatibility_ui.filter,
        yoctui_model::CompatibilityUiFilter::Unavailable
    );
    assert_eq!(app.compatibility_ui.query, "upgrade");
}
