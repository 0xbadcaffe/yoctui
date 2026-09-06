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
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    mod tempfile {
        use super::*;
        pub struct TempDir(PathBuf);
        impl TempDir {
            pub fn path(&self) -> &Path {
                &self.0
            }
        }
        impl Drop for TempDir {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
        pub fn tempdir() -> std::io::Result<TempDir> {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let unique = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "yoctui-environment-{}-{now}-{unique}",
                std::process::id()
            ));
            fs::create_dir(&path)?;
            Ok(TempDir(path))
        }
    }
    #[test]
    fn environment_setup_directory_scan_missing_build_hidden_and_split_source() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join(".hidden")).unwrap();
        fs::create_dir_all(root.path().join("layers/openembedded-core")).unwrap();
        fs::write(
            root.path()
                .join("layers/openembedded-core/oe-init-build-env"),
            "not executed",
        )
        .unwrap();
        fs::write(root.path().join("regular-file"), "ignored").unwrap();
        let result =
            read_environment_directory(&root.path().join("missing/build"), true, root.path())
                .unwrap();
        assert_eq!(result.path, root.path().canonicalize().unwrap());
        assert_eq!(result.children.len(), 2);
        assert!(result.children[0].ends_with(".hidden"));
        assert!(
            result
                .init_script
                .unwrap()
                .ends_with("layers/openembedded-core/oe-init-build-env")
        );
        assert!(!root.path().join("missing").exists());
        assert!(
            read_environment_directory(&root.path().join("missing"), false, root.path()).is_err()
        );
        assert!(read_environment_directory(Path::new("relative"), true, root.path()).is_err());
        assert!(
            read_environment_directory(&root.path().join("regular-file"), false, root.path())
                .is_err()
        );
    }
    #[test]
    fn environment_setup_directory_scan_is_bounded_and_empty_is_valid() {
        let root = tempfile::tempdir().unwrap();
        assert!(
            read_environment_directory(root.path(), false, root.path())
                .unwrap()
                .children
                .is_empty()
        );
        for i in 0..ENVIRONMENT_DIRECTORY_LIMIT + 2 {
            fs::create_dir(root.path().join(format!("d{i}"))).unwrap();
        }
        let result = read_environment_directory(root.path(), false, root.path()).unwrap();
        assert_eq!(result.children.len(), ENVIRONMENT_DIRECTORY_LIMIT);
        assert!(result.notice.is_some());
    }
    #[cfg(unix)]
    #[test]
    fn environment_setup_directory_symlinks_resolve_without_execution() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("real")).unwrap();
        symlink(root.path().join("real"), root.path().join("link")).unwrap();
        symlink(root.path().join("loop"), root.path().join("loop")).unwrap();
        let result =
            read_environment_directory(&root.path().join("link"), false, root.path()).unwrap();
        assert!(result.path.ends_with("real"));
        assert!(read_environment_directory(&root.path().join("loop"), false, root.path()).is_err());
    }
    #[test]
    fn environment_setup_keys_trap_editing_and_route_pages() {
        let mut setup = EnvironmentSetup {
            values: Default::default(),
            field: 0,
            editor: None,
            browser: None,
            error: None,
        };
        assert_eq!(
            environment_setup_action(&setup, Input::Char('b')),
            Some(Action::EnvironmentSetup(A::Browse))
        );
        setup.editor = Some(yoctui_model::TextAreaState::new(String::new()));
        assert_eq!(
            environment_setup_action(&setup, Input::Char('b')),
            Some(Action::EnvironmentSetup(A::Insert("b".into())))
        );
        assert_eq!(
            environment_setup_action(&setup, Input::Esc),
            Some(Action::EnvironmentSetup(A::Cancel))
        );
        setup.editor = None;
        setup.browser = Some(yoctui_model::EnvironmentBrowser {
            request: 1,
            loading: false,
            directory: None,
            selection: 0,
        });
        assert_eq!(
            environment_setup_action(&setup, Input::PageDown),
            Some(Action::EnvironmentSetup(A::Select(10)))
        );
        assert_eq!(
            environment_setup_action(&setup, Input::Right),
            Some(Action::EnvironmentSetup(A::EnterDirectory))
        );
        let mut app = yoctui_model::App::new_unconfigured(20, 2000);
        app.dialogs
            .push_front(yoctui_model::Dialog::EnvironmentSetup(Box::new(setup)));
        assert_eq!(
            crate::mouse_action_for_app(
                crate::MouseInput {
                    kind: crate::MouseKind::ScrollDown,
                    column: 40,
                    row: 12,
                },
                &app,
                80,
                24
            ),
            Some(Action::EnvironmentSetup(A::Select(1)))
        );
    }
}
