use super::*;

fn git_app() -> App {
    let mut app = App::new(32, 8_192);
    let identity = yoctui_model::RecipeIdentity {
        name: "busybox".into(),
        file: "/layers/meta/recipes-core/busybox.bb".into(),
    };
    app.devtool_statuses.insert(
        identity.clone(),
        yoctui_model::DevtoolStatus {
            identity,
            capability: yoctui_model::DevtoolCapability::Available,
            workspace: yoctui_model::DevtoolWorkspace::Present {
                source_path: "/workspace/busybox/source".into(),
                recipe_file: None,
            },
            git: yoctui_model::DevtoolGitState::Available {
                repository_root: Some("/workspace/busybox".into()),
                branch: Some("feature/editor".into()),
                upstream: Some("origin/feature/editor".into()),
                ahead: 2,
                behind: 1,
                head: Some("abc123".into()),
                modified: 1,
                untracked: 2,
                conflicted: 0,
            },
            error: None,
        },
    );
    let _ = update(
        &mut app,
        Action::OpenRecipeEditor {
            recipe: "busybox".into(),
            root: "/workspace/busybox/source".into(),
            files: vec!["main.c".into()],
        },
    );
    app
}

#[test]
fn editor_gitui_local_preview_and_f12_menu_preserve_dirty_buffer() {
    let mut app = App::new(32, 8_192);
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
    update(&mut app, Action::ToggleRecipeEditorEditing);
    update(&mut app, Action::AppendRecipeEditor('!'));
    let editor = app.active_dialog().cloned().unwrap();
    update(&mut app, Action::OpenApplicationMenu);
    update(&mut app, Action::SelectMenuGroup { delta: 6 });
    for (width, height) in [(160, 50), (100, 30), (80, 24)] {
        let text = rendered_text(&app, width, height);
        assert!(text.contains("GitUI"), "{text}");
        assert!(!text.contains("Select a source directory"), "{text}");
        assert!(text.contains("Alt+g"), "{text}");
    }
    update(&mut app, Action::CloseMenu);
    update(&mut app, Action::OpenRecipeEditorGitUi);
    let generation = app.editor_gitui_generation;
    update(
        &mut app,
        Action::RecipeEditorGitUiInspected {
            generation,
            root: "/layers/meta-local".into(),
            result: yoctui_model::SourceGitStatus::Ready(Default::default()),
        },
    );
    for (width, height) in [(160, 50), (100, 30), (80, 24)] {
        let text = rendered_text(&app, width, height);
        assert!(text.contains("/layers/meta-local"), "{text}");
        assert!(text.contains("gitui"), "{text}");
        assert_eq!(app.focus, FocusTarget::Dialog);
    }
    update(&mut app, Action::CancelTerminalLaunch);
    assert_eq!(app.active_dialog(), Some(&editor));
}

#[test]
fn devtool_editor_git_renders_repository_branch_sync_changes_and_gitui_action() {
    let output = rendered_text(&git_app(), 180, 56);
    for anchor in [
        "Repository:",
        "/workspace/busybox",
        "Branch: feature/editor",
        "origin/feature/editor",
        "unsynced",
        "ahead 2",
        "behind 1",
        "Changes: 1 modified",
        "[Alt+g] Open repository in GitUI",
    ] {
        assert!(output.contains(anchor), "missing {anchor:?}:\n{output}");
    }

    let mut workspace = git_app();
    workspace.dialogs.clear();
    workspace.screen = yoctui_model::Screen::Devtool;
    workspace.workspace.recipes.push(yoctui_model::Recipe {
        name: "busybox".into(),
        file: Some("/layers/meta/recipes-core/busybox.bb".into()),
        ..yoctui_model::Recipe::default()
    });
    let output = rendered_text(&workspace, 180, 56);
    for anchor in [
        "Repository: /workspace/busybox",
        "Branch: feature/editor",
        "Upstream: origin/feature/editor",
        "Sync: unsynced · diverged, ahead 2, behind 1",
        "GitUI: Alt+g",
    ] {
        assert!(output.contains(anchor), "missing {anchor:?}:\n{output}");
    }
}
