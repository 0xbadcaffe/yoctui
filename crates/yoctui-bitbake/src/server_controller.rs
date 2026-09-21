include!("server_controller/types_and_adapter.rs");

include!("server_controller/lifecycle_operations.rs");

include!("server_controller/timeouts_and_errors.rs");

#[cfg(test)]
mod tests {
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
                endpoint: BitBakeServerEndpoint::UnixSocket(PathBuf::from(
                    "/work/build/bitbake.sock",
                )),
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

    #[tokio::test]
    async fn server_controller_drives_typed_lifecycle_and_preserves_capabilities() {
        let adapter = FakeAdapter::default();
        let mut controller =
            BitBakeServerController::new(adapter, context(), Duration::from_secs(1)).unwrap();
        assert_eq!(
            controller.detect().await.unwrap(),
            BitBakeDetection::Available
        );
        assert_eq!(
            controller
                .state()
                .observation
                .as_ref()
                .unwrap()
                .capabilities,
            vec![
                BitBakeServerCapability::Metadata,
                BitBakeServerCapability::BuildControl,
            ]
        );
        controller.connect().await.unwrap();
        assert_eq!(
            controller.state().lifecycle,
            BitBakeServerLifecycle::Connected
        );
        controller.reconnect().await.unwrap();
        assert_eq!(
            controller.state().connection_identity.as_deref(),
            Some("connection-2")
        );
        controller.restart().await.unwrap();
        assert_eq!(
            controller.state().lifecycle,
            BitBakeServerLifecycle::Connected
        );
        controller.stop().await.unwrap();
        assert_eq!(
            controller.state().lifecycle,
            BitBakeServerLifecycle::Unavailable
        );
        assert!(controller.state().generation > 0);
        assert_eq!(
            controller.into_adapter().calls,
            vec![
                "detect",
                "connect",
                "disconnect",
                "connect",
                "disconnect",
                "stop",
                "start",
                "connect",
                "disconnect",
                "stop",
            ]
        );
    }

    #[tokio::test]
    async fn server_controller_reports_unavailable_failure_timeout_and_invalid_transitions() {
        let mut unavailable = FakeAdapter::default();
        unavailable.detections.push_back(None);
        let mut controller =
            BitBakeServerController::new(unavailable, context(), Duration::from_secs(1)).unwrap();
        assert_eq!(
            controller.detect().await.unwrap(),
            BitBakeDetection::Unavailable
        );
        assert!(matches!(
            controller.connect().await,
            Err(BitBakeServerControllerError::InvalidTransition { .. })
        ));

        let failed = FakeAdapter {
            fail: Some("start"),
            ..FakeAdapter::default()
        };
        let mut controller =
            BitBakeServerController::new(failed, context(), Duration::from_secs(1)).unwrap();
        assert!(matches!(
            controller.start().await,
            Err(BitBakeServerControllerError::Adapter {
                operation: BitBakeServerOperation::Start,
                ..
            })
        ));
        assert_eq!(controller.state().lifecycle, BitBakeServerLifecycle::Failed);

        let slow = FakeAdapter {
            delay: Duration::from_millis(50),
            ..FakeAdapter::default()
        };
        let mut controller =
            BitBakeServerController::new(slow, context(), Duration::from_millis(5)).unwrap();
        assert_eq!(
            controller.detect().await,
            Err(BitBakeServerControllerError::Timeout(
                BitBakeServerOperation::Detect
            ))
        );
        assert_eq!(controller.state().lifecycle, BitBakeServerLifecycle::Failed);
    }
}
