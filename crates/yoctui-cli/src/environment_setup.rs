//! One in-flight blocking scan plus one latest request; no unbounded worker fanout.
use std::path::PathBuf;
use yoctui_model::{Action, App, Dialog, Effect, EnvironmentDirectory, EnvironmentSetupAction};

type Scan = (u64, PathBuf, bool);
type ScanResult = (u64, Result<EnvironmentDirectory, String>);

#[derive(Default)]
pub(crate) struct EnvironmentBrowserIo {
    worker: Option<tokio::task::JoinHandle<ScanResult>>,
    queued: Option<Scan>,
    active_request: u64,
}

impl EnvironmentBrowserIo {
    pub fn submit(&mut self, effect: Effect) {
        if let Effect::ReadEnvironmentDirectory {
            request,
            path,
            initial,
        } = effect
        {
            self.queued = Some((request, path, initial));
            self.start();
        }
    }

    fn start(&mut self) {
        if self.worker.is_some() {
            return;
        }
        if let Some((request, path, initial)) = self.queued.take() {
            self.active_request = request;
            let fallback = yoctui_app::environment_browser_fallback();
            self.worker = Some(tokio::task::spawn_blocking(move || {
                (
                    request,
                    yoctui_app::read_environment_directory(&path, initial, &fallback),
                )
            }));
        }
    }

    pub async fn poll(&mut self, app: &mut App) -> bool {
        if !self
            .worker
            .as_ref()
            .is_some_and(|worker| worker.is_finished())
        {
            return false;
        }
        let result = self.worker.take().expect("finished worker").await;
        let (request, result) = result.unwrap_or_else(|error| {
            (
                self.active_request,
                Err(format!("Directory scan failed: {error}")),
            )
        });
        let visible = matches!(app.active_dialog(), Some(Dialog::EnvironmentSetup(setup)) if setup.browser.as_ref().is_some_and(|b| b.request == request));
        yoctui_model::update(
            app,
            Action::EnvironmentSetup(EnvironmentSetupAction::DirectoryLoaded { request, result }),
        );
        self.start();
        visible
    }
}

#[cfg(test)]
#[path = "tests/environment_setup/mod.rs"]
mod tests;
