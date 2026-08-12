use crate::{
    BitBakeServerAdapter, BitBakeServerCapability, BitBakeServerController,
    BitBakeServerControllerError, BitBakeServerLifecycle,
};
use async_trait::async_trait;
use thiserror::Error;
use yoctui_model::{BitBakeRestartAffectedJob, BitBakeRestartConfirmation, BitBakeRestartPreview};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitBakeRestartMetadata {
    pub bitbake_version: Option<String>,
    pub machine: Option<String>,
    pub images: Vec<String>,
}

#[async_trait]
pub trait BitBakeMetadataRefresher: Send {
    async fn refresh(&mut self) -> Result<BitBakeRestartMetadata, String>;
}

pub struct BitBakeRestartCoordinator<A, R> {
    controller: BitBakeServerController<A>,
    refresher: R,
}

impl<A: BitBakeServerAdapter, R: BitBakeMetadataRefresher> BitBakeRestartCoordinator<A, R> {
    pub fn new(controller: BitBakeServerController<A>, refresher: R) -> Self {
        Self {
            controller,
            refresher,
        }
    }

    pub fn preview(
        &self,
        affected_jobs: Vec<BitBakeRestartAffectedJob>,
    ) -> Result<BitBakeRestartPreview, BitBakeRestartError> {
        let state = self.controller.state();
        if !matches!(
            state.lifecycle,
            BitBakeServerLifecycle::Available | BitBakeServerLifecycle::Connected
        ) {
            return Err(BitBakeRestartError::UnsafeLifecycle(state.lifecycle));
        }
        let observation = state
            .observation
            .as_ref()
            .ok_or(BitBakeRestartError::MissingServer)?;
        for capability in [
            BitBakeServerCapability::ServerStop,
            BitBakeServerCapability::ServerRestart,
        ] {
            if !observation.capabilities.contains(&capability) {
                return Err(BitBakeRestartError::MissingCapability(capability));
            }
        }
        if affected_jobs.len() > yoctui_model::MAX_BITBAKE_RESTART_AFFECTED_JOBS {
            return Err(BitBakeRestartError::TooManyAffectedJobs);
        }
        Ok(BitBakeRestartPreview {
            controller_generation: state.generation,
            server_identity: observation.server_identity.clone(),
            affected_jobs,
        })
    }

    pub async fn restart(
        &mut self,
        preview: &BitBakeRestartPreview,
        current_affected_jobs: Vec<BitBakeRestartAffectedJob>,
        confirmation: Option<&BitBakeRestartConfirmation>,
    ) -> Result<BitBakeRestartMetadata, BitBakeRestartError> {
        let current = self.preview(current_affected_jobs)?;
        if &current != preview {
            return Err(BitBakeRestartError::StalePreview);
        }
        if preview.requires_confirmation()
            && !confirmation.is_some_and(|value| preview.validate_confirmation(value))
        {
            return Err(BitBakeRestartError::ConfirmationRequired);
        }
        self.controller.restart().await?;
        self.refresher
            .refresh()
            .await
            .map_err(BitBakeRestartError::MetadataRefresh)
    }

    pub fn controller(&self) -> &BitBakeServerController<A> {
        &self.controller
    }

    pub fn into_parts(self) -> (BitBakeServerController<A>, R) {
        (self.controller, self.refresher)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BitBakeRestartError {
    #[error("BitBake restart is unsafe while the controller is {0:?}")]
    UnsafeLifecycle(BitBakeServerLifecycle),
    #[error("BitBake server identity is unavailable")]
    MissingServer,
    #[error("BitBake restart requires server capability {0:?}")]
    MissingCapability(BitBakeServerCapability),
    #[error("too many active jobs to preview safely")]
    TooManyAffectedJobs,
    #[error("BitBake restart preview is stale")]
    StalePreview,
    #[error("exact confirmation is required while jobs are active")]
    ConfirmationRequired,
    #[error(transparent)]
    Controller(#[from] BitBakeServerControllerError),
    #[error("BitBake restarted but authoritative metadata refresh failed: {0}")]
    MetadataRefresh(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BitBakeServerAdapterError, BitBakeServerContext, BitBakeServerEndpoint,
        BitBakeServerObservation, BitBakeServerSession,
    };
    use std::{path::PathBuf, time::Duration};
    use yoctui_model::{BackgroundJobId, BitBakeRestartJobId};

    #[derive(Default)]
    struct Adapter {
        calls: Vec<&'static str>,
        connection: u64,
    }

    fn observation() -> BitBakeServerObservation {
        BitBakeServerObservation {
            endpoint: BitBakeServerEndpoint::UnixSocket("/work/build/bitbake.sock".into()),
            server_identity: "server-1".into(),
            version: Some("2.8.1".into()),
            capabilities: vec![
                BitBakeServerCapability::ServerStop,
                BitBakeServerCapability::ServerRestart,
            ],
        }
    }

    #[async_trait]
    impl BitBakeServerAdapter for Adapter {
        async fn detect(
            &mut self,
            _: &BitBakeServerContext,
        ) -> Result<Option<BitBakeServerObservation>, BitBakeServerAdapterError> {
            Ok(Some(observation()))
        }
        async fn start(
            &mut self,
            _: &BitBakeServerContext,
        ) -> Result<BitBakeServerObservation, BitBakeServerAdapterError> {
            self.calls.push("start");
            Ok(observation())
        }
        async fn connect(
            &mut self,
            _: &BitBakeServerContext,
            observation: &BitBakeServerObservation,
        ) -> Result<BitBakeServerSession, BitBakeServerAdapterError> {
            self.calls.push("connect");
            self.connection += 1;
            Ok(BitBakeServerSession {
                server_identity: observation.server_identity.clone(),
                connection_identity: format!("connection-{}", self.connection),
            })
        }
        async fn disconnect(
            &mut self,
            _: &BitBakeServerSession,
        ) -> Result<(), BitBakeServerAdapterError> {
            self.calls.push("disconnect");
            Ok(())
        }
        async fn stop(
            &mut self,
            _: &BitBakeServerContext,
            _: &BitBakeServerObservation,
        ) -> Result<(), BitBakeServerAdapterError> {
            self.calls.push("stop");
            Ok(())
        }
    }

    #[derive(Default)]
    struct Refresher {
        calls: usize,
    }
    #[async_trait]
    impl BitBakeMetadataRefresher for Refresher {
        async fn refresh(&mut self) -> Result<BitBakeRestartMetadata, String> {
            self.calls += 1;
            Ok(BitBakeRestartMetadata {
                bitbake_version: Some("2.8.1".into()),
                machine: Some("qemux86-64".into()),
                images: vec!["core-image-minimal".into()],
            })
        }
    }

    async fn coordinator() -> BitBakeRestartCoordinator<Adapter, Refresher> {
        let context = BitBakeServerContext {
            source_dir: PathBuf::from("/work/poky"),
            build_dir: PathBuf::from("/work/build"),
            init_script: PathBuf::from("/work/poky/oe-init-build-env"),
        };
        let mut controller =
            BitBakeServerController::new(Adapter::default(), context, Duration::from_secs(1))
                .unwrap();
        controller.start().await.unwrap();
        controller.connect().await.unwrap();
        BitBakeRestartCoordinator::new(controller, Refresher::default())
    }

    fn affected() -> Vec<BitBakeRestartAffectedJob> {
        vec![BitBakeRestartAffectedJob {
            id: BitBakeRestartJobId::Background(BackgroundJobId(7)),
            title: "image build".into(),
            status: "Running".into(),
        }]
    }

    #[tokio::test]
    async fn bitbake_restart_refuses_active_jobs_without_exact_confirmation() {
        let mut coordinator = coordinator().await;
        let preview = coordinator.preview(affected()).unwrap();
        assert_eq!(
            coordinator.restart(&preview, affected(), None).await,
            Err(BitBakeRestartError::ConfirmationRequired)
        );
        let mut stale = preview.confirmation();
        stale.controller_generation += 1;
        assert_eq!(
            coordinator
                .restart(&preview, affected(), Some(&stale))
                .await,
            Err(BitBakeRestartError::ConfirmationRequired)
        );
        assert_eq!(
            coordinator
                .restart(&preview, Vec::new(), Some(&preview.confirmation()))
                .await,
            Err(BitBakeRestartError::StalePreview)
        );
    }

    #[tokio::test]
    async fn bitbake_restart_disconnects_stops_starts_reconnects_and_refreshes() {
        let mut coordinator = coordinator().await;
        let preview = coordinator.preview(affected()).unwrap();
        let metadata = coordinator
            .restart(&preview, affected(), Some(&preview.confirmation()))
            .await
            .unwrap();
        assert_eq!(metadata.machine.as_deref(), Some("qemux86-64"));
        assert_eq!(
            coordinator.controller().state().lifecycle,
            BitBakeServerLifecycle::Connected
        );
        let (controller, refresher) = coordinator.into_parts();
        assert_eq!(refresher.calls, 1);
        assert_eq!(
            controller.into_adapter().calls,
            vec!["start", "connect", "disconnect", "stop", "start", "connect"]
        );
    }
}
