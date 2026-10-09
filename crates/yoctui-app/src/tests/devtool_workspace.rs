use super::*;

#[test]
fn devtool_recipe_selection_survives_reordered_daemon_snapshot() {
    use yoctui_protocol::daemon::DaemonBuildEvent;
    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([9; 16]),
        123,
        "boot-id".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let mut snapshot = daemon_protocol_snapshot(&state);
    let data = yoctui_protocol::WorkspaceData {
        build_dir: Some("/build".into()),
        source_dir: Some("/source".into()),
        variables: Default::default(),
        variable_provenance: Default::default(),
        variable_provenance_chain: Default::default(),
        bitbake_version: None,
        release: None,
        layers: Vec::new(),
        recipes: ["selected", "other"]
            .into_iter()
            .map(|name| yoctui_protocol::RecipeData {
                name: name.into(),
                file: Some(format!("/layer/{name}.bb")),
                version: None,
                layer: None,
                preferred_version: None,
                append_count: None,
            })
            .collect(),
    };
    snapshot
        .build_events
        .push(DaemonBuildEvent::Workspace { data });
    let mut app = yoctui_model::App::new(16, 4096);
    app.screen = yoctui_model::Screen::Devtool;
    app.metadata_query = "selected".into();
    app.workspace.recipes = vec![
        yoctui_model::Recipe {
            name: "other".into(),
            file: Some("/layer/other.bb".into()),
            ..Default::default()
        },
        yoctui_model::Recipe {
            name: "selected".into(),
            file: Some("/layer/selected.bb".into()),
            ..Default::default()
        },
    ];
    app.recipe_selection = 1;
    DaemonClientSnapshot::default().replace_app(&mut app, snapshot);
    assert_eq!(app.workspace.recipes[app.recipe_selection].name, "selected");
    assert_eq!(app.metadata_query, "selected");
    assert_eq!(app.screen, yoctui_model::Screen::Devtool);
}

#[test]
fn devtool_workspace_routes_the_ordered_recipe_development_loop() {
    assert_eq!(
        devtool_workspace_action(false, Input::Enter),
        Some(Action::BeginSelectedRecipeDevtoolStatus)
    );
    assert_eq!(
        devtool_workspace_action(false, Input::Char('d')),
        Some(Action::BeginSelectedRecipeDevtoolModify)
    );
    assert_eq!(
        devtool_workspace_action(false, Input::Char('b')),
        Some(Action::BeginSelectedRecipeBuild)
    );
    assert_eq!(
        devtool_workspace_action(false, Input::Char('P')),
        Some(Action::BeginSelectedRecipeDevtoolDeploy)
    );
    assert_eq!(
        devtool_workspace_action(false, Input::Char('u')),
        Some(Action::BeginSelectedRecipeDevtoolPatch)
    );
    assert_eq!(
        devtool_workspace_action(false, Input::Char('F')),
        Some(Action::BeginSelectedRecipeDevtoolFinish)
    );
    assert_eq!(
        devtool_workspace_action(false, Input::Char('G')),
        Some(Action::BeginSelectedRecipeDevtoolGitUi)
    );
}

#[test]
fn devtool_patch_dialogs_route_only_bounded_selection_preview_and_confirmation() {
    assert_eq!(
        devtool_patch_picker_action(Input::Up),
        Some(Action::SelectDevtoolPatchLayer { delta: -1 })
    );
    assert_eq!(
        devtool_patch_picker_action(Input::Enter),
        Some(Action::PreviewDevtoolPatch)
    );
    assert_eq!(
        devtool_patch_confirmation_action(Input::Enter),
        Some(Action::ConfirmDevtoolPatch)
    );
    assert_eq!(
        devtool_patch_confirmation_action(Input::Esc),
        Some(Action::CancelDevtoolPatchConfirmation)
    );
}

#[test]
fn devtool_workspace_search_keeps_recipe_selection_actions_bounded() {
    assert_eq!(
        devtool_workspace_action(true, Input::Char('b')),
        Some(Action::AppendMetadataQuery('b'))
    );
    assert_eq!(
        devtool_workspace_action(false, Input::PageDown),
        Some(Action::SelectRecipe { delta: 10 })
    );
}

#[test]
fn devtool_recipe_filter_owns_slash_and_text_before_global_commands() {
    let mut app = yoctui_model::App::new(16, 4096);
    app.screen = yoctui_model::Screen::Devtool;
    app.focus = yoctui_model::FocusTarget::Workspace;
    assert_eq!(global_search_action(&app, Input::Char('/')), None);
    assert_eq!(
        devtool_workspace_action(false, Input::Char('/')),
        Some(Action::BeginMetadataSearch)
    );
    assert!(!workspace_text_input_active(&app));
    let _ = yoctui_model::update(&mut app, Action::BeginMetadataSearch);
    assert!(workspace_text_input_active(&app));
    for character in ['b', 'd', '/', 'u'] {
        assert_eq!(
            devtool_workspace_action(true, Input::Char(character)),
            Some(Action::AppendMetadataQuery(character))
        );
    }
    let _ = yoctui_model::update(&mut app, Action::FinishMetadataSearch);
    assert!(!workspace_text_input_active(&app));
}
