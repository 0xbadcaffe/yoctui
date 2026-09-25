//! Raw input.
use super::*;

pub fn popup_editor_action(editing: bool, key: Input) -> Option<Action> {
    let command = if editing {
        match key {
            Input::Esc => PopupEditorCommand::ToggleInsert,
            Input::Enter => PopupEditorCommand::Newline,
            Input::Backspace => PopupEditorCommand::Backspace,
            Input::Left => PopupEditorCommand::Left,
            Input::Right => PopupEditorCommand::Right,
            Input::Up => PopupEditorCommand::Up,
            Input::Down => PopupEditorCommand::Down,
            Input::Home => PopupEditorCommand::Home,
            Input::End => PopupEditorCommand::End,
            Input::PageUp => PopupEditorCommand::PageUp,
            Input::PageDown => PopupEditorCommand::PageDown,
            Input::CtrlC => PopupEditorCommand::Copy,
            Input::CtrlV => PopupEditorCommand::Paste,
            Input::Char(character) => PopupEditorCommand::Insert(character),
            _ => return None,
        }
    } else {
        match key {
            Input::Char('i') => PopupEditorCommand::ToggleInsert,
            Input::Char('v') => PopupEditorCommand::ToggleVisual,
            Input::Char('e') => PopupEditorCommand::SelectValue,
            Input::Char('x') => PopupEditorCommand::Delete,
            Input::Char('b') => PopupEditorCommand::WordLeft,
            Input::Char('w') => PopupEditorCommand::WordRight,
            Input::Char('u') => PopupEditorCommand::Undo,
            Input::Char('r') => PopupEditorCommand::Redo,
            Input::Left | Input::Char('h') => PopupEditorCommand::Left,
            Input::Right | Input::Char('l') => PopupEditorCommand::Right,
            Input::Up | Input::Char('k') => PopupEditorCommand::Up,
            Input::Down | Input::Char('j') => PopupEditorCommand::Down,
            Input::Home => PopupEditorCommand::Home,
            Input::End => PopupEditorCommand::End,
            Input::PageUp => PopupEditorCommand::PageUp,
            Input::PageDown => PopupEditorCommand::PageDown,
            Input::CtrlC => PopupEditorCommand::Copy,
            _ => return None,
        }
    };
    Some(Action::EditActivePopup(command))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RawArgvEditorAction {
    Edit(PopupEditorCommand),
    Validate,
}

pub fn raw_argv_editor_action(editing: bool, key: Input) -> Option<RawArgvEditorAction> {
    if key == Input::Enter {
        return Some(RawArgvEditorAction::Validate);
    }
    let Action::EditActivePopup(command) = popup_editor_action(editing, key)? else {
        return None;
    };
    Some(RawArgvEditorAction::Edit(command))
}

pub fn validate_raw_argv_editor(
    editor: &mut yoctui_model::RawArgvEditor,
) -> Result<Vec<String>, yoctui_model::RawArgvError> {
    editor
        .validate()
        .map(|arguments| arguments.as_slice().to_vec())
}

pub fn reduce_raw_mode_state(
    state: &mut yoctui_model::RawModeState,
    catalog: &yoctui_model::RawCatalog,
    authority: Option<&yoctui_model::DaemonCompatibilitySnapshot>,
    action: yoctui_model::RawModeAction,
) {
    yoctui_model::reduce_raw_mode(state, catalog, authority, action);
}

pub trait RawModeInputContext {
    fn raw_mode_state(&self) -> &yoctui_model::RawModeState;
    fn raw_mode_app(&self) -> Option<&yoctui_model::App>;
}

impl RawModeInputContext for yoctui_model::App {
    fn raw_mode_state(&self) -> &yoctui_model::RawModeState {
        &self.raw_mode
    }

    fn raw_mode_app(&self) -> Option<&yoctui_model::App> {
        Some(self)
    }
}

impl RawModeInputContext for yoctui_model::RawModeState {
    fn raw_mode_state(&self) -> &yoctui_model::RawModeState {
        self
    }

    fn raw_mode_app(&self) -> Option<&yoctui_model::App> {
        None
    }
}

pub fn raw_mode_input<C: RawModeInputContext + ?Sized>(
    context: &C,
    key: Input,
) -> Option<yoctui_model::RawModeAction> {
    use yoctui_model::{RawBrowserColumn, RawModeAction, RawModeView};
    let state = context.raw_mode_state();

    if state.recipe_picker.is_some() {
        return match key {
            Input::Up | Input::Char('k') => Some(RawModeAction::SelectRecipePicker { delta: -1 }),
            Input::Down | Input::Char('j') => Some(RawModeAction::SelectRecipePicker { delta: 1 }),
            Input::PageUp => Some(RawModeAction::SelectRecipePicker { delta: -10 }),
            Input::PageDown => Some(RawModeAction::SelectRecipePicker { delta: 10 }),
            Input::Enter => Some(RawModeAction::ConfirmRecipePicker),
            Input::Esc => Some(RawModeAction::CancelRecipePicker),
            Input::Backspace => Some(RawModeAction::BackspaceRecipePickerQuery),
            Input::Char(character) => Some(RawModeAction::AppendRecipePickerQuery(character)),
            _ => None,
        };
    }

    if state.favorite_confirmation.is_some() {
        return match key {
            Input::Enter => Some(RawModeAction::ConfirmFavorite),
            Input::Esc => Some(RawModeAction::CancelFavorite),
            _ => None,
        };
    }
    if state.view == RawModeView::Execution && state.output.searching {
        return match key {
            Input::Esc => Some(RawModeAction::FinishOutputSearch),
            Input::CtrlU => Some(RawModeAction::ClearOutputSearch),
            Input::Backspace => Some(RawModeAction::BackspaceOutputSearch),
            Input::Char(character) => Some(RawModeAction::AppendOutputSearch(character)),
            _ => None,
        };
    }
    if state.search.editing {
        return match key {
            Input::Esc => Some(RawModeAction::FinishSearch),
            Input::CtrlU => Some(RawModeAction::ClearSearch),
            Input::Backspace => Some(RawModeAction::BackspaceSearch),
            Input::Char(character) => Some(RawModeAction::AppendSearch(character)),
            _ => None,
        };
    }
    if let Some(delta) = collection_scroll_delta(key) {
        return match state.view {
            RawModeView::Browser => Some(match state.browser_column {
                RawBrowserColumn::Categories => RawModeAction::SelectCategory { delta },
                RawBrowserColumn::Commands => RawModeAction::SelectCommand { delta },
            }),
            RawModeView::History => Some(RawModeAction::SelectHistory { delta }),
            RawModeView::Favorites => Some(RawModeAction::SelectFavorite { delta }),
            RawModeView::Execution => {
                let vertical = match key {
                    Input::Home => isize::MAX,
                    Input::End | Input::Char('G') => isize::MIN,
                    _ => delta.saturating_neg(),
                };
                Some(RawModeAction::ScrollOutput {
                    vertical,
                    horizontal: 0,
                })
            }
            RawModeView::Form | RawModeView::Preview => None,
        };
    }
    match state.view {
        RawModeView::Browser => match key {
            Input::Char('/') => Some(RawModeAction::BeginSearch),
            Input::Char('f') => Some(RawModeAction::ToggleFavorite),
            Input::Char('H') => Some(RawModeAction::OpenHistory),
            Input::Left | Input::Char('h') => Some(RawModeAction::FocusCategories),
            Input::Right | Input::Char('l') => Some(RawModeAction::FocusCommands),
            Input::Up | Input::Char('k') => Some(match state.browser_column {
                RawBrowserColumn::Categories => RawModeAction::SelectCategory { delta: -1 },
                RawBrowserColumn::Commands => RawModeAction::SelectCommand { delta: -1 },
            }),
            Input::Down | Input::Char('j') => Some(match state.browser_column {
                RawBrowserColumn::Categories => RawModeAction::SelectCategory { delta: 1 },
                RawBrowserColumn::Commands => RawModeAction::SelectCommand { delta: 1 },
            }),
            Input::Enter if state.browser_column == RawBrowserColumn::Categories => {
                Some(RawModeAction::FocusCommands)
            }
            Input::Enter => Some(RawModeAction::OpenSelected),
            Input::Esc => Some(RawModeAction::Back),
            _ => None,
        },
        RawModeView::Form => {
            let form = state.form.as_ref()?;
            let parameter = form.field_order.get(form.field_selection);
            let editing = parameter
                .and_then(|parameter| form.fields.get(parameter))
                .map_or(form.additional_arguments.editor.editing, |field| {
                    field.editor.editing
                });
            match key {
                Input::Tab => return Some(RawModeAction::SelectFormField { delta: 1 }),
                Input::BackTab => return Some(RawModeAction::SelectFormField { delta: -1 }),
                _ => {}
            }
            if !editing {
                match key {
                    Input::Down | Input::Char('j') => {
                        return Some(RawModeAction::SelectFormField { delta: 1 });
                    }
                    Input::Up | Input::Char('k') => {
                        return Some(RawModeAction::SelectFormField { delta: -1 });
                    }
                    Input::Char('q') | Input::Esc => return Some(RawModeAction::Back),
                    Input::Char('r') if parameter.is_some() => {
                        let parameter = parameter?;
                        let command = yoctui_model::builtin_raw_catalog().command(&form.command)?;
                        let definition = command
                            .parameters
                            .iter()
                            .find(|definition| &definition.id == parameter)?;
                        if definition.kind == yoctui_model::RawParameterKind::Recipe {
                            let app = context.raw_mode_app()?;
                            let selector =
                                raw_form_parameter_selector(app, command, parameter).ok()?;
                            let recipes = selector
                                .inventory
                                .choices()?
                                .iter()
                                .filter_map(|choice| match &choice.value {
                                    yoctui_model::RawParameterValue::Recipe(recipe) => {
                                        Some(recipe.clone())
                                    }
                                    _ => None,
                                })
                                .collect();
                            return Some(RawModeAction::OpenRecipePicker {
                                parameter: parameter.clone(),
                                recipes,
                            });
                        }
                    }
                    Input::Left if parameter.is_some() => {
                        if let Some(action) = context
                            .raw_mode_app()
                            .and_then(|app| raw_form_selector_choice(app, parameter?, -1))
                        {
                            return Some(action);
                        }
                    }
                    Input::Right if parameter.is_some() => {
                        if let Some(action) = context
                            .raw_mode_app()
                            .and_then(|app| raw_form_selector_choice(app, parameter?, 1))
                        {
                            return Some(action);
                        }
                    }
                    _ => {}
                }
            }
            let action = raw_argv_editor_action(editing, key)?;
            Some(match action {
                RawArgvEditorAction::Edit(command) => match parameter {
                    Some(parameter) => RawModeAction::EditParameterInput {
                        parameter: parameter.clone(),
                        command,
                    },
                    None => RawModeAction::EditAdditionalArguments(command),
                },
                RawArgvEditorAction::Validate => RawModeAction::RequestPreview,
            })
        }
        RawModeView::Preview => match key {
            Input::Enter => Some(RawModeAction::ConfirmPreview),
            Input::Esc => Some(RawModeAction::Back),
            _ => None,
        },
        RawModeView::Execution => {
            match key {
                Input::Char('f') => Some(RawModeAction::ToggleOutputFollow),
                Input::Char('/') => Some(RawModeAction::BeginOutputSearch),
                Input::Char('1') => Some(RawModeAction::SelectOutputStream(
                    yoctui_model::RawOutputStream::Stdout,
                )),
                Input::Char('2') => Some(RawModeAction::SelectOutputStream(
                    yoctui_model::RawOutputStream::Stderr,
                )),
                Input::Up | Input::Char('k') => Some(RawModeAction::ScrollOutput {
                    vertical: 1,
                    horizontal: 0,
                }),
                Input::Down | Input::Char('j') => Some(RawModeAction::ScrollOutput {
                    vertical: -1,
                    horizontal: 0,
                }),
                Input::Left | Input::Char('h') => Some(RawModeAction::ScrollOutput {
                    vertical: 0,
                    horizontal: -1,
                }),
                Input::Right | Input::Char('l') => Some(RawModeAction::ScrollOutput {
                    vertical: 0,
                    horizontal: 1,
                }),
                Input::Char('c') => state
                    .selected_execution()
                    .map(|execution| RawModeAction::CancelExecution(execution.request.id.clone())),
                Input::Char('d') => state.selected_execution().map(|execution| {
                    RawModeAction::SetExecutionAttachment {
                        request: execution.request.id.clone(),
                        attachment: yoctui_model::RawAttachmentState::Detached,
                    }
                }),
                Input::Char('r') => state.selected_execution().map(|execution| {
                    RawModeAction::SetExecutionAttachment {
                        request: execution.request.id.clone(),
                        attachment: yoctui_model::RawAttachmentState::Attached,
                    }
                }),
                Input::Esc => Some(RawModeAction::CloseExecution),
                _ => None,
            }
        }
        RawModeView::History => match key {
            Input::Up | Input::Char('k') => Some(RawModeAction::SelectHistory { delta: -1 }),
            Input::Down | Input::Char('j') => Some(RawModeAction::SelectHistory { delta: 1 }),
            Input::Enter => Some(RawModeAction::ActivateHistory),
            Input::Esc => Some(RawModeAction::Back),
            _ => None,
        },
        RawModeView::Favorites => match key {
            Input::Up | Input::Char('k') => Some(RawModeAction::SelectFavorite { delta: -1 }),
            Input::Down | Input::Char('j') => Some(RawModeAction::SelectFavorite { delta: 1 }),
            Input::Enter => Some(RawModeAction::ActivateFavorite),
            Input::Char('i') => Some(RawModeAction::InspectFavorite),
            Input::Char('x') | Input::Char('d') => Some(RawModeAction::RemoveFavorite),
            Input::Char('[') => Some(RawModeAction::MoveFavorite { delta: -1 }),
            Input::Char(']') => Some(RawModeAction::MoveFavorite { delta: 1 }),
            Input::Esc => Some(RawModeAction::Back),
            _ => None,
        },
    }
}
