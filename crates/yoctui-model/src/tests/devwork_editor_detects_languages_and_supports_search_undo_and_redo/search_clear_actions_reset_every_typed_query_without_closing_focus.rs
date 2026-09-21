use super::*;

#[test]
fn search_clear_actions_reset_every_typed_query_without_closing_focus() {
    let mut app = App::new(10, 1_000);
    app.command_palette_open = true;
    app.command_palette_query = "palette".into();
    app.compatibility_ui.query = "capability".into();
    app.image_artifact_query = "image".into();
    app.sdk_artifact_query = "sdk".into();
    app.test_result_query = "test".into();
    app.logs.query = "log".into();
    app.package_query = "package".into();
    app.metadata_query = "metadata".into();
    app.security.query = "security".into();
    app.qa.query = "qa".into();

    for action in [
        Action::ClearCommandPaletteQuery,
        Action::ClearCompatibilityQuery,
        Action::ClearImageArtifactQuery,
        Action::ClearSdkArtifactQuery,
        Action::ClearTestResultQuery,
        Action::ClearLogQuery,
        Action::ClearPackageQuery,
        Action::ClearMetadataQuery,
        Action::Security(SecurityAction::ClearQuery),
        Action::Qa(QaAction::ClearQuery),
    ] {
        let _ = update(&mut app, action);
    }

    assert!(app.command_palette_query.is_empty());
    assert!(app.compatibility_ui.query.is_empty());
    assert!(app.image_artifact_query.is_empty());
    assert!(app.sdk_artifact_query.is_empty());
    assert!(app.test_result_query.is_empty());
    assert!(app.logs.query.is_empty());
    assert!(app.package_query.is_empty());
    assert!(app.metadata_query.is_empty());
    assert!(app.security.query.is_empty());
    assert!(app.qa.query.is_empty());
    assert!(app.command_palette_open);
}
