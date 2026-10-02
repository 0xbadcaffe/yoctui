use super::*;

fn editor_app() -> App {
    let mut app = App::new(16, 4_096);
    app.screen = Screen::Layers;
    app.gitui_program = Some("/usr/bin/gitui".into());
    update(
        &mut app,
        Action::OpenRecipeEditor {
            recipe: "Layer: meta-local".into(),
            root: "/layers/meta-local".into(),
            files: vec!["recipe.bb".into()],
        },
    );
    update(&mut app, Action::LoadRecipeEditorContent("original".into()));
    update(
        &mut app,
        Action::FocusRecipeEditor(RecipeEditorFocus::Document),
    );
    update(&mut app, Action::ToggleRecipeEditorEditing);
    update(
        &mut app,
        Action::EditRecipeEditor(PopupEditorCommand::Insert('!')),
    );
    app
}

fn complete(app: &mut App, generation: u64, result: SourceGitStatus) {
    update(
        app,
        Action::RecipeEditorGitUiInspected {
            generation,
            root: "/layers/meta-local".into(),
            result,
        },
    );
}

#[test]
fn editor_gitui_uses_local_context_without_devtool_or_global_git_authority() {
    let mut app = editor_app();
    app.workspace.source_dir = Some("/unrelated/source".into());
    let editor = app.active_dialog().cloned().unwrap();
    let git = app
        .command_palette_commands()
        .into_iter()
        .find(|item| item.id == CommandId::OpenGitUi)
        .unwrap();
    assert_eq!(git.disabled_reason, None);
    assert_eq!(
        command_action(&app, CommandId::OpenGitUi),
        Action::OpenRecipeEditorGitUi
    );
    assert!(
        matches!(update(&mut app, Action::OpenGitUi), Some(Effect::InspectRecipeEditorGitUi { ref root, ref cwd, .. })
        if root == &PathBuf::from("/layers/meta-local") && cwd == root)
    );
    let generation = app.editor_gitui_generation;
    complete(
        &mut app,
        generation,
        SourceGitStatus::Ready(SourceGitSummary::default()),
    );
    assert!(
        matches!(app.active_dialog(), Some(Dialog::TerminalLaunch(dialog)) if dialog.request.cwd == std::path::Path::new("/layers/meta-local"))
    );
    update(&mut app, Action::CancelTerminalLaunch);
    assert_eq!(app.active_dialog(), Some(&editor));
    assert_eq!(app.focus, FocusTarget::Dialog);
}

#[test]
fn editor_gitui_failure_and_stale_results_preserve_document_and_dialogs() {
    let mut app = editor_app();
    let editor = app.active_dialog().cloned().unwrap();
    app.gitui_program = None;
    assert_eq!(update(&mut app, Action::OpenRecipeEditorGitUi), None);
    assert!(
        app.notification
            .as_deref()
            .unwrap()
            .contains("Install GitUI")
    );
    app.gitui_program = Some("/usr/bin/gitui".into());
    update(&mut app, Action::OpenRecipeEditorGitUi);
    let stale = app.editor_gitui_generation;
    update(&mut app, Action::OpenRecipeEditorGitUi);
    complete(
        &mut app,
        stale,
        SourceGitStatus::Ready(SourceGitSummary::default()),
    );
    assert_eq!(app.active_dialog(), Some(&editor));
    assert!(app.editor_gitui_pending.is_some());
    let current = app.editor_gitui_generation;
    complete(
        &mut app,
        current,
        SourceGitStatus::Unavailable("not a readable Git worktree".into()),
    );
    assert!(
        app.notification
            .as_deref()
            .unwrap()
            .contains("not a readable Git")
    );
    assert_eq!(app.active_dialog(), Some(&editor));
    update(&mut app, Action::OpenRecipeEditorGitUi);
    let covered = app.editor_gitui_generation;
    app.dialogs.push_front(Dialog::QuitConfirmation);
    complete(
        &mut app,
        covered,
        SourceGitStatus::Ready(SourceGitSummary::default()),
    );
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::QuitConfirmation)
    ));
    assert!(app.editor_gitui_pending.is_none());
}

#[test]
fn editor_gitui_embedded_handoff_restores_exact_dirty_editor_without_trapping_pty() {
    let mut app = editor_app();
    let editor = app.active_dialog().cloned().unwrap();
    update(&mut app, Action::OpenRecipeEditorGitUi);
    let generation = app.editor_gitui_generation;
    complete(
        &mut app,
        generation,
        SourceGitStatus::Ready(SourceGitSummary::default()),
    );
    let effect = update(&mut app, Action::ConfirmTerminalLaunch);
    assert!(matches!(
        effect,
        Some(Effect::Terminal(TerminalEffect::Create {
            kind: TerminalCreationKind::GitUi,
            ..
        }))
    ));
    assert_eq!(app.screen, Screen::TerminalSessions);
    assert!(app.active_dialog().is_none());
    assert!(app.suspended_recipe_editor.is_some());
    update(&mut app, Action::RestoreRecipeEditor);
    assert_eq!(app.screen, Screen::Layers);
    assert_eq!(app.active_dialog(), Some(&editor));
    assert_eq!(app.suspended_recipe_editor, None);
}

#[test]
fn editor_gitui_offline_and_detached_keep_editor_in_place() {
    for detached in [false, true] {
        let mut app = editor_app();
        app.require_daemon = true;
        let editor = app.active_dialog().cloned().unwrap();
        update(&mut app, Action::OpenRecipeEditorGitUi);
        let generation = app.editor_gitui_generation;
        complete(
            &mut app,
            generation,
            SourceGitStatus::Ready(SourceGitSummary::default()),
        );
        if detached && let Some(Dialog::TerminalLaunch(dialog)) = app.active_dialog_mut() {
            dialog.destination = TerminalLaunchDestination::Detached;
        }
        let effect = update(&mut app, Action::ConfirmTerminalLaunch);
        if detached {
            assert!(matches!(effect, Some(Effect::LaunchDetachedTerminal(_))));
        } else {
            assert!(matches!(effect, Some(Effect::Terminal(_))));
        }
        assert_eq!(app.active_dialog(), Some(&editor));
        assert!(app.suspended_recipe_editor.is_none());
    }
}

#[test]
fn editor_gitui_cannot_replace_retained_dirty_editor_or_accept_relative_root() {
    let mut app = editor_app();
    suspend(&mut app);
    let retained = app.suspended_recipe_editor.clone();
    update(
        &mut app,
        Action::OpenRecipeEditor {
            recipe: "other".into(),
            root: "/other".into(),
            files: vec![],
        },
    );
    assert!(
        app.notification
            .as_deref()
            .unwrap()
            .contains("Save the retained editor")
    );
    assert_eq!(
        app.active_dialog(),
        Some(&Dialog::RecipeEditor(*retained.unwrap().1))
    );
    if let Some(Dialog::RecipeEditor(editor)) = app.active_dialog_mut() {
        editor.root = "relative".into();
    }
    assert_eq!(update(&mut app, Action::OpenRecipeEditorGitUi), None);
    assert!(app.notification.as_deref().unwrap().contains("absolute"));
}

#[test]
fn editor_gitui_generated_rootfs_does_not_launch_in_its_host_repository() {
    let mut app = editor_app();
    app.layer_browser = Some(LayerBrowser::new(
        "Rootfs: image".into(),
        "/layers/meta-local".into(),
    ));
    assert_eq!(update(&mut app, Action::OpenRecipeEditorGitUi), None);
    assert!(
        app.notification
            .as_deref()
            .unwrap()
            .contains("Generated RootFS")
    );
    let item = app
        .command_palette_commands()
        .into_iter()
        .find(|item| item.id == CommandId::OpenGitUi)
        .unwrap();
    assert!(item.disabled_reason.unwrap().contains("Generated RootFS"));
    assert!(app.editor_gitui_pending.is_none());
}
