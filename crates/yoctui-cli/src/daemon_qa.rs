use std::{collections::HashMap, path::PathBuf};
use tokio::sync::mpsc;
use yoctui_bitbake::{
    QaLayerCommandSpec, QaLayerJobRunner, QaLayerRunnerEvent, QaTaskCapabilityInput,
    QaTaskCapabilityInspector, QaTaskScopeInput,
};
use yoctui_model::{
    QaCheckId, QaLayerIdentity, QaLayerOperationId, QaLayerSessionId, QaOutputStream,
    RecipeIdentity,
};
use yoctui_protocol::daemon::{DaemonQaCapabilityInput, DaemonQaSnapshot, JobId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaemonQaEvent {
    Started {
        job_id: JobId,
        session_id: u64,
    },
    Output {
        job_id: JobId,
        session_id: u64,
        stream: QaOutputStream,
        line: String,
        truncated: bool,
    },
    Completed {
        job_id: JobId,
        session_id: u64,
        exit_code: Option<i32>,
    },
    Failed {
        job_id: JobId,
        session_id: u64,
        exit_code: Option<i32>,
    },
    Cancelled {
        job_id: JobId,
        session_id: u64,
        forced: bool,
        exit_code: Option<i32>,
    },
    Lost {
        job_id: JobId,
        session_id: u64,
        message: String,
    },
}

pub struct DaemonQaSupervisor {
    next_job_id: u64,
    active: HashMap<u64, mpsc::UnboundedSender<()>>,
    tx: mpsc::UnboundedSender<DaemonQaEvent>,
    rx: mpsc::UnboundedReceiver<DaemonQaEvent>,
}

impl Default for DaemonQaSupervisor {
    fn default() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            next_job_id: 1,
            active: HashMap::new(),
            tx,
            rx,
        }
    }
}

impl DaemonQaSupervisor {
    #[allow(clippy::too_many_arguments)]
    pub fn start(
        &mut self,
        session_id: u64,
        operation_id: u64,
        check_id: String,
        layer_name: String,
        layer_root: String,
        executable: String,
        arguments: Vec<String>,
        report_roots: Vec<String>,
    ) -> Result<JobId, String> {
        if session_id == 0 || self.active.contains_key(&session_id) {
            return Err("QA layer session is already active or invalid".into());
        }
        let session = QaLayerSessionId(session_id);
        let check = QaCheckId::new(check_id).map_err(str::to_owned)?;
        let layer = QaLayerIdentity::new(layer_name, layer_root.into()).map_err(str::to_owned)?;
        let command = QaLayerCommandSpec::from_paths(
            session,
            QaLayerOperationId(operation_id),
            check,
            layer,
            executable.into(),
            arguments,
            report_roots.into_iter().map(Into::into).collect(),
        )
        .map_err(|error| error.to_string())?;
        let job_id = JobId(self.next_job_id);
        self.next_job_id = self.next_job_id.saturating_add(1);
        let (cancel_tx, mut cancel_rx) = mpsc::unbounded_channel();
        self.active.insert(session_id, cancel_tx);
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let mut runner = QaLayerJobRunner::new();
            if let Err(error) = runner.start(command).await {
                let _ = tx.send(DaemonQaEvent::Lost {
                    job_id,
                    session_id,
                    message: error.to_string(),
                });
                return;
            }
            loop {
                tokio::select! {
                    cancel = cancel_rx.recv() => {
                        if cancel.is_some() { let _ = runner.cancel(session).await; }
                    }
                    event = runner.next_event() => {
                        let mapped = match event {
                            Ok(QaLayerRunnerEvent::Started { .. }) => DaemonQaEvent::Started { job_id, session_id },
                            Ok(QaLayerRunnerEvent::Output { stream, line, truncated, .. }) => DaemonQaEvent::Output { job_id, session_id, stream, line, truncated },
                            Ok(QaLayerRunnerEvent::Completed { exit_code, .. }) => DaemonQaEvent::Completed { job_id, session_id, exit_code },
                            Ok(QaLayerRunnerEvent::Failed { exit_code, .. }) | Ok(QaLayerRunnerEvent::TimedOut { exit_code, .. }) => DaemonQaEvent::Failed { job_id, session_id, exit_code },
                            Ok(QaLayerRunnerEvent::Cancelled { forced, exit_code, .. }) => DaemonQaEvent::Cancelled { job_id, session_id, forced, exit_code },
                            Ok(QaLayerRunnerEvent::CancellationRejected { message, .. }) | Ok(QaLayerRunnerEvent::Lost { message, .. }) => DaemonQaEvent::Lost { job_id, session_id, message },
                            Ok(QaLayerRunnerEvent::CancellationRequested { .. }) => continue,
                            Err(error) => DaemonQaEvent::Lost { job_id, session_id, message: error.to_string() },
                        };
                        let terminal = matches!(mapped, DaemonQaEvent::Completed { .. } | DaemonQaEvent::Failed { .. } | DaemonQaEvent::Cancelled { .. } | DaemonQaEvent::Lost { .. });
                        if tx.send(mapped).is_err() || terminal { break; }
                    }
                }
            }
        });
        Ok(job_id)
    }

    pub fn cancel(&mut self, session_id: u64) -> Result<(), String> {
        self.active
            .get(&session_id)
            .ok_or_else(|| format!("unknown QA layer session {session_id}"))?
            .send(())
            .map_err(|_| "QA layer session is no longer active".into())
    }

    pub fn try_event(&mut self) -> Option<DaemonQaEvent> {
        let event = self.rx.try_recv().ok()?;
        if matches!(
            event,
            DaemonQaEvent::Completed { .. }
                | DaemonQaEvent::Failed { .. }
                | DaemonQaEvent::Cancelled { .. }
                | DaemonQaEvent::Lost { .. }
        ) {
            let id = match &event {
                DaemonQaEvent::Completed { session_id, .. }
                | DaemonQaEvent::Failed { session_id, .. }
                | DaemonQaEvent::Cancelled { session_id, .. }
                | DaemonQaEvent::Lost { session_id, .. } => *session_id,
                _ => 0,
            };
            self.active.remove(&id);
        }
        Some(event)
    }
}

pub fn inspect(input: DaemonQaCapabilityInput) -> Result<DaemonQaSnapshot, String> {
    let selected = RecipeIdentity {
        name: input.selected_recipe_name.clone(),
        file: PathBuf::from(input.selected_recipe_file),
    };
    let scope = QaTaskScopeInput {
        identity: selected.clone(),
        reported_tasks: input.recipe_names,
        family_tasks: Vec::new(),
        is_kernel: false,
        report_roots: Vec::new(),
    };
    let request = QaTaskCapabilityInput {
        release: None,
        build_directory: PathBuf::from(input.build_directory),
        selected,
        scopes: vec![scope],
    };
    let response = QaTaskCapabilityInspector::new(request)
        .inspect()
        .map_err(|error| error.to_string())?;
    let snapshot = response.snapshot();
    Ok(DaemonQaSnapshot {
        generation: input.generation,
        capability: if response.is_partial() {
            "partial".into()
        } else {
            "available".into()
        },
        task_bindings: snapshot
            .checks
            .iter()
            .map(|check| format!("{:?}", check))
            .collect(),
        reports: input.report_roots,
        limitations: snapshot.limitations.clone(),
    }
    .bounded())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn client_runtime_qa_adapter_rejects_unsafe_scope() {
        let input = DaemonQaCapabilityInput {
            generation: 1,
            build_directory: "relative".into(),
            source_directory: None,
            layer_directories: Vec::new(),
            recipe_names: Vec::new(),
            report_roots: Vec::new(),
            selected_recipe_name: "recipe".into(),
            selected_recipe_file: "/tmp/recipe.bb".into(),
        };
        assert!(inspect(input).is_err());
    }

    #[test]
    fn client_runtime_qa_task_runner_rejects_invalid_request() {
        let mut supervisor = DaemonQaSupervisor::default();
        let result = supervisor.start(
            0,
            0,
            "invalid".into(),
            "layer".into(),
            "relative".into(),
            "/missing/yocto-check-layer".into(),
            vec!["relative".into()],
            Vec::new(),
        );
        assert!(result.is_err());
    }
}
