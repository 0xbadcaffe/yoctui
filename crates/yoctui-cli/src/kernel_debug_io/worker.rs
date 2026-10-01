use super::*;
use yoctui_model::{
    Action, App, Effect, KernelDebugAction, KernelDebugOperation, KernelDebugRequest,
    KernelDebugResult,
};

#[derive(Default)]
pub(crate) struct KernelDebugIo {
    running: Option<(u64, tokio::task::JoinHandle<Action>)>,
    queued: Option<KernelDebugRequest>,
}

impl Drop for KernelDebugIo {
    fn drop(&mut self) {
        if let Some((_, handle)) = self.running.take() {
            handle.abort();
        }
    }
}

impl KernelDebugIo {
    pub(crate) fn submit(&mut self, effect: Effect) {
        let Effect::KernelDebug(request) = effect else {
            return;
        };
        if self.running.is_some() {
            self.queued = Some(request);
            return;
        }
        let generation = request.generation;
        self.running = Some((
            generation,
            tokio::task::spawn_blocking(move || {
                let result = match request.operation {
                    KernelDebugOperation::Inspect => discover().map(KernelDebugResult::Tools),
                    KernelDebugOperation::Prepare { draft, tools } => {
                        if draft.tool == KernelDebugTool::KgdbSerial {
                            prepare(&draft, &tools).and_then(|request| {
                                let report =
                                    crate::kgdb_serial::validate_files(&draft.serial_spec(&tools)?)
                                        .map_err(|error| format!("{error:#}"))?;
                                Ok(KernelDebugResult::PreparedSerial { request, report })
                            })
                        } else {
                            prepare(&draft, &tools).map(KernelDebugResult::Prepared)
                        }
                    }
                };
                Action::KernelDebug(KernelDebugAction::Finished {
                    generation: request.generation,
                    result,
                })
            }),
        ));
    }

    pub(crate) async fn poll(&mut self, app: &mut App) -> bool {
        if !self
            .running
            .as_ref()
            .is_some_and(|(_, handle)| handle.is_finished())
        {
            return false;
        }
        let (generation, handle) = self.running.take().expect("finished worker present");
        match handle.await {
            Ok(action) => {
                let _ = yoctui_model::update(app, action);
            }
            Err(error) => {
                let _ = yoctui_model::update(
                    app,
                    Action::KernelDebug(KernelDebugAction::Finished {
                        generation,
                        result: Err(format!("Debug tool preparation failed: {error}")),
                    }),
                );
            }
        }
        if let Some(request) = self.queued.take() {
            self.submit(Effect::KernelDebug(request));
        }
        true
    }
}
