use super::*;

fn editor(focus: yoctui_model::RecipeEditorFocus) -> yoctui_model::RecipeEditor {
    yoctui_model::RecipeEditor {
        recipe: "busybox".into(),
        root: "/workspace/busybox".into(),
        files: vec!["main.c".into()],
        file_inventory_truncated: false,
        selection: 0,
        focus,
        language: yoctui_model::SourceLanguage::C,
        document: yoctui_model::TextAreaState::new("int main(void) {}".into()),
        searching: false,
        pending_search_position: None,
    }
}

#[test]
fn devtool_editor_git_routes_g_from_tree_and_normal_document() {
    for focus in [
        yoctui_model::RecipeEditorFocus::Files,
        yoctui_model::RecipeEditorFocus::Document,
    ] {
        assert_eq!(
            recipe_editor_action(&editor(focus), Input::Char('G')),
            Some(Action::OpenRecipeEditorGitUi)
        );
    }

    let mut insert = editor(yoctui_model::RecipeEditorFocus::Document);
    insert.document.set_mode(yoctui_model::TextAreaMode::Insert);
    assert_eq!(
        recipe_editor_action(&insert, Input::Char('G')),
        Some(Action::EditRecipeEditor(
            yoctui_model::PopupEditorCommand::Insert('G')
        ))
    );
}

#[test]
fn devtool_editor_git_round_trips_complete_tracking_state_through_daemon_protocol() {
    let status = yoctui_model::DevtoolStatus {
        identity: yoctui_model::RecipeIdentity {
            name: "busybox".into(),
            file: "/layers/meta/busybox.bb".into(),
        },
        capability: yoctui_model::DevtoolCapability::Available,
        workspace: yoctui_model::DevtoolWorkspace::Present {
            source_path: "/workspace/busybox/source".into(),
            recipe_file: None,
        },
        git: yoctui_model::DevtoolGitState::Available {
            repository_root: Some("/workspace/busybox".into()),
            branch: Some("feature/editor".into()),
            upstream: Some("origin/feature/editor".into()),
            ahead: 3,
            behind: 2,
            head: Some("abc123".into()),
            modified: 1,
            untracked: 2,
            conflicted: 0,
        },
        error: None,
    };
    let wire = crate::devtool_status_to_protocol(&status);
    assert_eq!(crate::devtool_status_from_protocol(&wire).unwrap(), status);
}
