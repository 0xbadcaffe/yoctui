//! Client-local setup catalogue and reviewed requests for the official tool.
use crate::*;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitBakeSetupEntry {
    pub name: String,
    pub profile: BuildEnvironmentProfile,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BitBakeSetupState {
    pub catalogue: bool,
    pub top_dir: String,
    pub tool: String,
    pub entries: Vec<BitBakeSetupEntry>,
    pub selected: usize,
    pub generation: u64,
    pub loading: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitBakeSetupWizard {
    pub values: [String; 5],
    pub field: usize,
    pub editor: Option<TextAreaState>,
    pub root_only: bool,
    pub review: bool,
    pub preparing: Option<u64>,
    pub error: Option<String>,
}

impl BitBakeSetupWizard {
    pub const LABELS: [&'static str; 5] = [
        "bitbake-setup executable",
        "Top directory",
        "Setup name",
        "Template ID / path / URL (blank: choose)",
        "Registry (blank: tool default)",
    ];
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BitBakeSetupAction {
    Catalogue,
    Environment,
    Refresh,
    Select(isize),
    Import,
    New,
    Root,
    Field(isize),
    Edit,
    Insert(String),
    Motion(TextAreaMotion),
    Backspace,
    Clear,
    AcceptEdit,
    Review,
    Confirm,
    Cancel,
    Listed {
        generation: u64,
        result: Result<Vec<BitBakeSetupEntry>, String>,
    },
    Prepared {
        generation: u64,
        result: Result<TerminalLaunchRequest, String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BitBakeSetupRequest {
    List {
        generation: u64,
        root: PathBuf,
    },
    Prepare {
        generation: u64,
        plan: BitBakeSetupPlan,
    },
}

fn list(app: &mut App) -> Option<Effect> {
    app.bitbake_setup.generation = app.bitbake_setup.generation.wrapping_add(1);
    app.bitbake_setup.loading = true;
    app.bitbake_setup.error = None;
    Some(Effect::BitBakeSetup(BitBakeSetupRequest::List {
        generation: app.bitbake_setup.generation,
        root: app.bitbake_setup.top_dir.clone().into(),
    }))
}

pub(crate) fn reduce(app: &mut App, action: BitBakeSetupAction) -> Option<Effect> {
    use BitBakeSetupAction as A;
    match action {
        A::Catalogue => {
            app.bitbake_setup.catalogue = true;
            return list(app);
        }
        A::Environment => app.bitbake_setup.catalogue = false,
        A::Refresh => return list(app),
        A::Select(delta) => {
            app.bitbake_setup.selected = app
                .bitbake_setup
                .selected
                .saturating_add_signed(delta)
                .min(app.bitbake_setup.entries.len().saturating_sub(1));
        }
        A::Listed { generation, result } if generation == app.bitbake_setup.generation => {
            app.bitbake_setup.loading = false;
            match result {
                Ok(mut entries) => {
                    let previous = app
                        .bitbake_setup
                        .entries
                        .get(app.bitbake_setup.selected)
                        .map(|entry| entry.profile.build_dir.clone());
                    entries.truncate(256);
                    app.bitbake_setup.selected = previous
                        .and_then(|path| {
                            entries
                                .iter()
                                .position(|entry| entry.profile.build_dir == path)
                        })
                        .unwrap_or(0);
                    app.bitbake_setup.entries = entries;
                    app.bitbake_setup.error = None;
                }
                Err(error) => app.bitbake_setup.error = Some(error),
            }
        }
        A::Import => {
            if app.bitbake_setup.loading {
                return None;
            }
            if let Some(entry) = app.bitbake_setup.entries.get(app.bitbake_setup.selected) {
                let profile = entry.profile.clone();
                open_dialog(app, Dialog::EnvironmentSetup(Box::new(EnvironmentSetup {
                    values: [profile.source_dir.display().to_string(), profile.build_dir.display().to_string(),
                        profile.init_script.display().to_string()], field: 0, editor: None,
                    browser: None, error: Some("Review paths, s saves. Verification/daemon switching is separate; no build starts.".into()),
                })));
            }
        }
        A::New | A::Root => {
            if app.active_dialog().is_some() {
                return None;
            }
            let root_only = action == A::Root;
            open_dialog(
                app,
                Dialog::BitBakeSetup(Box::new(BitBakeSetupWizard {
                    values: [
                        app.bitbake_setup.tool.clone(),
                        app.bitbake_setup.top_dir.clone(),
                        "yoctui-setup".into(),
                        String::new(),
                        String::new(),
                    ],
                    field: if root_only { 1 } else { 0 },
                    editor: None,
                    root_only,
                    review: false,
                    preparing: None,
                    error: None,
                })),
            );
        }
        A::Prepared { generation, result } => {
            if !matches!(app.active_dialog(), Some(Dialog::BitBakeSetup(w)) if w.preparing == Some(generation))
            {
                return None;
            }
            match result {
                Ok(request) => {
                    close_dialog(app);
                    app.screen = Screen::TerminalSessions;
                    app.focus = FocusTarget::Workspace;
                    app.focus_return = None;
                    app.prepare_created_terminal_selection();
                    app.notification = Some("Official setup wizard: choose Machine/Distro in the terminal. Alt+o returns to sessions; closing Yoctui does not cancel setup.".into());
                    return Some(Effect::Terminal(TerminalEffect::Create {
                        name: request.name,
                        kind: request.kind,
                        cwd: request.cwd,
                        program: request.program,
                        arguments: request.arguments,
                        completion: None,
                    }));
                }
                Err(error) => {
                    if let Some(Dialog::BitBakeSetup(w)) = app.active_dialog_mut() {
                        w.preparing = None;
                        w.error = Some(error);
                    }
                }
            }
        }
        A::Cancel => {
            if let Some(Dialog::BitBakeSetup(w)) = app.active_dialog_mut() {
                if w.editor.take().is_some() {
                    return None;
                }
                if w.review && w.preparing.is_none() {
                    w.review = false;
                    return None;
                }
                close_dialog(app);
            }
        }
        A::Confirm => {
            if app.daemon.status != ClientReplicaStatus::Current {
                if let Some(Dialog::BitBakeSetup(w)) = app.active_dialog_mut() {
                    w.error = Some("Start/connect a daemon through Tools → Daemons first. An initialized BitBake environment is not required.".into());
                }
                return None;
            }
            let Some(Dialog::BitBakeSetup(w)) = app.active_dialog() else {
                return None;
            };
            if !w.review || w.preparing.is_some() {
                return None;
            }
            let plan = BitBakeSetupPlan::from_values(&w.values).ok()?;
            app.bitbake_setup.generation = app.bitbake_setup.generation.wrapping_add(1);
            let generation = app.bitbake_setup.generation;
            if let Some(Dialog::BitBakeSetup(w)) = app.active_dialog_mut() {
                w.preparing = Some(generation);
            }
            return Some(Effect::BitBakeSetup(BitBakeSetupRequest::Prepare {
                generation,
                plan,
            }));
        }
        A::Review => {
            let Some(Dialog::BitBakeSetup(w)) = app.active_dialog_mut() else {
                return None;
            };
            if w.root_only {
                if let Err(error) = crate::bitbake_setup_plan::validate_top(&w.values[1]) {
                    w.error = Some(error);
                    return None;
                }
                let root = w.values[1].clone();
                close_dialog(app);
                app.bitbake_setup.top_dir = root;
                app.bitbake_setup.catalogue = true;
                return list(app);
            }
            match BitBakeSetupPlan::from_values(&w.values) {
                Ok(_) => {
                    w.review = true;
                    w.error = None;
                }
                Err(error) => w.error = Some(error),
            }
        }
        A::Field(_)
        | A::Edit
        | A::Insert(_)
        | A::Motion(_)
        | A::Backspace
        | A::Clear
        | A::AcceptEdit => {
            let Some(Dialog::BitBakeSetup(w)) = app.active_dialog_mut() else {
                return None;
            };
            if w.review || w.preparing.is_some() {
                return None;
            }
            match action {
                A::Field(delta) if w.editor.is_none() && !w.root_only => {
                    w.field = (w.field as isize + delta.rem_euclid(5)).rem_euclid(5) as usize
                }
                A::Edit => {
                    let mut editor = TextAreaState::new(w.values[w.field].clone());
                    editor.selection = Some((0, editor.text.len()));
                    w.editor = Some(editor);
                }
                A::Insert(text) => {
                    if let Some(editor) = &mut w.editor {
                        let replaced = editor.selection.map_or(0, |(a, b)| b.saturating_sub(a));
                        if !text.chars().any(char::is_control)
                            && editor.text.len().saturating_sub(replaced) + text.len() <= 4096
                        {
                            editor.insert(&text);
                            w.error = None;
                        } else {
                            w.error = Some(
                                "Fields must be at most 4096 bytes with no control characters."
                                    .into(),
                            );
                        }
                    }
                }
                A::Motion(motion) => {
                    if let Some(editor) = &mut w.editor {
                        editor.move_cursor(motion);
                    }
                }
                A::Backspace => {
                    if let Some(editor) = &mut w.editor {
                        editor.backspace();
                    }
                }
                A::Clear => {
                    if let Some(editor) = &mut w.editor {
                        *editor = TextAreaState::new(String::new());
                    }
                }
                A::AcceptEdit => {
                    if let Some(editor) = w.editor.take() {
                        w.values[w.field] = editor.text;
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
    None
}

#[cfg(test)]
#[path = "tests/bitbake_setup.rs"]
mod tests;
