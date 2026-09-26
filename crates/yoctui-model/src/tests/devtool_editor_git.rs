use super::*;

#[test]
fn devtool_editor_git_opens_exact_repository_root_without_recipe_selection() {
    let mut app = App::new(16, 4_096);
    app.gitui_program = Some("/usr/bin/gitui".into());
    let identity = RecipeIdentity {
        name: "busybox".into(),
        file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
    };
    app.devtool_statuses.insert(
        identity.clone(),
        DevtoolStatus {
            identity,
            capability: DevtoolCapability::Available,
            workspace: DevtoolWorkspace::Present {
                source_path: "/workspace/busybox/source".into(),
                recipe_file: None,
            },
            git: DevtoolGitState::Available {
                repository_root: Some("/workspace/busybox".into()),
                branch: Some("feature/editor".into()),
                upstream: Some("origin/feature/editor".into()),
                ahead: 2,
                behind: 1,
                head: Some("abc123".into()),
                modified: 1,
                untracked: 0,
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

    assert_eq!(update(&mut app, Action::OpenRecipeEditorGitUi), None);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::TerminalLaunch(TerminalLaunchDialog { request, .. }))
            if request.kind == TerminalCreationKind::GitUi
                && request.cwd == Path::new("/workspace/busybox")
                && request.program == Path::new("/usr/bin/gitui")
                && request.name == "GitUI · devtool busybox"
    ));
    let _ = update(&mut app, Action::CancelTerminalLaunch);
    assert!(matches!(app.active_dialog(), Some(Dialog::RecipeEditor(_))));
}
