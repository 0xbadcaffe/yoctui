use std::path::PathBuf;
use yoctui_model::*;

fn fixture() -> App {
    let mut app = App::new(32, 8192);
    app.workspace.build_dir = Some("/build/poky".into());
    update(
        &mut app,
        Action::Workbench(WorkbenchAction::OpenRecipe {
            recipe: "busybox".into(),
            root: "/source/busybox".into(),
        }),
    );
    app
}
fn send(app: &mut App, action: WorkbenchAction) -> Option<Effect> {
    update(app, Action::Workbench(action))
}
fn open_file(app: &mut App, name: &str, text: &str) {
    app.workbench.file_request += 1;
    send(
        app,
        WorkbenchAction::FileLoaded {
            session: app.workbench.active.unwrap(),
            root: "/source/busybox".into(),
            path: PathBuf::from("/source/busybox").join(name),
            generation: app.workbench.file_request,
            result: Ok(text.into()),
        },
    );
}

#[test]
fn workbench_devtool_handoff_reuses_session_without_dropping_dirty_buffers() {
    let mut app = fixture();
    open_file(&mut app, "Makefile", "all:\n");
    send(
        &mut app,
        WorkbenchAction::Edit(PopupEditorCommand::Insert('#')),
    );
    send(
        &mut app,
        WorkbenchAction::OpenRecipe {
            recipe: "busybox".into(),
            root: "/source/busybox".into(),
        },
    );
    assert_eq!(app.screen, Screen::Workbench);
    assert_eq!(app.workbench.sessions.len(), 1);
    assert!(app.workbench.current().unwrap().tabs[0].is_dirty());
}

#[test]
fn workbench_switching_tabs_and_sessions_retains_undoable_edits() {
    let mut app = fixture();
    open_file(&mut app, "first.bb", "first\n");
    send(
        &mut app,
        WorkbenchAction::Edit(PopupEditorCommand::Insert('x')),
    );
    open_file(&mut app, "second.bb", "second\n");
    send(&mut app, WorkbenchAction::Tab(-1));
    assert_eq!(
        app.workbench.current().unwrap().tabs[0].document.text,
        "xfirst\n"
    );
    send(&mut app, WorkbenchAction::Sessions);
    send(&mut app, WorkbenchAction::Open);
    send(&mut app, WorkbenchAction::Edit(PopupEditorCommand::Undo));
    assert_eq!(
        app.workbench.current().unwrap().tabs[0].document.text,
        "first\n"
    );
}

#[test]
fn workbench_dirty_tab_and_session_cannot_be_removed() {
    let mut app = fixture();
    open_file(&mut app, "script", "old");
    send(
        &mut app,
        WorkbenchAction::Edit(PopupEditorCommand::Insert('x')),
    );
    send(&mut app, WorkbenchAction::CloseTab);
    assert_eq!(app.workbench.current().unwrap().tabs.len(), 1);
    send(&mut app, WorkbenchAction::Sessions);
    send(&mut app, WorkbenchAction::RemoveSession);
    assert_eq!(app.workbench.sessions.len(), 1);
}

#[test]
fn workbench_late_save_keeps_edits_typed_while_disk_write_was_pending() {
    let mut app = fixture();
    open_file(&mut app, "file", "old");
    send(
        &mut app,
        WorkbenchAction::Edit(PopupEditorCommand::Insert('a')),
    );
    let Some(Effect::Workbench(WorkbenchRequest::Save {
        session,
        path,
        content,
        ..
    })) = send(&mut app, WorkbenchAction::Save)
    else {
        panic!()
    };
    send(
        &mut app,
        WorkbenchAction::Edit(PopupEditorCommand::Insert('b')),
    );
    send(
        &mut app,
        WorkbenchAction::Saved {
            session,
            path,
            content: content.clone(),
            result: Ok(()),
        },
    );
    let tab = &app.workbench.current().unwrap().tabs[0];
    assert_eq!(tab.original, content);
    assert_eq!(tab.document.text, "abold");
    assert!(tab.is_dirty());
}

#[test]
fn workbench_recovery_retains_dirty_text_unicode_cursor_and_build_identity() {
    let mut app = fixture();
    open_file(&mut app, "file", "hello");
    send(
        &mut app,
        WorkbenchAction::Edit(PopupEditorCommand::Insert('λ')),
    );
    let saved = app.workbench.saved();
    let restored = WorkbenchState::restore(saved.clone()).unwrap();
    assert_eq!(restored.saved(), saved);
    assert!(restored.sessions[0].tabs[0].is_dirty());
    assert_eq!(restored.sessions[0].terminal, None);
}

#[test]
fn workbench_recovery_rejects_duplicate_ids_outside_roots_and_large_buffers() {
    let mut app = fixture();
    open_file(&mut app, "file", "text");
    let mut saved = app.workbench.saved();
    saved.sessions.push(saved.sessions[0].clone());
    assert!(saved.validate().is_err());
    let mut saved = app.workbench.saved();
    saved.sessions[0].tabs[0].path = "/etc/passwd".into();
    assert!(saved.validate().is_err());
    let mut saved = app.workbench.saved();
    saved.sessions[0].tabs[0].text = "x".repeat(TEXTAREA_MAX_BYTES + 1);
    assert!(saved.validate().is_err());
}

#[test]
fn workbench_stale_directory_and_file_results_cannot_replace_current_view() {
    let mut app = fixture();
    let Some(Effect::Workbench(WorkbenchRequest::Directory {
        session,
        root,
        directory,
        generation,
    })) = send(&mut app, WorkbenchAction::Refresh)
    else {
        panic!()
    };
    send(&mut app, WorkbenchAction::Refresh);
    send(
        &mut app,
        WorkbenchAction::DirectoryLoaded {
            session,
            root: root.clone(),
            directory,
            generation,
            result: Ok(vec![LayerBrowserEntry {
                path: root.join("stale"),
                ..Default::default()
            }]),
        },
    );
    assert!(
        app.workbench.current().unwrap().folders[0]
            .browser
            .entries
            .is_empty()
    );
    app.workbench.file_request = 2;
    send(
        &mut app,
        WorkbenchAction::FileLoaded {
            session,
            root: root.clone(),
            path: root.join("stale"),
            generation: 1,
            result: Ok("old".into()),
        },
    );
    assert!(app.workbench.current().unwrap().tabs.is_empty());
}

#[test]
fn workbench_escape_returns_pane_then_sessions_then_navigator() {
    let mut app = fixture();
    open_file(&mut app, "file", "text");
    send(&mut app, WorkbenchAction::Cancel);
    assert_eq!(
        app.workbench.current().unwrap().focus,
        WorkbenchFocus::Files
    );
    send(&mut app, WorkbenchAction::Cancel);
    assert!(app.workbench.active.is_none());
    send(&mut app, WorkbenchAction::Cancel);
    assert_eq!(app.focus, FocusTarget::Navigator);
}

#[test]
fn workbench_wrong_build_cannot_run_recipe_actions() {
    let mut app = fixture();
    app.workspace.build_dir = Some("/other/build".into());
    assert!(
        send(
            &mut app,
            WorkbenchAction::RecipeAction(Box::new(Action::BeginSelectedRecipeBuild))
        )
        .is_none()
    );
    assert!(app.active_dialog().is_none());
    assert!(app.notification.as_ref().unwrap().contains("another build"));
}

#[test]
fn workbench_build_reviews_bound_recipe_and_returns_to_editor_on_cancel_or_confirm() {
    let mut app = fixture();
    app.workspace.recipes.push(Recipe {
        name: "busybox".into(),
        file: Some("/recipes/busybox.bb".into()),
        ..Default::default()
    });
    open_file(&mut app, "source.c", "int x;\n");
    send(
        &mut app,
        WorkbenchAction::RecipeAction(Box::new(Action::BeginSelectedRecipeBuild)),
    );
    assert!(
        matches!(app.active_dialog(), Some(Dialog::RecipeTaskConfirmation(request)) if request.targets == ["busybox"])
    );
    update(&mut app, Action::CancelRecipeTask);
    assert_eq!(app.screen, Screen::Workbench);
    assert_eq!(
        app.workbench.current().unwrap().tabs[0].document.text,
        "int x;\n"
    );
    send(
        &mut app,
        WorkbenchAction::RecipeAction(Box::new(Action::BeginSelectedRecipeBuild)),
    );
    assert!(
        matches!(update(&mut app, Action::ConfirmRecipeTask), Some(Effect::Start(request)) if request.targets == ["busybox"])
    );
    assert_eq!(app.screen, Screen::Workbench);
    assert_eq!(app.focus, FocusTarget::Workspace);
}

#[test]
fn workbench_dirty_source_blocks_build_without_publishing_or_losing_buffer() {
    let mut app = fixture();
    open_file(&mut app, "source.c", "int x;\n");
    send(
        &mut app,
        WorkbenchAction::Edit(PopupEditorCommand::Insert('/')),
    );
    send(
        &mut app,
        WorkbenchAction::RecipeAction(Box::new(Action::BeginSelectedRecipeBuild)),
    );
    assert!(app.active_dialog().is_none());
    assert!(app.notification.unwrap().contains("Save open changes"));
}

#[test]
fn workbench_is_in_build_navigator_and_f12_and_palette() {
    let app = fixture();
    assert_eq!(
        workspace_screen_destination(Screen::Workbench),
        WorkspaceDestination::Workbench
    );
    assert_eq!(
        ApplicationMenuGroup::for_command(CommandId::OpenWorkbench),
        ApplicationMenuGroup::Build
    );
    assert_eq!(CommandId::OpenWorkbench.screen(), Some(Screen::Workbench));
    let definition = global_operator_action_definition(CommandId::OpenWorkbench);
    assert_eq!(definition.menu_path[0], "Build");
    assert!(app.workbench.saved().validate().is_ok());
}

#[test]
fn workbench_adding_folder_finishes_for_original_session_after_switching() {
    let mut app = fixture();
    send(
        &mut app,
        WorkbenchAction::Form(WorkbenchFormKind::AddFolder),
    );
    send(&mut app, WorkbenchAction::FormClear);
    for c in "/source/extra".chars() {
        send(&mut app, WorkbenchAction::FormText(c));
    }
    let Some(Effect::Workbench(WorkbenchRequest::Folder {
        session,
        generation,
        ..
    })) = send(&mut app, WorkbenchAction::ConfirmForm)
    else {
        panic!()
    };
    send(
        &mut app,
        WorkbenchAction::OpenRecipe {
            recipe: "other".into(),
            root: "/source/other".into(),
        },
    );
    let active = app.workbench.active;
    send(
        &mut app,
        WorkbenchAction::FolderLoaded {
            session,
            generation,
            result: Ok("/source/extra".into()),
        },
    );
    assert_eq!(app.workbench.active, active);
    assert_eq!(
        app.workbench.sessions[0].folders[1].browser.root,
        PathBuf::from("/source/extra")
    );
}
