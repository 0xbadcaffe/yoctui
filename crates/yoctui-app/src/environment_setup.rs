//! Read-only local onboarding filesystem adapter and modal key routing.
use crate::Input;
use std::{
    fs,
    path::{Path, PathBuf},
};
use yoctui_model::{
    Action, ENVIRONMENT_DIRECTORY_LIMIT, ENVIRONMENT_PATH_LIMIT, EnvironmentDirectory,
    EnvironmentSetup, EnvironmentSetupAction as A, TextAreaMotion,
};

pub fn environment_setup_action(setup: &EnvironmentSetup, key: Input) -> Option<Action> {
    let action = if setup.editor.is_some() {
        match key {
            Input::Char(c) => A::Insert(c.to_string()),
            Input::Backspace => A::Backspace,
            Input::CtrlU => A::Clear,
            Input::Left => A::Move(TextAreaMotion::Left),
            Input::Right => A::Move(TextAreaMotion::Right),
            Input::Home => A::Move(TextAreaMotion::LineStart),
            Input::End => A::Move(TextAreaMotion::LineEnd),
            Input::Enter => A::AcceptEdit,
            Input::Esc => A::Cancel,
            _ => return None,
        }
    } else if setup.browser.is_some() {
        match key {
            Input::Up | Input::Char('k') => A::Select(-1),
            Input::Down | Input::Char('j') => A::Select(1),
            Input::PageUp => A::Select(-10),
            Input::PageDown => A::Select(10),
            Input::Home => A::Select(isize::MIN),
            Input::End => A::Select(isize::MAX),
            Input::Enter | Input::Right => A::EnterDirectory,
            Input::Left | Input::Backspace => A::ParentDirectory,
            Input::Char('s') => A::ChooseDirectory,
            Input::Esc => A::Cancel,
            _ => return None,
        }
    } else {
        match key {
            Input::Up | Input::BackTab => A::Field(-1),
            Input::Down | Input::Tab => A::Field(1),
            Input::Enter | Input::Char('e') => A::Edit,
            Input::Char('b') => A::Browse,
            Input::Char('s') | Input::CtrlS => A::Save,
            Input::Esc => A::Cancel,
            _ => return None,
        }
    };
    Some(Action::EnvironmentSetup(action))
}

fn usable_path(path: &Path) -> bool {
    path.to_str().is_some_and(|text| {
        text.len() <= ENVIRONMENT_PATH_LIMIT && !text.chars().any(char::is_control)
    })
}

/// Only one directory level, at most 4096 directory entries visited and 1024
/// child directories retained. Call on a blocking worker, never during render.
pub fn read_environment_directory(
    path: &Path,
    initial: bool,
    fallback: &Path,
) -> Result<EnvironmentDirectory, String> {
    if !usable_path(path) {
        return Err("Path is too long or contains unsupported characters.".into());
    }
    let mut path = if path.as_os_str().is_empty() {
        fallback.to_path_buf()
    } else if path.is_absolute() {
        path.to_path_buf()
    } else {
        return Err("Enter an absolute directory path before browsing.".into());
    };
    if initial {
        // Missing build paths start at their existing parent. Permission errors
        // must stay visible rather than silently taking the user elsewhere.
        loop {
            match fs::metadata(&path) {
                Ok(metadata) if metadata.is_dir() => break,
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(format!("Cannot inspect {}: {error}", path.display())),
            }
            if !path.pop() {
                return Err("No existing parent directory found.".into());
            }
        }
    }
    let path = fs::canonicalize(&path)
        .map_err(|error| format!("Cannot open {}: {error}", path.display()))?;
    if !usable_path(&path) {
        return Err("Resolved path cannot be represented safely in the path editor.".into());
    }
    let entries =
        fs::read_dir(&path).map_err(|error| format!("Cannot list {}: {error}", path.display()))?;
    let mut children = Vec::new();
    let mut limited = false;
    let mut skipped = 0usize;
    for (visited, entry) in entries.enumerate() {
        if visited >= 4096 || children.len() >= ENVIRONMENT_DIRECTORY_LIMIT {
            limited = true;
            break;
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        let child = entry.path();
        if !usable_path(&child) {
            skipped += 1;
            continue;
        }
        match fs::metadata(&child) {
            Ok(metadata) if metadata.is_dir() => children.push(child),
            Ok(_) => {}
            Err(_) => skipped += 1,
        }
    }
    children.sort();
    let init_script = [
        "oe-init-build-env",
        "layers/openembedded-core/oe-init-build-env",
    ]
    .into_iter()
    .map(|name| path.join(name))
    .find(|candidate| candidate.is_file())
    .and_then(|candidate| fs::canonicalize(candidate).ok())
    .filter(|candidate| usable_path(candidate) && candidate.starts_with(&path));
    let notice = if limited {
        Some("Listing limited to 1024 directories / 4096 entries; enter a more specific path manually.".into())
    } else if skipped > 0 {
        Some(format!(
            "Skipped {skipped} unreadable or unsupported entries."
        ))
    } else {
        None
    };
    Ok(EnvironmentDirectory {
        path,
        children,
        init_script,
        notice,
    })
}

pub fn environment_browser_fallback() -> PathBuf {
    yoctui_utils::home_dir()
}

#[cfg(test)]
#[path = "tests/environment_setup/mod.rs"]
mod tests;
