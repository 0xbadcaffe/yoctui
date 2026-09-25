//! Input routing.
use super::*;

pub(crate) fn direct_menu_shortcut_action(
    app: &App,
    input: Input,
    replayed: bool,
) -> Option<Action> {
    if replayed || app.command_palette_open {
        return None;
    }
    if yoctui_app::terminal_owns_input(app) {
        if matches!(
            input,
            Input::F1
                | Input::F2
                | Input::F3
                | Input::F4
                | Input::F5
                | Input::F6
                | Input::F7
                | Input::F8
                | Input::F9
                | Input::F12
        ) {
            return yoctui_app::key_action(input);
        }
        return None;
    }
    match input {
        Input::F12 => Some(Action::OpenApplicationMenu),
        Input::Char('a')
            if app.active_dialog().is_none() && !yoctui_app::workspace_text_input_active(app) =>
        {
            Some(Action::OpenContextMenu)
        }
        _ => None,
    }
}

pub(crate) fn pane_focus_route(app: &App, input: Input) -> Option<Action> {
    focus_action_for_app(app, input)
}

pub(crate) fn workspace_owns_focus_key(app: &App, input: Input) -> bool {
    (input == Input::Esc && app.screen == Screen::Layers && app.layer_browser.is_some())
        || (matches!(input, Input::Tab | Input::BackTab)
            && matches!(
                app.screen,
                Screen::Images | Screen::Kernel | Screen::Firmware
            ))
}

pub(crate) fn global_search_return_action(app: &App) -> Option<Action> {
    (app.command_palette_mode == yoctui_model::CommandPaletteMode::GlobalRegexSearch
        && matches!(
            app.global_search_content,
            yoctui_model::GlobalSearchContentState::Ready { .. }
        ))
    .then_some(Action::RestoreGlobalSearchResults)
}

pub(crate) fn layer_list_open_action(app: &App, input: Input) -> Option<Action> {
    (app.screen == Screen::Layers
        && app.layer_browser.is_none()
        && matches!(input, Input::Enter | Input::Right | Input::Char('l')))
    .then_some(Action::BeginSelectedLayerBrowser)
}

pub(crate) fn notification_input_action(
    visible: bool,
    actionable: bool,
    settings_retry_available: bool,
    input: Input,
) -> Option<Action> {
    if !visible || (settings_retry_available && input == Input::Char('r')) {
        return None;
    }
    match input {
        Input::Enter if actionable => Some(Action::ActivateNotification),
        Input::Esc => Some(Action::DismissNotification),
        _ => None,
    }
}

pub(crate) fn active_popup_accepts_paste(app: &yoctui_model::App) -> bool {
    match app.active_dialog() {
        Some(
            Dialog::BuildEnvironmentEditor(editor)
            | Dialog::BuildEnvironmentCloneEditor(editor)
            | Dialog::BbmaskEdit(editor)
            | Dialog::SdkPublishTomlEditor(editor)
            | Dialog::SdkNativeTomlEditor(editor),
        ) => editor.editing,
        Some(Dialog::ConfigEdit { editor, .. } | Dialog::BuildTarget { editor, .. }) => {
            editor.editing
        }
        Some(
            Dialog::WicCreateTomlEditor { editor, .. }
            | Dialog::TestLaunchTomlEditor { editor, .. }
            | Dialog::TestResultImportTomlEditor { editor, .. }
            | Dialog::TestComparisonTomlEditor { editor, .. }
            | Dialog::TestJunitTomlEditor { editor, .. },
        ) => editor.editing,
        Some(Dialog::Security(yoctui_model::SecurityDialog::Import { editor, .. }))
        | Some(Dialog::Qa(yoctui_model::QaDialog::Import { editor, .. })) => editor.editing,
        Some(Dialog::Maintenance(dialog)) => match dialog.as_ref() {
            yoctui_model::MaintenanceDialog::ReadinessToml { editor, .. }
            | yoctui_model::MaintenanceDialog::CleanupToml { editor, .. }
            | yoctui_model::MaintenanceDialog::PrServiceToml { editor, .. }
            | yoctui_model::MaintenanceDialog::LockedCacheToml { editor, .. }
            | yoctui_model::MaintenanceDialog::BuildHistoryToml { editor, .. }
            | yoctui_model::MaintenanceDialog::GitArchiveToml { editor, .. } => editor.editing,
            _ => false,
        },
        _ => false,
    }
}

pub(crate) fn terminal_event_requires_full_redraw(event: &Event) -> bool {
    matches!(event, Event::Resize(_, _))
}

pub(crate) fn mouse_kind_from_event(kind: crossterm::event::MouseEventKind) -> Option<MouseKind> {
    match kind {
        crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Right) => {
            Some(MouseKind::ContextDown)
        }
        crossterm::event::MouseEventKind::Down(_) => Some(MouseKind::Down),
        crossterm::event::MouseEventKind::Drag(_) => Some(MouseKind::Drag),
        crossterm::event::MouseEventKind::Up(_) => Some(MouseKind::Up),
        crossterm::event::MouseEventKind::ScrollUp => Some(MouseKind::ScrollUp),
        crossterm::event::MouseEventKind::ScrollDown => Some(MouseKind::ScrollDown),
        _ => None,
    }
}

pub(crate) fn input_from_key(key: KeyEvent) -> Option<Input> {
    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Input::CtrlC),
        KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Input::CtrlV),
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Input::CtrlS),
        KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Input::CtrlB),
        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Input::CtrlP),
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Input::CtrlU),
        KeyCode::F(1) => Some(Input::F1),
        KeyCode::F(2) => Some(Input::F2),
        KeyCode::F(3) => Some(Input::F3),
        KeyCode::F(4) => Some(Input::F4),
        KeyCode::F(5) => Some(Input::F5),
        KeyCode::F(6) => Some(Input::F6),
        KeyCode::F(7) => Some(Input::F7),
        KeyCode::F(8) => Some(Input::F8),
        KeyCode::F(9) => Some(Input::F9),
        KeyCode::F(10) => Some(Input::F10),
        KeyCode::F(12) => Some(Input::F12),
        KeyCode::Tab => Some(Input::Tab),
        KeyCode::BackTab => Some(Input::BackTab),
        KeyCode::Char(character) => Some(Input::Char(character)),
        KeyCode::Esc => Some(Input::Esc),
        KeyCode::Enter => Some(Input::Enter),
        KeyCode::Up => Some(Input::Up),
        KeyCode::Down => Some(Input::Down),
        KeyCode::Backspace => Some(Input::Backspace),
        KeyCode::Left => Some(Input::Left),
        KeyCode::Right => Some(Input::Right),
        KeyCode::Home => Some(Input::Home),
        KeyCode::End => Some(Input::End),
        KeyCode::PageUp => Some(Input::PageUp),
        KeyCode::PageDown => Some(Input::PageDown),
        _ => None,
    }
}

pub(crate) fn terminal_input_bytes(input: Input) -> Option<Vec<u8>> {
    let bytes = match input {
        Input::Char(character) => {
            let mut buffer = [0; 4];
            return Some(character.encode_utf8(&mut buffer).as_bytes().to_vec());
        }
        Input::Enter => b"\r".as_slice(),
        Input::CtrlC => b"\x03".as_slice(),
        Input::CtrlV => b"\x16".as_slice(),
        Input::CtrlB => b"\x02".as_slice(),
        Input::CtrlP => b"\x10".as_slice(),
        Input::CtrlU => b"\x15".as_slice(),
        Input::CtrlS => b"\x13".as_slice(),
        Input::Tab => b"\t".as_slice(),
        Input::BackTab => b"\x1b[Z".as_slice(),
        Input::Esc => b"\x1b".as_slice(),
        Input::Backspace => b"\x7f".as_slice(),
        Input::Up => b"\x1b[A".as_slice(),
        Input::Down => b"\x1b[B".as_slice(),
        Input::Right => b"\x1b[C".as_slice(),
        Input::Left => b"\x1b[D".as_slice(),
        Input::Home => b"\x1b[H".as_slice(),
        Input::End => b"\x1b[F".as_slice(),
        Input::PageUp => b"\x1b[5~".as_slice(),
        Input::PageDown => b"\x1b[6~".as_slice(),
        Input::F1 => b"\x1bOP".as_slice(),
        Input::F2 => b"\x1bOQ".as_slice(),
        Input::F3 => b"\x1bOR".as_slice(),
        Input::F4 => b"\x1bOS".as_slice(),
        Input::F5 => b"\x1b[15~".as_slice(),
        Input::F6 => b"\x1b[17~".as_slice(),
        Input::F7 => b"\x1b[18~".as_slice(),
        Input::F8 => b"\x1b[19~".as_slice(),
        Input::F9 => b"\x1b[20~".as_slice(),
        Input::F10 => b"\x1b[21~".as_slice(),
        Input::F12 => b"\x1b[24~".as_slice(),
    };
    Some(bytes.to_vec())
}

pub(crate) const MAX_NORMAL_RENDER_RATE: Duration = Duration::from_millis(100);

pub(crate) fn interactive_frame_interval(configured_refresh: Duration) -> Duration {
    configured_refresh.max(MAX_NORMAL_RENDER_RATE)
}
