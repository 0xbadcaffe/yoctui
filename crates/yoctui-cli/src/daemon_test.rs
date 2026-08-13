use std::path::PathBuf;
use tokio::sync::mpsc;
use yoctui_bitbake::{TestRunnerAdapter, TestRunnerEvent, TestRunnerJob};
use yoctui_model::{PtestCapability, TestFamily, TestOutputStream, TestSelftestRequest};
use yoctui_protocol::daemon::{DaemonTestSelftestRequest, JobId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaemonTestEvent {
    Started {
        job_id: JobId,
        session_id: u64,
    },
    Output {
        job_id: JobId,
        session_id: u64,
        stream: TestOutputStream,
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
    TimedOut {
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

pub struct DaemonTestSupervisor {
    next: u64,
    active: std::collections::HashMap<u64, mpsc::UnboundedSender<()>>,
    tx: mpsc::UnboundedSender<DaemonTestEvent>,
    rx: mpsc::UnboundedReceiver<DaemonTestEvent>,
}
impl Default for DaemonTestSupervisor {
    fn default() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            next: 1,
            active: Default::default(),
            tx,
            rx,
        }
    }
}
impl DaemonTestSupervisor {
    pub fn start(
        &mut self,
        session_id: u64,
        wire: DaemonTestSelftestRequest,
        build: String,
        paths: Vec<String>,
    ) -> Result<JobId, String> {
        let family = match wire.family.as_str() {
            "OeSelftest" => TestFamily::OeSelftest,
            "BitbakeSelftest" => TestFamily::BitbakeSelftest,
            _ => return Err("unsupported test family".into()),
        };
        let request = TestSelftestRequest::new(
            wire.executable.into(),
            family,
            wire.selector,
            wire.parallelism,
            wire.verbose,
            wire.skip_network,
        )
        .map_err(str::to_owned)?;
        let adapter = TestRunnerAdapter::new(
            build.into(),
            paths.into_iter().map(PathBuf::from).collect(),
            PtestCapability::default(),
        );
        let command = adapter.command(&request).map_err(|e| e.to_string())?;
        let id = JobId(self.next);
        self.next += 1;
        let (tx_cancel, mut rx_cancel) = mpsc::unbounded_channel();
        self.active.insert(session_id, tx_cancel);
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let mut runner = TestRunnerJob::new();
            if let Err(e) = runner.start(command).await {
                let _ = tx.send(DaemonTestEvent::Lost {
                    job_id: id,
                    session_id,
                    message: e.to_string(),
                });
                return;
            }
            loop {
                tokio::select! {c=rx_cancel.recv()=>{if c.is_some(){let _=runner.cancel().await;}},e=runner.next_event()=>{let event=match e{Ok(TestRunnerEvent::Started)=>DaemonTestEvent::Started{job_id:id,session_id},Ok(TestRunnerEvent::Output{stream,line,truncated})=>DaemonTestEvent::Output{job_id:id,session_id,stream,line,truncated},Ok(TestRunnerEvent::Completed{exit_code,..})=>DaemonTestEvent::Completed{job_id:id,session_id,exit_code},Ok(TestRunnerEvent::Failed{exit_code,..})=>DaemonTestEvent::Failed{job_id:id,session_id,exit_code},Ok(TestRunnerEvent::Cancelled{forced,exit_code})=>DaemonTestEvent::Cancelled{job_id:id,session_id,forced,exit_code},Ok(TestRunnerEvent::TimedOut{forced,exit_code})=>DaemonTestEvent::TimedOut{job_id:id,session_id,forced,exit_code},Ok(TestRunnerEvent::CancellationRejected{message})|Ok(TestRunnerEvent::Lost{message})=>DaemonTestEvent::Lost{job_id:id,session_id,message},Err(e)=>DaemonTestEvent::Lost{job_id:id,session_id,message:e.to_string()}};let terminal=matches!(event,DaemonTestEvent::Completed{..}|DaemonTestEvent::Failed{..}|DaemonTestEvent::Cancelled{..}|DaemonTestEvent::TimedOut{..}|DaemonTestEvent::Lost{..});if tx.send(event).is_err()||terminal{return;}}}
            }
        });
        Ok(id)
    }
    pub fn cancel(&mut self, session_id: u64) -> Result<(), String> {
        self.active
            .get(&session_id)
            .ok_or_else(|| format!("unknown test session {session_id}"))?
            .send(())
            .map_err(|_| "test session is no longer active".into())
    }
    pub fn try_event(&mut self) -> Option<DaemonTestEvent> {
        let e = self.rx.try_recv().ok()?;
        if matches!(
            e,
            DaemonTestEvent::Completed { .. }
                | DaemonTestEvent::Failed { .. }
                | DaemonTestEvent::Cancelled { .. }
                | DaemonTestEvent::TimedOut { .. }
                | DaemonTestEvent::Lost { .. }
        ) {
            let id = match &e {
                DaemonTestEvent::Completed { session_id, .. }
                | DaemonTestEvent::Failed { session_id, .. }
                | DaemonTestEvent::Cancelled { session_id, .. }
                | DaemonTestEvent::TimedOut { session_id, .. }
                | DaemonTestEvent::Lost { session_id, .. } => *session_id,
                _ => 0,
            };
            self.active.remove(&id);
        }
        Some(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn client_runtime_test_session_rejects_unknown_family() {
        let mut s = DaemonTestSupervisor::default();
        let result = s.start(
            1,
            DaemonTestSelftestRequest {
                executable: "/tmp/oe-selftest".into(),
                family: "unknown".into(),
                selector: None,
                parallelism: 1,
                verbose: false,
                skip_network: false,
            },
            "/tmp/build".into(),
            Vec::new(),
        );
        assert!(result.is_err());
    }
}
