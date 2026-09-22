use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    Char(char),
    Esc,
    Enter,
    CtrlC,
    CtrlV,
    CtrlB,
    CtrlP,
    CtrlU,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F12,
    Tab,
    BackTab,
    CtrlS,
    Up,
    Down,
    Backspace,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
}

/// Resolve terminal-workbench control keys. `None` deliberately means the
/// input remains eligible for forwarding to the daemon PTY writer.
pub fn terminal_workspace_action(app: &yoctui_model::App, input: Input) -> Option<Action> {
    use yoctui_model::TerminalWorkbenchMode as Mode;
    match app.terminal.mode {
        Mode::Search => match input {
            Input::Enter | Input::Esc => Some(Action::TerminalFinishSearch),
            Input::Backspace => Some(Action::TerminalBackspaceSearch),
            Input::CtrlU => Some(Action::TerminalClearSearch),
            Input::Char(character) => Some(Action::TerminalAppendSearch(character)),
            _ => None,
        },
        Mode::Rename => match input {
            Input::Enter => Some(Action::TerminalConfirmRename),
            Input::Esc => Some(Action::TerminalCancelMode),
            Input::Backspace => Some(Action::TerminalBackspaceRename),
            Input::Char(character) => Some(Action::TerminalAppendRename(character)),
            _ => None,
        },
        Mode::PasteReview => match input {
            Input::Enter => Some(Action::TerminalConfirmPaste),
            Input::Esc => Some(Action::TerminalCancelMode),
            _ => None,
        },
        Mode::KillConfirmation => match input {
            Input::Enter => Some(Action::TerminalConfirmKill),
            Input::Esc => Some(Action::TerminalCancelMode),
            _ => None,
        },
        Mode::Help => match input {
            Input::Esc | Input::Char('?') => Some(Action::TerminalToggleHelp),
            _ => None,
        },
        Mode::Copy => match input {
            Input::Up | Input::Char('k') => Some(Action::TerminalMoveCopyRow { delta: -1 }),
            Input::Down | Input::Char('j') => Some(Action::TerminalMoveCopyRow { delta: 1 }),
            Input::Enter | Input::Char('y') => Some(Action::TerminalCopyViewport),
            Input::Char('/') => Some(Action::TerminalBeginSearch),
            Input::Esc | Input::Char('v') => Some(Action::TerminalCancelMode),
            _ => None,
        },
        Mode::Live if app.selected_terminal_is_writer() => None,
        Mode::Live => match input {
            Input::Char('n') => Some(Action::TerminalCreateBuildShell),
            Input::Char('s') => Some(Action::TerminalCreateSelectedDevshell),
            Input::Char('m') => Some(Action::TerminalCreateSelectedMenuconfig),
            Input::Enter => Some(Action::TerminalTakeControl),
            Input::Char('/') => Some(Action::TerminalBeginSearch),
            Input::Char('v') => Some(Action::TerminalEnterCopyMode),
            Input::Char('r') => Some(Action::TerminalBeginRename),
            Input::Char('o') => Some(Action::TerminalTakeControl),
            Input::Char('O') => Some(Action::TerminalReleaseControl),
            Input::Char('c') => Some(Action::TerminalReleaseControl),
            Input::Char('x') => Some(Action::TerminalBeginKill),
            Input::Char('?') => Some(Action::TerminalToggleHelp),
            Input::PageUp => Some(Action::TerminalScroll { delta: 10 }),
            Input::PageDown => Some(Action::TerminalScroll { delta: -10 }),
            Input::Home => Some(Action::TerminalScroll { delta: isize::MAX }),
            Input::End => Some(Action::TerminalScroll {
                delta: isize::MIN + 1,
            }),
            _ => None,
        },
    }
}

/// Resolve an explicitly activated Terminal Sessions context-menu item. This
/// separate route prevents ordinary writer keystrokes from becoming controls.
pub fn terminal_context_action(input: Input) -> Option<Action> {
    match input {
        Input::Char('n') => Some(Action::TerminalCreateBuildShell),
        Input::Char('s') => Some(Action::TerminalCreateSelectedDevshell),
        Input::Char('m') => Some(Action::TerminalCreateSelectedMenuconfig),
        Input::Char('o') | Input::Enter => Some(Action::TerminalTakeControl),
        Input::Char('c') => Some(Action::TerminalReleaseControl),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckboxInputAction {
    Toggle,
    Move(isize),
    Primary,
}

pub fn checkbox_input_action(input: Input) -> Option<CheckboxInputAction> {
    match input {
        Input::Char(' ') => Some(CheckboxInputAction::Toggle),
        Input::Up | Input::Char('k') => Some(CheckboxInputAction::Move(-1)),
        Input::Down | Input::Char('j') => Some(CheckboxInputAction::Move(1)),
        Input::Enter => Some(CheckboxInputAction::Primary),
        _ => None,
    }
}

/// Route the controls advertised by a visible guidance/failure popup before
/// screen-local Enter or Esc handling can consume them.
pub fn notification_popup_action(app: &App, input: Input) -> Option<Action> {
    let message = app.notification.as_deref()?;
    if !yoctui_model::notification_requires_acknowledgement(message)
        || app.active_dialog().is_some()
        || app.menu.is_open()
        || app.onboarding.open
        || app.keymap_preferences_ui.open
        || app.command_palette_open
        || (app.screen == Screen::RawMode
            && matches!(
                app.raw_mode.view,
                yoctui_model::RawModeView::Form | yoctui_model::RawModeView::Preview
            ))
    {
        return None;
    }
    match input {
        Input::Enter => Some(Action::ActivateNotification),
        Input::Esc => Some(Action::DismissNotification),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeymapInputResult {
    Action(Box<Action>),
    Pending,
    Unmatched,
}

impl KeymapInputResult {
    pub fn action(self) -> Option<Action> {
        match self {
            Self::Action(action) => Some(*action),
            Self::Pending | Self::Unmatched => None,
        }
    }
}

pub fn keymap_action_for_app(app: &mut yoctui_model::App, key: Input) -> KeymapInputResult {
    if app.onboarding.open
        || app.keymap_preferences_ui.open
        || app.menu.is_open()
        || app.active_dialog().is_some()
        || app.command_palette_open
        || matches!(app.focus, FocusTarget::Dialog | FocusTarget::CommandPalette)
    {
        app.keymap_chord.clear();
        return KeymapInputResult::Unmatched;
    }
    let stroke = input_key_stroke(key);
    let workspace = yoctui_model::workspace_screen_destination(app.screen);
    match app
        .effective_keymap
        .resolve_input(&mut app.keymap_chord, workspace, stroke)
    {
        yoctui_model::KeymapResolution::Pending => KeymapInputResult::Pending,
        yoctui_model::KeymapResolution::Unmatched => key_action(key)
            .filter(|action| !catalog_routed_action(action))
            .map(Box::new)
            .map_or(KeymapInputResult::Unmatched, KeymapInputResult::Action),
        yoctui_model::KeymapResolution::Activated(action_id) => {
            let definition = yoctui_model::global_operator_action_definitions()
                .into_iter()
                .find(|definition| definition.id == action_id)
                .expect("effective keymaps retain catalog action IDs");
            let yoctui_model::OperatorActionTarget::Command(command) = definition.target else {
                unreachable!("the current effective keymap contains command targets")
            };
            KeymapInputResult::Action(Box::new(yoctui_model::command_action(app, command)))
        }
    }
}

/// Route menuconfig-style global search without stealing `/` from an active
/// text editor, contextual search, or daemon-owned terminal session.
pub fn global_search_action(app: &yoctui_model::App, key: Input) -> Option<Action> {
    if key != Input::Char('/')
        || app.onboarding.open
        || app.keymap_preferences_ui.open
        || app.menu.is_open()
        || app.active_dialog().is_some()
        || app.command_palette_open
        || matches!(app.focus, FocusTarget::Dialog | FocusTarget::CommandPalette)
        || app.screen == yoctui_model::Screen::TerminalSessions
        || workspace_text_input_active(app)
    {
        None
    } else {
        Some(Action::OpenGlobalSearch)
    }
}

pub fn command_palette_navigation_action(key: Input) -> Option<Action> {
    let delta = match key {
        Input::Up => -1,
        Input::Down => 1,
        Input::PageUp => -10,
        Input::PageDown => 10,
        _ => return None,
    };
    Some(Action::SelectCommandPalette { delta })
}

/// Screen-local editors and searches retain text, Escape and navigation keys.
pub fn workspace_text_input_active(app: &yoctui_model::App) -> bool {
    (app.screen == yoctui_model::Screen::BuildEnvironment
        && app
            .build_environment_draft
            .as_ref()
            .is_some_and(|draft| draft.editing))
        || (app.screen == yoctui_model::Screen::Tasks && app.task_filter_editing)
        || (app.screen == yoctui_model::Screen::Logs
            && (app.logs.searching || app.internal_logs.searching))
        || (matches!(
            app.screen,
            yoctui_model::Screen::Recipes
                | yoctui_model::Screen::Layers
                | yoctui_model::Screen::Configuration
        ) && app.metadata_searching)
        || (app.screen == yoctui_model::Screen::Dependencies && app.dependency_graph_searching)
        || (app.screen == yoctui_model::Screen::Packages && app.package_searching)
        || (app.screen == yoctui_model::Screen::Images && app.image_artifact_searching)
        || (app.screen == yoctui_model::Screen::Sdk && app.sdk_artifact_searching)
        || (app.screen == yoctui_model::Screen::Testing && app.test_result_searching)
        || (app.screen == yoctui_model::Screen::Security && app.security.searching)
        || (app.screen == yoctui_model::Screen::Qa && app.qa.searching)
        || (app.screen == yoctui_model::Screen::Compatibility && app.compatibility_ui.searching)
        || (app.screen == yoctui_model::Screen::RawMode
            && (app.raw_mode.search.editing || app.raw_mode.output.searching))
        || (app.screen == yoctui_model::Screen::RawMode
            && matches!(
                app.raw_mode.view,
                yoctui_model::RawModeView::Form | yoctui_model::RawModeView::Preview
            ))
}

pub fn terminal_owns_input(app: &yoctui_model::App) -> bool {
    app.screen == yoctui_model::Screen::TerminalSessions
        && app.focus == FocusTarget::Workspace
        && (app.selected_terminal_is_writer()
            || app.terminal.mode != yoctui_model::TerminalWorkbenchMode::Live)
}

pub fn input_key_stroke(key: Input) -> yoctui_model::KeyStroke {
    use yoctui_model::KeyStroke as Stroke;
    match key {
        Input::Char(character) => Stroke::Char(character),
        Input::Esc => Stroke::Esc,
        Input::Enter => Stroke::Enter,
        Input::CtrlC => Stroke::CtrlC,
        Input::CtrlV => Stroke::CtrlV,
        Input::CtrlB => Stroke::CtrlB,
        Input::CtrlP => Stroke::CtrlP,
        Input::CtrlU => Stroke::CtrlU,
        Input::F1 => Stroke::F1,
        Input::F2 => Stroke::F2,
        Input::F3 => Stroke::F3,
        Input::F4 => Stroke::F4,
        Input::F5 => Stroke::F5,
        Input::F6 => Stroke::F6,
        Input::F7 => Stroke::F7,
        Input::F8 => Stroke::F8,
        Input::F9 => Stroke::F9,
        Input::F10 => Stroke::F10,
        Input::F12 => Stroke::F12,
        Input::Tab => Stroke::Tab,
        Input::BackTab => Stroke::BackTab,
        Input::CtrlS => Stroke::CtrlS,
        Input::Up => Stroke::Up,
        Input::Down => Stroke::Down,
        Input::Backspace => Stroke::Backspace,
        Input::Left => Stroke::Left,
        Input::Right => Stroke::Right,
        Input::Home => Stroke::Home,
        Input::End => Stroke::End,
        Input::PageUp => Stroke::PageUp,
        Input::PageDown => Stroke::PageDown,
    }
}
