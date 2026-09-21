pub fn recipe_editor_action(editor: &yoctui_model::RecipeEditor, key: Input) -> Option<Action> {
    use yoctui_model::{RecipeEditorFocus as Focus, TextAreaMode};

    if editor.searching {
        return match key {
            Input::Char(character) => Some(Action::AppendRecipeEditorSearch(character)),
            Input::Backspace => Some(Action::BackspaceRecipeEditorSearch),
            Input::Enter | Input::Esc => Some(Action::FinishRecipeEditorSearch),
            _ => None,
        };
    }

    if editor.focus == Focus::Files {
        return match key {
            Input::Esc | Input::Char('q') => Some(Action::CloseRecipeEditor),
            Input::Up | Input::Char('k') => Some(Action::SelectRecipeEditorFile { delta: -1 }),
            Input::Down | Input::Char('j') => Some(Action::SelectRecipeEditorFile { delta: 1 }),
            Input::Enter | Input::Tab => Some(Action::FocusRecipeEditor(Focus::Document)),
            Input::Char('e') => Some(Action::OpenRecipeEditorExternal),
            Input::CtrlS => Some(Action::SaveRecipeEditor),
            Input::CtrlB => Some(Action::BeginRecipeEditorBuild),
            _ => None,
        };
    }

    match key {
        Input::CtrlS => Some(Action::SaveRecipeEditor),
        Input::CtrlB => Some(Action::BeginRecipeEditorBuild),
        Input::Char('e') if editor.document.mode() != TextAreaMode::Insert => {
            Some(Action::OpenRecipeEditorExternal)
        }
        Input::Tab | Input::BackTab => Some(Action::FocusRecipeEditor(Focus::Files)),
        Input::Char('/') if editor.document.mode() != TextAreaMode::Insert => {
            Some(Action::BeginRecipeEditorSearch)
        }
        Input::Char('n') if editor.document.mode() != TextAreaMode::Insert => {
            Some(Action::NextRecipeEditorMatch { backwards: false })
        }
        Input::Char('N') if editor.document.mode() != TextAreaMode::Insert => {
            Some(Action::NextRecipeEditorMatch { backwards: true })
        }
        Input::Esc if editor.document.mode() == TextAreaMode::Normal => {
            Some(Action::FocusRecipeEditor(Focus::Files))
        }
        input => {
            popup_editor_action(editor.document.editing, input).and_then(|action| match action {
                Action::EditActivePopup(command) => Some(Action::EditRecipeEditor(command)),
                _ => None,
            })
        }
    }
}

pub fn config_workspace_action(searching: bool, key: Input) -> Option<Action> {
    if searching {
        return match key {
            Input::Char(character) => Some(Action::AppendMetadataQuery(character)),
            Input::Backspace => Some(Action::BackspaceMetadataQuery),
            Input::CtrlU => Some(Action::ClearMetadataQuery),
            Input::Enter | Input::Esc => Some(Action::FinishMetadataSearch),
            _ => None,
        };
    }
    if let Some(delta) = collection_scroll_delta(key) {
        return Some(Action::SelectConfigVariable { delta });
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectConfigVariable { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectConfigVariable { delta: 1 }),
        Input::Enter => Some(Action::BeginSelectedConfigDetail),
        Input::Char('C') => Some(Action::CopySelectedConfigEffective),
        Input::Char('U') => Some(Action::CopySelectedConfigUnexpanded),
        Input::Char('s') => Some(Action::OpenConfigScopePicker),
        Input::Char('c') => Some(Action::OpenConfigComparison),
        Input::Char('E') => Some(Action::BeginConfigEdit),
        Input::Char('/') => Some(Action::BeginMetadataSearch),
        Input::CtrlU => Some(Action::ClearMetadataQuery),
        Input::Char('o') => Some(Action::OpenSelectedConfigSource),
        _ => None,
    }
}

pub fn config_source_picker_action(key: Input) -> Option<Action> {
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectConfigSource { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectConfigSource { delta: 1 }),
        Input::Enter => Some(Action::OpenSelectedConfigSourceChoice),
        Input::Esc => Some(Action::CancelConfigSourcePicker),
        _ => None,
    }
}

pub fn config_scope_picker_action(key: Input) -> Option<Action> {
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectConfigScope { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectConfigScope { delta: 1 }),
        Input::Enter => Some(Action::ConfirmConfigScope),
        Input::Esc => Some(Action::CancelConfigScopePicker),
        _ => None,
    }
}

pub fn config_compare_dialog_action(key: Input) -> Option<Action> {
    matches!(key, Input::Enter | Input::Esc).then_some(Action::CloseConfigComparison)
}

pub fn config_edit_dialog_action(key: Input) -> Option<Action> {
    match key {
        Input::Char(character) => Some(Action::AppendConfigEdit(character)),
        Input::Backspace => Some(Action::BackspaceConfigEdit),
        Input::Enter => Some(Action::PreviewConfigEdit),
        Input::Esc => Some(Action::CancelConfigEdit),
        _ => None,
    }
}

pub fn config_edit_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmConfigEdit),
        Input::Esc => Some(Action::CancelConfigEditConfirmation),
        _ => None,
    }
}
