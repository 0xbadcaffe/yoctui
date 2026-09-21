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
        BitBakeServerController::new(Adapter::default(), context, Duration::from_secs(1)).unwrap();
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

mod bitbake_restart_refuses_active_jobs_without_exact_confirmation;

mod bitbake_restart_disconnects_stops_starts_reconnects_and_refreshes;
