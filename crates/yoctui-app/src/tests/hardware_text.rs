use super::*;

#[test]
fn hardware_text_edit_key_keeps_viewer_and_trapped_form_routes() {
    use yoctui_model::{
        HardwareAction, HardwareDocument, HardwareDocumentKind, HardwareProjectAction,
        HardwareProjectForm,
    };
    let mut app = App::new(32, 4096);
    app.screen = Screen::Hardware;
    let edit = Some(Action::Hardware(HardwareAction::EditSelected));
    assert_eq!(hardware_workspace_action(&app, Input::Char('e')), edit);
    assert_eq!(
        hardware_workspace_action(&app, Input::Enter),
        Some(Action::Hardware(HardwareAction::OpenSelected))
    );
    app.hardware.documents.push(HardwareDocument {
        path: "/data/board.pdf".into(),
        category: app.hardware.category,
        kind: HardwareDocumentKind::Pdf,
    });
    yoctui_model::update(&mut app, Action::Hardware(HardwareAction::OpenSelected));
    assert_eq!(hardware_workspace_action(&app, Input::Char('e')), edit);
    app.hardware.viewer = None;
    app.hardware.projects.visible = true;
    assert_eq!(hardware_workspace_action(&app, Input::Char('e')), edit);
    app.hardware.projects.form = Some(HardwareProjectForm::Name {
        value: String::new(),
    });
    assert_eq!(
        hardware_workspace_action(&app, Input::Char('e')),
        Some(Action::Hardware(HardwareAction::Project(
            HardwareProjectAction::EditName('e')
        )))
    );
}

#[test]
fn hardware_text_editor_keeps_edit_save_search_navigation_and_no_recipe_build() {
    let mut editor = yoctui_model::RecipeEditor {
        recipe: "Hardware: driver.c".into(),
        root: "/data/board".into(),
        files: vec!["driver.c".into()],
        file_inventory_truncated: false,
        context: yoctui_model::SourceEditorContext::HardwareProject,
        selection: 0,
        focus: yoctui_model::RecipeEditorFocus::Document,
        language: yoctui_model::SourceLanguage::C,
        document: yoctui_model::TextAreaState::new("int value = 0;\n".into()),
        searching: false,
        pending_search_position: None,
    };
    assert!(matches!(
        recipe_editor_action(&editor, Input::Char('i')),
        Some(Action::EditRecipeEditor(
            yoctui_model::PopupEditorCommand::ToggleInsert
        ))
    ));
    editor.document.set_mode(yoctui_model::TextAreaMode::Insert);
    assert!(matches!(
        recipe_editor_action(&editor, Input::Char('x')),
        Some(Action::EditRecipeEditor(
            yoctui_model::PopupEditorCommand::Insert('x')
        ))
    ));
    assert_eq!(
        recipe_editor_action(&editor, Input::CtrlS),
        Some(Action::SaveRecipeEditor)
    );
    assert_eq!(recipe_editor_action(&editor, Input::CtrlB), None);
    editor.document.set_mode(yoctui_model::TextAreaMode::Normal);
    assert_eq!(
        recipe_editor_action(&editor, Input::Char('/')),
        Some(Action::BeginRecipeEditorSearch)
    );
    assert_eq!(
        recipe_editor_action(&editor, Input::CtrlF),
        Some(Action::BeginRecipeEditorSearch)
    );
    editor.context = yoctui_model::SourceEditorContext::Recipe;
    assert_eq!(
        recipe_editor_action(&editor, Input::CtrlB),
        Some(Action::BeginRecipeEditorBuild)
    );
}
