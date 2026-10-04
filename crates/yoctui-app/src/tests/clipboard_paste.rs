use super::*;
use yoctui_model::Dialog;
use yoctui_model::{
    KernelDebugDialog, KernelDebugDraft, KernelDebugTool, TextAreaMode, TextAreaPasteSource,
};

fn paste(app: &mut App, text: &str) -> Result<(), String> {
    for action in
        text_paste_actions(app, text.into(), TextAreaPasteSource::Clipboard)?.unwrap_or_default()
    {
        assert!(
            yoctui_model::update(app, action).is_none(),
            "paste must not launch/submit"
        );
    }
    Ok(())
}

fn kernel() -> App {
    let mut app = App::new(32, 4096);
    app.screen = Screen::Kernel;
    app.focus = FocusTarget::Dialog;
    app.dialogs
        .push_back(Dialog::KernelDebug(KernelDebugDialog {
            draft: KernelDebugDraft::new(KernelDebugTool::QemuGdb),
            selection: 0,
            guide_scroll: 0,
            error: None,
        }));
    app
}

#[test]
fn clipboard_paste_kernel_paths_are_literal_bounded_and_never_submit() {
    let mut app = kernel();
    assert!(text_paste_active(&app));
    paste(&mut app, "/home/猫 kernel/bin/runqemu").unwrap();
    let Some(Dialog::KernelDebug(dialog)) = app.active_dialog() else {
        panic!()
    };
    assert_eq!(dialog.draft.qemu.runqemu, "/home/猫 kernel/bin/runqemu");
    assert!(app.kernel_debug.pending.is_none());
    for unsafe_text in ["\nEnter".into(), "escape\x1b[31m".into(), "x".repeat(4097)] {
        assert!(paste(&mut app, &unsafe_text).is_err());
    }
    let Some(Dialog::KernelDebug(dialog)) = app.active_dialog_mut() else {
        panic!()
    };
    assert_eq!(dialog.draft.qemu.runqemu, "/home/猫 kernel/bin/runqemu");
    dialog.selection = 7; // boot-mode selector, not a text field.
    assert!(!text_paste_active(&app));
}

#[test]
fn clipboard_paste_popup_preserves_multiline_selection_and_single_undo() {
    let mut app = App::new(32, 4096);
    let mut editor = yoctui_model::PopupEditor::new("original".into());
    editor.set_mode(TextAreaMode::Insert);
    app.dialogs
        .push_back(Dialog::BuildEnvironmentEditor(editor));
    paste(&mut app, "\nUTF-8 猫\n").unwrap();
    let Some(Dialog::BuildEnvironmentEditor(editor)) = app.active_dialog_mut() else {
        panic!()
    };
    assert_eq!(editor.text, "original\nUTF-8 猫\n");
    assert_eq!(editor.history_lengths(), (1, 0));
    editor.undo();
    assert_eq!(editor.text, "original");
    assert!(paste(&mut app, &"x".repeat(262145)).is_err());
}

#[test]
fn clipboard_paste_search_is_text_and_modal_menu_terminal_guards_hold() {
    let mut app = App::new(32, 4096);
    app.command_palette_open = true;
    paste(&mut app, "literal xyz").unwrap();
    assert_eq!(app.command_palette_query, "literal xyz");
    app.dialogs.push_back(Dialog::BuildOptions);
    assert!(!text_paste_active(&app));
    app.dialogs.clear();
    app.command_palette_open = false;
    app.screen = Screen::TerminalSessions;
    app.focus = FocusTarget::Workspace;
    assert!(
        !text_paste_active(&app),
        "native PTY Ctrl+V must stay native"
    );
    app.terminal.mode = yoctui_model::TerminalWorkbenchMode::KillConfirmation;
    app.command_palette_open = true;
    assert!(!text_paste_active(&app));
}

#[test]
fn clipboard_paste_source_editor_routes_a_single_multiline_action() {
    let mut app = App::new(32, 4096);
    yoctui_model::update(
        &mut app,
        Action::OpenRecipeEditor {
            recipe: "source".into(),
            root: "/work".into(),
            files: vec!["entry".into()],
        },
    );
    yoctui_model::update(
        &mut app,
        Action::LoadRecipeEditorContent("original\n".into()),
    );
    yoctui_model::update(&mut app, Action::ToggleRecipeEditorEditing);
    if let Some(Dialog::RecipeEditor(editor)) = app.active_dialog_mut() {
        editor.document.select_position(1, 0, false);
    }
    let actions = text_paste_actions(
        &app,
        "UTF-8 猫\nline two\n".into(),
        TextAreaPasteSource::Clipboard,
    )
    .unwrap()
    .unwrap();
    assert_eq!(actions.len(), 1);
    assert!(matches!(
        &actions[0],
        Action::EditRecipeEditor(yoctui_model::PopupEditorCommand::PasteText {
            source: TextAreaPasteSource::Clipboard,
            ..
        })
    ));
    for action in actions {
        yoctui_model::update(&mut app, action);
    }
    let Some(Dialog::RecipeEditor(editor)) = app.active_dialog() else {
        panic!()
    };
    assert_eq!(editor.document.text, "original\nUTF-8 猫\nline two\n");
}

#[test]
fn clipboard_paste_raw_form_uses_owned_editor_even_with_dialog_focus() {
    let mut app = App::new(32, 4096);
    app.screen = Screen::RawMode;
    app.focus = FocusTarget::Dialog;
    app.raw_mode.view = yoctui_model::RawModeView::Form;
    app.raw_mode.focus = yoctui_model::RawModeFocus::Form;
    let mut additional_arguments = yoctui_model::RawArgvEditor::new("").unwrap();
    additional_arguments.editor.set_mode(TextAreaMode::Insert);
    app.raw_mode.form = Some(yoctui_model::RawCommandForm {
        command: yoctui_model::RawCatalog::builtin().commands[0].id.clone(),
        fields: Default::default(),
        field_order: Vec::new(),
        field_selection: 0,
        additional_arguments,
        capability_generation: 1,
        build_directory: "/work/build".into(),
    });
    let actions = text_paste_actions(
        &app,
        "--literal value".into(),
        TextAreaPasteSource::Clipboard,
    )
    .unwrap()
    .unwrap();
    assert!(matches!(
        &actions[0],
        Action::RawMode(yoctui_model::RawModeAction::EditAdditionalArguments(
            yoctui_model::PopupEditorCommand::PasteText {
                source: TextAreaPasteSource::Clipboard,
                ..
            }
        ))
    ));
}

#[test]
fn clipboard_paste_kernel_pending_review_and_popup_normal_mode_are_trapped() {
    let mut app = kernel();
    app.kernel_debug.pending = Some(yoctui_model::KernelDebugOperation::Inspect);
    assert!(!text_paste_active(&app));
    assert!(
        text_paste_actions(&app, "path".into(), TextAreaPasteSource::BracketedPaste)
            .unwrap()
            .is_none()
    );
    app.dialogs.clear();
    app.dialogs.push_back(Dialog::BuildEnvironmentEditor(
        yoctui_model::PopupEditor::new("original".into()),
    ));
    assert!(!text_paste_active(&app));
}
