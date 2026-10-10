use super::*;
use yoctui_model::{DaemonControl, DaemonManagerState};

#[derive(Default)]
pub(super) struct DaemonManagerIo {
    task: Option<tokio::task::JoinHandle<DaemonManagerState>>,
    next_refresh: Option<Instant>,
}

impl DaemonManagerIo {
    fn refresh(&mut self, app: &mut App, action: Option<DaemonControl>) {
        if self.task.is_some() {
            return;
        }
        app.daemon_manager.loading = true;
        app.daemon_manager.message = Some("Refreshing daemon health…".into());
        let build = app.daemon_manager.build_directory.clone();
        let source = app.daemon_manager.source_directory.clone();
        self.task = Some(tokio::spawn(async move {
            let result = if let Some(action) = action {
                Some(crate::daemon_manager_io::control(action, build, source).await)
            } else {
                None
            };
            let mut state = crate::daemon_manager_io::inspect().await;
            if let Some(result) = result {
                state.message =
                    Some(result.unwrap_or_else(|error| format!("Control refused/failed: {error}")));
            }
            state
        }));
    }

    pub(super) async fn poll(&mut self, app: &mut App) -> bool {
        let mut changed = false;
        if self.task.as_ref().is_some_and(|task| task.is_finished()) {
            let result = self.task.take().expect("checked task").await;
            match result {
                Ok(mut state) => {
                    let current = &app.daemon_manager;
                    state.logs_visible = current.logs_visible;
                    state.scroll = current.scroll;
                    state.visible_rows = current.visible_rows;
                    state.editing = current.editing;
                    state.review = current.review;
                    state.source_field = current.source_field;
                    state.source_directory = if current.editing || current.review.is_some() {
                        current.source_directory.clone()
                    } else {
                        app.workspace
                            .source_dir
                            .as_ref()
                            .map(|path| path.display().to_string())
                            .unwrap_or_else(|| current.source_directory.clone())
                    };
                    state.build_directory = if current.editing || current.review.is_some() {
                        current.build_directory.clone()
                    } else {
                        app.workspace
                            .build_dir
                            .as_ref()
                            .map(|path| path.display().to_string())
                            .unwrap_or_else(|| current.build_directory.clone())
                    };
                    if state.message.is_none() {
                        state.message = current
                            .message
                            .clone()
                            .filter(|message| !message.starts_with("Refreshing"));
                    }
                    app.daemon_manager = state;
                }
                Err(error) => {
                    app.daemon_manager.loading = false;
                    app.daemon_manager.message = Some(format!("Daemon inspection failed: {error}"));
                }
            }
            self.next_refresh = Some(Instant::now() + Duration::from_secs(5));
            changed = true;
        }
        if app.screen == Screen::Daemons
            && self.task.is_none()
            && self
                .next_refresh
                .is_none_or(|deadline| Instant::now() >= deadline)
            && !app.daemon_manager.editing
            && app.daemon_manager.review.is_none()
        {
            self.refresh(app, None);
            changed = true;
        }
        changed
    }

    pub(super) fn abort(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

impl InteractiveRuntime {
    pub(super) async fn daemon_workspace_key(&mut self, input: Input) -> Result<bool> {
        if self.app.screen != Screen::Daemons
            || self.app.menu.is_open()
            || self.app.command_palette_open
            || self.app.active_dialog().is_some()
            || self.app.onboarding.open
            || (self.app.focus != yoctui_model::FocusTarget::Workspace
                && !self.app.daemon_manager.editing
                && self.app.daemon_manager.review.is_none())
        {
            return Ok(false);
        }
        if !self.app.daemon_manager.editing && self.app.daemon_manager.review.is_none() {
            let delta = match input {
                Input::Down | Input::Char('j') => Some(1),
                Input::Up | Input::Char('k') => Some(-1),
                Input::PageDown => Some(10),
                Input::PageUp => Some(-10),
                Input::Home => Some(isize::MIN),
                Input::End => Some(isize::MAX),
                _ => None,
            };
            if let Some(delta) = delta {
                self.app.scroll_daemon_manager(delta);
                return Ok(true);
            }
        }
        let state = &mut self.app.daemon_manager;
        if state.editing {
            match input {
                Input::Char(c)
                    if !c.is_control() && state.edited_path().len() + c.len_utf8() <= 4096 =>
                {
                    state.edited_path().push(c)
                }
                Input::Backspace => {
                    state.edited_path().pop();
                }
                Input::CtrlU => state.edited_path().clear(),
                Input::Tab | Input::BackTab => state.source_field = !state.source_field,
                Input::CtrlV => match crate::clipboard::read_system_clipboard().await {
                    Ok(text) => append_configuration_paste(state, &text),
                    Err(error) => state.message = Some(format!("Clipboard unavailable: {error}")),
                },
                Input::Enter => {
                    state.editing = false;
                    state.review = Some(DaemonControl::Configure);
                }
                Input::Esc => state.editing = false,
                _ => {}
            }
            return Ok(true);
        }
        if let Some(action) = state.review {
            match input {
                Input::Enter if !state.loading => {
                    state.review = None;
                    self.daemon_manager_io.refresh(&mut self.app, Some(action));
                }
                Input::Esc => state.review = None,
                _ => {}
            }
            return Ok(true);
        }
        match input {
            Input::Char('1') => {
                state.logs_visible = false;
                state.scroll = 0;
            }
            Input::Char('2') => {
                state.logs_visible = true;
                state.scroll = 0;
            }
            Input::Char('r') => self.daemon_manager_io.refresh(&mut self.app, None),
            Input::Char('c') if !state.loading => {
                state.editing = true;
                state.source_field = false;
                if state.source_directory.is_empty() {
                    state.source_directory = self
                        .app
                        .workspace
                        .source_dir
                        .as_ref()
                        .map(|path| path.display().to_string())
                        .unwrap_or_default();
                }
                if state.build_directory.is_empty() {
                    state.build_directory = self.session_build_dir.display().to_string();
                }
            }
            Input::Char('s') if !state.loading => state.review = Some(DaemonControl::Start),
            Input::Char('x') if !state.loading => state.review = Some(DaemonControl::Stop),
            Input::Char('t') if !state.loading => state.review = Some(DaemonControl::Restart),
            _ => return Ok(false),
        }
        Ok(true)
    }
}

pub(super) fn append_configuration_paste(state: &mut DaemonManagerState, text: &str) {
    let path = state.edited_path();
    for c in text.trim().chars().filter(|c| !c.is_control()) {
        if path.len() + c.len_utf8() > 4096 {
            break;
        }
        path.push(c);
    }
}
