use super::*;
use std::collections::VecDeque;

#[derive(Default)]
struct FakeAdapter {
    calls: Vec<String>,
    detections: VecDeque<Option<BitBakeServerObservation>>,
    fail: Option<&'static str>,
    delay: Duration,
    connection: u64,
}

impl FakeAdapter {
    fn observation() -> BitBakeServerObservation {
        BitBakeServerObservation {
            endpoint: BitBakeServerEndpoint::UnixSocket(PathBuf::from("/work/build/bitbake.sock")),
            server_identity: "server-1".into(),
            version: Some("2.8.1".into()),
            capabilities: vec![
                BitBakeServerCapability::Metadata,
                BitBakeServerCapability::Metadata,
                BitBakeServerCapability::BuildControl,
            ],
        }
    }

    async fn step(&mut self, name: &'static str) -> Result<(), BitBakeServerAdapterError> {
        self.calls.push(name.into());
        if !self.delay.is_zero() {
            tokio::time::sleep(self.delay).await;
        }
        if self.fail == Some(name) {
            return Err(BitBakeServerAdapterError::new(format!("{name} failed")));
        }
        Ok(())
    }
}

#[async_trait]
impl BitBakeServerAdapter for FakeAdapter {
    async fn detect(
        &mut self,
        _context: &BitBakeServerContext,
    ) -> Result<Option<BitBakeServerObservation>, BitBakeServerAdapterError> {
        self.step("detect").await?;
        Ok(self
            .detections
            .pop_front()
            .unwrap_or_else(|| Some(Self::observation())))
    }

    async fn start(
        &mut self,
        _context: &BitBakeServerContext,
    ) -> Result<BitBakeServerObservation, BitBakeServerAdapterError> {
        self.step("start").await?;
        Ok(Self::observation())
    }

    async fn connect(
        &mut self,
        _context: &BitBakeServerContext,
        observation: &BitBakeServerObservation,
    ) -> Result<BitBakeServerSession, BitBakeServerAdapterError> {
        self.step("connect").await?;
        self.connection += 1;
        Ok(BitBakeServerSession {
            server_identity: observation.server_identity.clone(),
            connection_identity: format!("connection-{}", self.connection),
        })
    }

    async fn disconnect(
        &mut self,
        _session: &BitBakeServerSession,
    ) -> Result<(), BitBakeServerAdapterError> {
        self.step("disconnect").await
    }

    async fn stop(
        &mut self,
        _context: &BitBakeServerContext,
        _observation: &BitBakeServerObservation,
    ) -> Result<(), BitBakeServerAdapterError> {
        self.step("stop").await
    }
}

fn context() -> BitBakeServerContext {
    BitBakeServerContext {
        source_dir: PathBuf::from("/work/poky"),
        build_dir: PathBuf::from("/work/poky/build"),
        init_script: PathBuf::from("/work/poky/oe-init-build-env"),
    }
}

mod server_controller_drives_typed_lifecycle_and_preserves_capabilities;

mod server_controller_reports_unavailable_failure_timeout_and_invalid_transitions;
