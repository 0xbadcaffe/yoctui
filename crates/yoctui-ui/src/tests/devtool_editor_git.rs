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
        "[G] Open repository in GitUI",
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
        "GitUI: G",
    ] {
        assert!(output.contains(anchor), "missing {anchor:?}:\n{output}");
    }
}
