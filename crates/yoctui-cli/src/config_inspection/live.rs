//! Fresh configuration is client-local; never replace the daemon's recipe inventory.
use super::*;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::mpsc::{Receiver, sync_channel};

mod query;

#[derive(Default)]
pub(crate) struct ConfigurationIo {
    context: Option<(PathBuf, Option<PathBuf>)>,
    watcher: Option<RecommendedWatcher>,
    events: Option<Receiver<notify::Result<notify::Event>>>,
    watched: std::collections::HashSet<PathBuf>,
    worker: Option<crate::daemon_metadata::StartupMetadata<query::FreshConfiguration>>,
    requested: Option<VariableIdentity>,
    running: Option<VariableIdentity>,
    saved: Option<VariableIdentity>,
    dirty: bool,
    revision: u64,
    running_revision: u64,
    ready_at: Option<Instant>,
    visible: bool,
}

impl ConfigurationIo {
    pub(crate) fn active(&self) -> bool {
        self.worker.is_some()
    }

    pub(crate) fn refresh(&mut self) {
        self.dirty = true;
        self.revision = self.revision.wrapping_add(1);
        self.ready_at = Some(Instant::now());
    }

    pub(crate) fn inspect(&mut self, identity: VariableIdentity) {
        self.requested = Some(identity);
        self.refresh();
    }

    pub(crate) fn saved(&mut self, identity: VariableIdentity) {
        self.saved = Some(identity.clone());
        self.inspect(identity);
    }

    fn watch(&mut self, path: PathBuf, recursive: RecursiveMode) {
        if self.watched.len() >= 256 || self.watched.contains(&path) {
            return;
        }
        if let Some(watcher) = self.watcher.as_mut()
            && watcher.watch(&path, recursive).is_ok()
        {
            self.watched.insert(path);
        }
    }

    fn start_watching(&mut self, app: &App, build_dir: &Path) {
        let (send, events) = sync_channel(64);
        match notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
            if event.as_ref().map_or(true, |event| {
                matches!(
                    event.kind,
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
                )
            }) {
                // A full queue already contains a refresh trigger; never grow it.
                let _ = send.try_send(event);
            }
        }) {
            Ok(watcher) => {
                self.watcher = Some(watcher);
                self.events = Some(events);
                self.watch(build_dir.join("conf"), RecursiveMode::Recursive);
                for layer in &app.workspace.layers {
                    self.watch(layer.path.join("conf"), RecursiveMode::Recursive);
                }
            }
            Err(error) => {
                tracing::warn!(%error, "configuration watch unavailable; use r to refresh")
            }
        }
    }

    pub(crate) async fn poll(&mut self, app: &mut App, build_dir: &Path, enabled: bool) -> bool {
        let visible = app.screen == Screen::Configuration;
        let context = (
            build_dir.to_path_buf(),
            app.source_repository_path().map(Path::to_owned),
        );
        if self.context.as_ref() != Some(&context) || !enabled {
            if let Some(identity) = &self.requested {
                app.variable_detail_loading.remove(identity);
            }
            if let Some(identity) = &self.running {
                app.variable_detail_loading.remove(identity);
            }
            self.shutdown().await;
            self.context = Some(context.clone());
            self.watcher = None;
            self.events = None;
            self.watched.clear();
            self.requested = None;
            self.saved = None;
            self.dirty = false;
            self.visible = false;
            if !enabled {
                return false;
            }
        }
        if visible && !self.visible {
            self.refresh();
            if self.watcher.is_none() {
                self.start_watching(app, build_dir);
            }
        }
        self.visible = visible;
        let mut changed = false;
        if self.events.as_ref().is_some_and(|events| {
            events.try_iter().take(1024).fold(false, |dirty, event| {
                dirty
                    | (event.is_err()
                        || event.is_ok_and(|event| {
                            matches!(
                                event.kind,
                                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
                            )
                        }))
            })
        }) {
            self.refresh();
            self.ready_at = Some(Instant::now() + Duration::from_millis(350));
        }
        if let Some(result) = self.worker.as_mut().and_then(|worker| worker.try_result()) {
            self.worker = None;
            let identity = self.running.take();
            if self.running_revision == self.revision {
                match result {
                    Ok(Some(fresh)) => {
                        apply_snapshot(app, fresh.workspace);
                        if let Some((identity, result)) = fresh.detail {
                            match result {
                                Ok(variable) => {
                                    for operation in &variable.operations {
                                        if let Some(parent) =
                                            operation.file.as_ref().and_then(|p| p.parent())
                                        {
                                            let parent = if parent.is_absolute() {
                                                parent.to_owned()
                                            } else {
                                                build_dir.join(parent)
                                            };
                                            self.watch(parent, RecursiveMode::NonRecursive);
                                        }
                                    }
                                    if self.saved.as_ref() == Some(&identity) {
                                        self.saved = None;
                                        finish_config_edit_refresh(app, identity, Ok(variable));
                                    } else {
                                        let _ = update(
                                            app,
                                            config_variable_loaded_action(identity, variable),
                                        );
                                    }
                                }
                                Err(message) => {
                                    let _ = update(
                                        app,
                                        Action::VariableDetailFailed { identity, message },
                                    );
                                }
                            }
                        }
                    }
                    Err(error) => {
                        let message = format!("{error:#}");
                        if let Some(identity) = identity {
                            let _ = update(app, Action::VariableDetailFailed { identity, message });
                        } else {
                            app.notification =
                                Some(format!("Configuration refresh failed: {message}"));
                        }
                    }
                    Ok(None) => {}
                }
            } else if let Some(identity) = identity {
                app.variable_detail_loading.remove(&identity);
            }
            changed = true;
        }
        // Only inspect while visible or explicitly requested. Never hold a connection idle.
        if self.worker.is_none()
            && self.dirty
            && (visible || self.requested.is_some())
            && app.active_dialog().is_none()
            && !matches!(
                app.build.status,
                yoctui_model::BuildStatus::Parsing
                    | yoctui_model::BuildStatus::Running
                    | yoctui_model::BuildStatus::Cancelling
            )
            && self.ready_at.is_none_or(|ready| Instant::now() >= ready)
        {
            let identity = self.requested.take().or_else(|| selected_identity(app));
            if let Some(identity) = &identity {
                app.variable_detail_loading.insert(identity.clone());
                app.variable_detail_errors.remove(identity);
            }
            self.running = identity.clone();
            self.running_revision = self.revision;
            self.dirty = false;
            self.worker = Some(crate::daemon_metadata::StartupMetadata::spawn(
                move |cancel| query::read(context.0, context.1, identity, cancel),
            ));
            changed = true;
        }
        changed
    }

    pub(crate) async fn shutdown(&mut self) {
        if let Some(mut worker) = self.worker.take() {
            worker.shutdown().await;
        }
        self.running = None;
    }
}

fn selected_identity(app: &App) -> Option<VariableIdentity> {
    let query = app.metadata_query.to_ascii_lowercase();
    let mut names = app
        .workspace
        .variables
        .iter()
        .filter(|(name, value)| {
            query.is_empty()
                || name.to_ascii_lowercase().contains(&query)
                || value.to_ascii_lowercase().contains(&query)
        })
        .map(|(name, _)| name)
        .collect::<Vec<_>>();
    names.sort();
    names
        .get(app.config_selection)
        .map(|name| VariableIdentity {
            name: (*name).clone(),
            recipe: app.config_scope.clone(),
        })
}

fn apply_snapshot(app: &mut App, workspace: yoctui_model::Workspace) {
    let selected = selected_identity(app).map(|identity| identity.name);
    if app.workspace.variables != workspace.variables {
        app.variable_details.clear();
        app.variable_detail_errors.clear();
    }
    app.workspace.variables = workspace.variables;
    app.workspace.variable_provenance = workspace.variable_provenance;
    app.workspace.variable_provenance_chain = workspace.variable_provenance_chain;
    // Filtering may change when a value changes; keep the same selected variable if possible.
    let query = app.metadata_query.to_ascii_lowercase();
    let mut names = app
        .workspace
        .variables
        .iter()
        .filter(|(name, value)| {
            query.is_empty()
                || name.to_ascii_lowercase().contains(&query)
                || value.to_ascii_lowercase().contains(&query)
        })
        .map(|(name, _)| name)
        .collect::<Vec<_>>();
    names.sort();
    app.config_selection = selected
        .and_then(|name| names.iter().position(|candidate| **candidate == name))
        .unwrap_or_else(|| app.config_selection.min(names.len().saturating_sub(1)));
}

#[cfg(test)]
#[path = "../tests/live_configuration.rs"]
mod tests;
