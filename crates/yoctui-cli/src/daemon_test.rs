use std::path::PathBuf;
use tokio::sync::mpsc;
use yoctui_bitbake::{
    TestResultAdapter, TestResultImportResponse, TestRunnerAdapter, TestRunnerEvent, TestRunnerJob,
};
use yoctui_model::{PtestCapability, TestFamily, TestOutputStream, TestSelftestRequest};
use yoctui_protocol::daemon::{
    DaemonTestResultRecord, DaemonTestResultSnapshot, DaemonTestSelftestRequest, JobId,
};

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
    Snapshot {
        job_id: JobId,
        snapshot: DaemonTestResultSnapshot,
        authoritative: Option<TestResultImportResponse>,
    },
}

pub struct DaemonTestSupervisor {
    next: u64,
    active: std::collections::HashMap<u64, mpsc::UnboundedSender<()>>,
    tx: mpsc::UnboundedSender<DaemonTestEvent>,
    rx: mpsc::UnboundedReceiver<DaemonTestEvent>,
    pub cache: DaemonTestResultCache,
}
impl Default for DaemonTestSupervisor {
    fn default() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            next: 1,
            active: Default::default(),
            tx,
            rx,
            cache: DaemonTestResultCache::default(),
        }
    }
}
impl DaemonTestSupervisor {
    pub fn import_results(&mut self, generation: u64, roots: Vec<String>) -> Result<JobId, String> {
        let request = yoctui_model::TestResultImportRequest::new(
            generation,
            roots.into_iter().map(PathBuf::from).collect(),
        )
        .map_err(str::to_owned)?;
        let adapter = TestResultAdapter::new(Vec::new());
        let id = JobId(self.next);
        self.next += 1;
        let tx = self.tx.clone();
        tokio::task::spawn_blocking(move || {
            let result = adapter.import(&request);
            let (snapshot, authoritative) = match result {
                Ok(response) => (
                    DaemonTestResultSnapshot {
                        generation,
                        records: response
                            .records
                            .clone()
                            .into_iter()
                            .map(|record| DaemonTestResultRecord {
                                identity: format!("{:?}", record.identity),
                                outcome: format!(
                                    "family={:?}; suites={}",
                                    record.family,
                                    record.suites.len()
                                ),
                                duration_ms: record
                                    .duration
                                    .map(|duration| duration.as_millis() as u64),
                                log_path: None,
                            })
                            .collect(),
                        limitations: response.limitations.clone(),
                        complete: true,
                    }
                    .bounded(),
                    Some(response),
                ),
                Err(error) => (
                    DaemonTestResultSnapshot {
                        generation,
                        records: Vec::new(),
                        limitations: vec![error.to_string()],
                        complete: false,
                    },
                    None,
                ),
            };
            let _ = tx.send(DaemonTestEvent::Snapshot {
                job_id: id,
                snapshot,
                authoritative,
            });
        });
        Ok(id)
    }
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
        if let DaemonTestEvent::Snapshot {
            snapshot,
            authoritative,
            ..
        } = &e
        {
            self.cache.insert(snapshot.clone());
            if let Some(response) = authoritative.clone() {
                self.cache.insert_authoritative(response);
            }
        }
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

#[derive(Debug, Default)]
pub struct DaemonTestResultCache {
    snapshots: std::collections::BTreeMap<u64, DaemonTestResultSnapshot>,
    authoritative: std::collections::BTreeMap<u64, TestResultImportResponse>,
}

impl DaemonTestResultCache {
    const MAX_GENERATIONS: usize = 8;

    pub fn insert(&mut self, snapshot: DaemonTestResultSnapshot) {
        self.snapshots
            .insert(snapshot.generation, snapshot.bounded());
        while self.snapshots.len() > Self::MAX_GENERATIONS {
            if let Some(generation) = self.snapshots.keys().next().copied() {
                self.snapshots.remove(&generation);
            }
        }
    }

    pub fn insert_authoritative(&mut self, response: TestResultImportResponse) {
        self.authoritative
            .insert(response.request.generation, response);
        while self.authoritative.len() > Self::MAX_GENERATIONS {
            if let Some(generation) = self.authoritative.keys().next().copied() {
                self.authoritative.remove(&generation);
            }
        }
    }

    #[allow(dead_code)]
    pub fn get(&self, generation: u64) -> Option<&DaemonTestResultSnapshot> {
        self.snapshots.get(&generation)
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

    #[test]
    fn daemon_test_result_cache_replaces_and_bounds_generations() {
        let mut cache = DaemonTestResultCache::default();
        for generation in 1..=10 {
            cache.insert(DaemonTestResultSnapshot {
                generation,
                records: Vec::new(),
                limitations: Vec::new(),
                complete: true,
            });
        }
        assert!(cache.get(1).is_none());
        assert!(cache.get(10).is_some());
    }
}
