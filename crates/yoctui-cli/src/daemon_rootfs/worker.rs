use anyhow::{Context, Result};
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::{Semaphore, oneshot};
use yoctui_model::DaemonCompatibilitySnapshot;
use yoctui_protocol::{
    daemon::{DaemonInstanceId, RequestId},
    rootfs::{RootfsSourcesData, RootfsSourcesRequestData},
};

use super::{QUERY_TIMEOUT, authority_validation::validate_authority, source_acquisition::acquire};

pub struct PendingQuery {
    pub request_id: RequestId,
    pub query: RootfsSourcesRequestData,
    cancel: Option<oneshot::Sender<()>>,
    result: oneshot::Receiver<Result<RootfsSourcesData>>,
    worker: tokio::task::JoinHandle<()>,
}

impl PendingQuery {
    pub fn start(
        request_id: RequestId,
        query: RootfsSourcesRequestData,
        instance: DaemonInstanceId,
        compatibility: DaemonCompatibilitySnapshot,
        environment: BTreeMap<String, String>,
        permit: Arc<Semaphore>,
    ) -> Result<Self> {
        let build = validate_authority(&query, instance, &compatibility)?;
        let permit = permit
            .try_acquire_owned()
            .context("rootfs metadata query is already running")?;
        let (cancel, cancelled) = oneshot::channel();
        let (send, result) = oneshot::channel();
        let worker_query = query.clone();
        let worker = tokio::spawn(async move {
            let _permit = permit;
            let result = acquire(
                worker_query,
                build,
                compatibility,
                environment,
                cancelled,
                QUERY_TIMEOUT,
            )
            .await;
            let _ = send.send(result);
        });
        Ok(Self {
            request_id,
            query,
            cancel: Some(cancel),
            result,
            worker,
        })
    }

    pub fn try_result(&mut self) -> Option<Result<RootfsSourcesData>> {
        match self.result.try_recv() {
            Ok(result) => Some(result),
            Err(oneshot::error::TryRecvError::Empty) => None,
            Err(oneshot::error::TryRecvError::Closed) => {
                Some(Err(anyhow::anyhow!("rootfs metadata worker was lost")))
            }
        }
    }

    pub async fn shutdown(&mut self) {
        self.cancel();
        let _ = (&mut self.worker).await;
    }

    pub fn cancel(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            let _ = cancel.send(());
        }
    }

    pub fn is_finished(&self) -> bool {
        self.worker.is_finished()
    }
}

impl Drop for PendingQuery {
    fn drop(&mut self) {
        self.cancel();
    }
}
