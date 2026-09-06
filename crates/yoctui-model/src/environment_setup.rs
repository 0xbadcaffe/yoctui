//! Transient onboarding drafts. Filesystem inspection belongs to the app adapter.
use crate::{
    Action, App, BuildEnvironmentProfile, BuildEnvironmentState, Dialog, Effect, TextAreaMotion,
    TextAreaState, close_dialog, open_dialog, update,
};
use std::path::PathBuf;

pub const ENVIRONMENT_DIRECTORY_LIMIT: usize = 1024;
pub const ENVIRONMENT_PATH_LIMIT: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentDirectory {
    pub path: PathBuf,
    pub children: Vec<PathBuf>,
    pub init_script: Option<PathBuf>,
    pub notice: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentBrowser {
    pub request: u64,
    pub loading: bool,
    pub directory: Option<EnvironmentDirectory>,
    pub selection: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentSetup {
    pub values: [String; 3],
    pub field: usize,
    pub editor: Option<TextAreaState>,
    pub browser: Option<EnvironmentBrowser>,
    pub error: Option<String>,
}

impl EnvironmentSetup {
    pub const LABELS: [&'static str; 3] =
        ["Source directory", "Build directory", "Environment script"];
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvironmentSetupAction {
    Open {
        browse: bool,
    },
    Field(isize),
    Edit,
    Insert(String),
    Move(TextAreaMotion),
    Backspace,
    Clear,
    AcceptEdit,
    Browse,
    Select(isize),
    EnterDirectory,
    ParentDirectory,
    ChooseDirectory,
    DirectoryLoaded {
        request: u64,
        result: Result<EnvironmentDirectory, String>,
    },
    Save,
    Cancel,
}

fn request_directory(app: &mut App, path: PathBuf, initial: bool) -> Option<Effect> {
    app.environment_setup_generation = app.environment_setup_generation.wrapping_add(1);
    let request = app.environment_setup_generation;
    let Some(Dialog::EnvironmentSetup(setup)) = app.active_dialog_mut() else {
        return None;
    };
    setup.error = None;
    let previous = setup.browser.take().and_then(|browser| browser.directory);
    setup.browser = Some(EnvironmentBrowser {
        request,
        loading: true,
        directory: previous,
        selection: 0,
    });
    Some(Effect::ReadEnvironmentDirectory {
        request,
        path,
        initial,
    })
}

pub(crate) fn environment_setup_update(
    app: &mut App,
    action: EnvironmentSetupAction,
) -> Option<Effect> {
    use EnvironmentSetupAction as A;
    if let A::Open { browse } = action {
        if app.active_dialog().is_some() {
            return None;
        }
        let values = match &app.build_environment {
            BuildEnvironmentState::Configured(p)
            | BuildEnvironmentState::Connected(p)
            | BuildEnvironmentState::Verifying { profile: p, .. }
            | BuildEnvironmentState::Failed { profile: p, .. } => [
                p.source_dir.display().to_string(),
                p.build_dir.display().to_string(),
                p.init_script.display().to_string(),
            ],
            BuildEnvironmentState::Unconfigured => Default::default(),
        };
        open_dialog(
            app,
            Dialog::EnvironmentSetup(Box::new(EnvironmentSetup {
                values,
                field: 0,
                editor: None,
                browser: None,
                error: None,
            })),
        );
        return if browse {
            environment_setup_update(app, A::Browse)
        } else {
            None
        };
    }
    let Some(Dialog::EnvironmentSetup(setup)) = app.active_dialog_mut() else {
        return None;
    };
    match action {
        A::Open { .. } => unreachable!(),
        A::Cancel => {
            if setup.editor.take().is_none() && setup.browser.take().is_none() {
                close_dialog(app);
            } else {
                setup.error = None;
            }
        }
        A::DirectoryLoaded { request, result } => {
            if let Some(browser) = &mut setup.browser
                && browser.loading
                && browser.request == request
            {
                browser.loading = false;
                match result {
                    Ok(mut directory) => {
                        directory.children.truncate(ENVIRONMENT_DIRECTORY_LIMIT);
                        browser.directory = Some(directory);
                        setup.error = None;
                    }
                    Err(error) => {
                        setup.error = Some(error);
                    }
                }
            }
        }
        A::Insert(text) => {
            if let Some(editor) = &mut setup.editor {
                let replaced = editor
                    .selection
                    .map_or(0, |(start, end)| end.saturating_sub(start));
                if text.chars().any(char::is_control)
                    || editor
                        .text
                        .len()
                        .saturating_sub(replaced)
                        .saturating_add(text.len())
                        > ENVIRONMENT_PATH_LIMIT
                {
                    setup.error =
                        Some("Paths must fit 4096 bytes and contain no control characters.".into());
                } else {
                    editor.insert(&text);
                    setup.error = None;
                }
            }
        }
        A::Move(motion) => {
            if let Some(editor) = &mut setup.editor {
                editor.move_cursor(motion);
            }
        }
        A::Backspace => {
            if let Some(editor) = &mut setup.editor {
                editor.backspace();
            }
        }
        A::Clear => {
            if let Some(editor) = &mut setup.editor {
                *editor = TextAreaState::new(String::new());
            }
        }
        A::AcceptEdit => {
            if let Some(editor) = setup.editor.take() {
                setup.values[setup.field] = editor.text;
                setup.error = None;
            }
        }
        A::Field(delta) if setup.browser.is_none() && setup.editor.is_none() => {
            setup.field = (setup.field as isize + delta.rem_euclid(3)).rem_euclid(3) as usize;
        }
        A::Edit if setup.browser.is_none() && setup.editor.is_none() => {
            let mut editor = TextAreaState::new(setup.values[setup.field].clone());
            editor.selection = Some((0, editor.text.len()));
            setup.editor = Some(editor);
            setup.error = None;
        }
        A::Browse if setup.editor.is_none() && setup.browser.is_none() => {
            if setup.field == 2 {
                setup.error = Some(
                    "The script is a file: use e to enter its path, or browse Source to detect it."
                        .into(),
                );
                return None;
            }
            let path = PathBuf::from(&setup.values[setup.field]);
            return request_directory(app, path, true);
        }
        A::Select(delta) => {
            if let Some(browser) = &mut setup.browser
                && !browser.loading
            {
                let count = browser.directory.as_ref().map_or(0, |d| d.children.len());
                browser.selection = browser
                    .selection
                    .saturating_add_signed(delta)
                    .min(count.saturating_sub(1));
            }
        }
        A::EnterDirectory | A::ParentDirectory => {
            if let Some(browser) = &setup.browser
                && !browser.loading
                && let Some(directory) = &browser.directory
            {
                let path = if matches!(action, A::ParentDirectory) {
                    directory.path.parent().map(PathBuf::from)
                } else {
                    directory.children.get(browser.selection).cloned()
                };
                if let Some(path) = path {
                    return request_directory(app, path, false);
                }
            }
        }
        A::ChooseDirectory => {
            if let Some(browser) = &setup.browser
                && !browser.loading
                && let Some(directory) = &browser.directory
            {
                setup.values[setup.field] = directory.path.display().to_string();
                if setup.field == 0 {
                    setup.values[2] = directory
                        .init_script
                        .as_ref()
                        .map_or_else(String::new, |p| p.display().to_string());
                    setup.error = directory.init_script.is_none().then(|| "No oe-init-build-env found. Choose a Yocto source directory or enter a custom script path.".into());
                } else {
                    setup.error = None;
                }
                setup.browser = None;
            }
        }
        A::Save if setup.browser.is_none() && setup.editor.is_none() => {
            let profile = BuildEnvironmentProfile {
                source_dir: setup.values[0].clone().into(),
                build_dir: setup.values[1].clone().into(),
                init_script: setup.values[2].clone().into(),
            };
            if let Err(error) = profile.validate() {
                setup.error = Some(error.to_string());
            } else {
                close_dialog(app);
                return update(app, Action::ConfigureBuildEnvironment(profile));
            }
        }
        _ => {}
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    fn action(app: &mut App, a: EnvironmentSetupAction) -> Option<Effect> {
        update(app, Action::EnvironmentSetup(a))
    }
    fn setup(app: &App) -> &EnvironmentSetup {
        let Some(Dialog::EnvironmentSetup(s)) = app.active_dialog() else {
            panic!("setup missing")
        };
        s
    }
    #[test]
    fn environment_setup_manual_save_cancel_and_validation() {
        let mut app = App::new(20, 2000);
        app.build_environment = BuildEnvironmentState::Unconfigured;
        action(&mut app, EnvironmentSetupAction::Open { browse: false });
        action(&mut app, EnvironmentSetupAction::Save);
        assert!(setup(&app).error.is_some());
        for value in [
            "/tmp/source with spaces",
            "/tmp/new-build",
            "/tmp/source with spaces/oe-init-build-env",
        ] {
            action(&mut app, EnvironmentSetupAction::Edit);
            action(&mut app, EnvironmentSetupAction::Insert(value.into()));
            action(&mut app, EnvironmentSetupAction::AcceptEdit);
            action(&mut app, EnvironmentSetupAction::Field(1));
        }
        action(&mut app, EnvironmentSetupAction::Save);
        assert!(
            matches!(&app.build_environment, BuildEnvironmentState::Configured(p) if p.build_dir == std::path::Path::new("/tmp/new-build"))
        );
        let before = app.build_environment.clone();
        action(&mut app, EnvironmentSetupAction::Open { browse: false });
        action(&mut app, EnvironmentSetupAction::Edit);
        action(
            &mut app,
            EnvironmentSetupAction::Insert("/elsewhere".into()),
        );
        action(&mut app, EnvironmentSetupAction::Cancel);
        action(&mut app, EnvironmentSetupAction::Cancel);
        assert_eq!(app.build_environment, before);
    }
    #[test]
    fn environment_setup_browser_rejects_stale_results_and_keeps_selection_bounded() {
        let mut app = App::new(20, 2000);
        let Some(Effect::ReadEnvironmentDirectory { request, .. }) =
            action(&mut app, EnvironmentSetupAction::Open { browse: true })
        else {
            panic!("read missing")
        };
        action(&mut app, EnvironmentSetupAction::Cancel);
        action(&mut app, EnvironmentSetupAction::Browse);
        action(
            &mut app,
            EnvironmentSetupAction::DirectoryLoaded {
                request,
                result: Err("stale".into()),
            },
        );
        assert!(setup(&app).error.is_none());
        let request = setup(&app).browser.as_ref().unwrap().request;
        action(
            &mut app,
            EnvironmentSetupAction::DirectoryLoaded {
                request,
                result: Ok(EnvironmentDirectory {
                    path: "/yocto".into(),
                    children: vec!["/yocto/sub".into()],
                    init_script: Some("/yocto/oe-init-build-env".into()),
                    notice: None,
                }),
            },
        );
        action(&mut app, EnvironmentSetupAction::Select(isize::MAX));
        assert_eq!(setup(&app).browser.as_ref().unwrap().selection, 0);
        action(&mut app, EnvironmentSetupAction::ChooseDirectory);
        assert_eq!(setup(&app).values[0], "/yocto");
        assert_eq!(setup(&app).values[2], "/yocto/oe-init-build-env");
    }

    #[test]
    fn environment_setup_manual_unicode_bounds_and_browser_errors_preserve_draft() {
        let mut app = App::new_unconfigured(20, 2000);
        action(&mut app, EnvironmentSetupAction::Open { browse: false });
        action(&mut app, EnvironmentSetupAction::Edit);
        action(&mut app, EnvironmentSetupAction::Insert("/tmp/café".into()));
        action(&mut app, EnvironmentSetupAction::Backspace);
        assert_eq!(setup(&app).editor.as_ref().unwrap().text, "/tmp/caf");
        action(
            &mut app,
            EnvironmentSetupAction::Insert("\nnot-a-path".into()),
        );
        assert!(setup(&app).error.is_some());
        assert_eq!(setup(&app).editor.as_ref().unwrap().text, "/tmp/caf");
        action(&mut app, EnvironmentSetupAction::Clear);
        action(
            &mut app,
            EnvironmentSetupAction::Insert("/".repeat(ENVIRONMENT_PATH_LIMIT)),
        );
        action(&mut app, EnvironmentSetupAction::Insert("x".into()));
        assert_eq!(
            setup(&app).editor.as_ref().unwrap().text.len(),
            ENVIRONMENT_PATH_LIMIT
        );
        action(&mut app, EnvironmentSetupAction::Cancel);
        let Some(Effect::ReadEnvironmentDirectory { request, .. }) =
            action(&mut app, EnvironmentSetupAction::Browse)
        else {
            panic!("read missing")
        };
        action(
            &mut app,
            EnvironmentSetupAction::DirectoryLoaded {
                request,
                result: Err("Permission denied".into()),
            },
        );
        assert_eq!(setup(&app).error.as_deref(), Some("Permission denied"));
        action(&mut app, EnvironmentSetupAction::ChooseDirectory);
        assert!(setup(&app).values[0].is_empty());
        action(&mut app, EnvironmentSetupAction::Cancel);
        action(&mut app, EnvironmentSetupAction::Cancel);
        assert!(matches!(
            app.build_environment,
            BuildEnvironmentState::Unconfigured
        ));
    }
}
