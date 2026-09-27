use super::*;
use crossterm::event::KeyEventKind;

/// Resolve platform terminal ownership before any screen-local shortcuts.
pub(crate) fn platform_menuconfig_key(app: &mut App, key: KeyEvent) -> Option<Option<Effect>> {
    if app.active_dialog().is_some()
        || app.menu.is_open()
        || app.onboarding.open
        || app.command_palette_open
        || app.keymap_preferences_ui.open
    {
        return None;
    }
    if key.kind == KeyEventKind::Release {
        return Some(None);
    }
    if matches!(key.code, KeyCode::Char('g' | 'G'))
        && key.modifiers.contains(KeyModifiers::CONTROL)
        && app.platform_menuconfig_running()
    {
        let effect =
            compatibility_workspace_action(app, Action::TogglePlatformMenuconfigForeground);
        return Some(effect);
    }
    if !app.platform_menuconfig_visible() {
        return None;
    }
    if !app.selected_terminal_is_writer() {
        app.notification =
            Some("Waiting for menuconfig keyboard control; Ctrl+G returns to Yoctui.".into());
        return Some(None);
    }
    let application_cursor = app
        .selected_terminal_screen()
        .is_some_and(|s| s.application_cursor);
    let bytes = terminal_key_bytes(key, application_cursor)?;
    let details = app.selected_terminal_details()?;
    Some(Some(Effect::Terminal(
        yoctui_model::TerminalEffect::Input {
            session_id: details.id,
            writer_epoch: details.writer_epoch,
            bytes,
        },
    )))
}

pub(crate) fn terminal_key_bytes(key: KeyEvent, application_cursor: bool) -> Option<Vec<u8>> {
    if key.kind == KeyEventKind::Release {
        return None;
    }
    let modifier = 1
        + u8::from(key.modifiers.contains(KeyModifiers::SHIFT))
        + 2 * u8::from(key.modifiers.contains(KeyModifiers::ALT))
        + 4 * u8::from(key.modifiers.contains(KeyModifiers::CONTROL));
    let cursor = match key.code {
        KeyCode::Up => Some('A'),
        KeyCode::Down => Some('B'),
        KeyCode::Right => Some('C'),
        KeyCode::Left => Some('D'),
        KeyCode::Home => Some('H'),
        KeyCode::End => Some('F'),
        _ => None,
    };
    if let Some(cursor) = cursor {
        return Some(if modifier > 1 {
            format!("\x1b[1;{modifier}{cursor}").into_bytes()
        } else if application_cursor {
            format!("\x1bO{cursor}").into_bytes()
        } else {
            format!("\x1b[{cursor}").into_bytes()
        });
    }
    let mut bytes = if let KeyCode::Char(c) = key.code {
        if key.modifiers.contains(KeyModifiers::CONTROL) && c.is_ascii() {
            match c {
                ' ' | '@' | '2' => vec![0],
                'a'..='z' | 'A'..='Z' | '['..='_' => vec![(c as u8) & 0x1f],
                '?' => vec![0x7f],
                _ => c.to_string().into_bytes(),
            }
        } else {
            c.to_string().into_bytes()
        }
    } else {
        match key.code {
            KeyCode::Insert => b"\x1b[2~".to_vec(),
            KeyCode::F(11) => b"\x1b[23~".to_vec(),
            _ => terminal_input_bytes(input_from_key(key)?)?,
        }
    };
    if !matches!(key.code, KeyCode::Char(_)) && modifier > 1 {
        if bytes.starts_with(b"\x1b[") && bytes.ends_with(b"~") {
            let number = std::str::from_utf8(&bytes[2..bytes.len() - 1]).ok()?;
            return Some(format!("\x1b[{number};{modifier}~").into_bytes());
        }
        if bytes.starts_with(b"\x1bO") && bytes.len() == 3 {
            return Some(format!("\x1b[1;{modifier}{}", char::from(bytes[2])).into_bytes());
        }
    }
    if key.modifiers.contains(KeyModifiers::ALT) {
        bytes.insert(0, 0x1b);
    }
    Some(bytes)
}
