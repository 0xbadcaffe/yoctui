//! Client-local deploy watches. Never recurse into builds or interrupt active work.
use super::*;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::mpsc::{Receiver, sync_channel};

#[derive(Default)]
pub(crate) struct ImageArtifactRefresh {
    context: Option<(PathBuf, String)>,
    watcher: Option<RecommendedWatcher>,
    events: Option<Receiver<notify::Result<notify::Event>>>,
    watched: Option<PathBuf>,
    retry_at: Option<Instant>,
    due_at: Option<Instant>,
    visible: bool,
    building: bool,
}

impl ImageArtifactRefresh {
    fn watch(&mut self, deploy: &Path, now: Instant) {
        self.retry_at = Some(now + Duration::from_secs(5));
        // Before a first build the deploy directory may not exist yet. Watch its
        // nearest existing ancestor without recursively watching tmp/work.
        let path = deploy.ancestors().find(|path| path.is_dir());
        if path == self.watched.as_deref() && self.watcher.is_some() {
            return;
        }
        self.watcher = None;
        self.events = None;
        self.watched = None;
        let Some(path) = path else { return };
        let (send, events) = sync_channel(64);
        let watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
            if event.as_ref().map_or(true, |event| {
                matches!(
                    event.kind,
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
                )
            }) {
                // Bounded even during a large deploy. Reads/access events are
                // ignored so scanning our own inventory cannot trigger a loop.
                let _ = send.try_send(event);
            }
        });
        match watcher {
            Ok(mut watcher) => match watcher.watch(path, RecursiveMode::NonRecursive) {
                Ok(()) => {
                    self.watcher = Some(watcher);
                    self.events = Some(events);
                    self.watched = Some(path.to_owned());
                }
                Err(error) => tracing::debug!(%error, "deploy watch unavailable"),
            },
            Err(error) => tracing::debug!(%error, "deploy watch unavailable"),
        }
    }

    pub(crate) fn poll(&mut self, app: &mut App, busy: bool, now: Instant) -> Option<Effect> {
        let context = app
            .workspace
            .variables
            .get("DEPLOY_DIR_IMAGE")
            .map(PathBuf::from)
            .zip(app.workspace.variables.get("MACHINE").cloned())
            .filter(|(path, machine)| {
                path.is_absolute() && !machine.is_empty() && !app.is_offline()
            });
        if self.context != context {
            *self = Self::default();
            self.context = context;
        }
        let (deploy, _) = self.context.clone()?;
        let visible = app.screen == Screen::Images;
        let loading = matches!(
            app.image_artifacts,
            ImageArtifactInventoryState::Loading { .. }
        );
        if visible && !self.visible && !loading {
            self.due_at = Some(now);
        }
        self.visible = visible;
        let building = matches!(
            app.build.status,
            yoctui_model::BuildStatus::LoadingWorkspace
                | yoctui_model::BuildStatus::Parsing
                | yoctui_model::BuildStatus::Running
                | yoctui_model::BuildStatus::Cancelling
        );
        if self.building && !building {
            self.due_at = Some(now + Duration::from_millis(500));
        }
        self.building = building;
        if visible && self.retry_at.is_none_or(|due| now >= due) {
            let previous = self.watched.clone();
            self.watch(&deploy, now);
            if previous != self.watched || self.watcher.is_none() {
                self.due_at = Some(now + Duration::from_millis(500));
            }
        }
        let (dirty, rearm) = self.events.as_ref().map_or((false, false), |events| {
            events
                .try_iter()
                .take(128)
                .fold((false, false), |(_, rearm), event| {
                    let lost = event.as_ref().map_or(true, |event| {
                        matches!(
                            event.kind,
                            EventKind::Remove(_)
                                | EventKind::Modify(notify::event::ModifyKind::Name(_))
                        ) && self
                            .watched
                            .as_ref()
                            .is_some_and(|path| event.paths.contains(path))
                    });
                    (true, rearm || lost)
                })
        });
        if rearm {
            self.watcher = None;
            self.events = None;
            self.watched = None;
        }
        if dirty {
            self.due_at = Some(now + Duration::from_millis(500));
            // Re-arm a removed/replaced directory or advance an ancestor watch.
            self.retry_at = Some(now);
        }
        if visible
            && !busy
            && !loading
            && !building
            && app.active_dialog().is_none()
            && self.due_at.is_some_and(|due| now >= due)
        {
            self.due_at = None;
            return update(app, Action::RefreshImageArtifactInventory);
        }
        None
    }
}
