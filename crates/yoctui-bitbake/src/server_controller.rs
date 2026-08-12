use async_trait::async_trait;
use std::{path::PathBuf, time::Duration};
use thiserror::Error;

const MAX_SERVER_CAPABILITIES: usize = 128;
const MAX_IDENTITY_BYTES: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitBakeServerContext {
    pub source_dir: PathBuf,
    pub build_dir: PathBuf,
    pub init_script: PathBuf,
}

impl BitBakeServerContext {
    pub fn validate(&self) -> Result<(), BitBakeServerControllerError> {
        for (field, path) in [
            ("source directory", &self.source_dir),
            ("build directory", &self.build_dir),
            ("init script", &self.init_script),
        ] {
            if !path.is_absolute()
                || path
                    .components()
                    .any(|component| matches!(component, std::path::Component::ParentDir))
            {
                return Err(BitBakeServerControllerError::InvalidContext(format!(
                    "{field} must be an absolute normalized path"
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BitBakeServerEndpoint {
    UnixSocket(PathBuf),
    Managed(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BitBakeServerCapability {
    CommandChannel,
    EventStream,
    Metadata,
    BuildControl,
    Cancellation,
    ServerStop,
    ServerRestart,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitBakeServerObservation {
    pub endpoint: BitBakeServerEndpoint,
    pub server_identity: String,
    pub version: Option<String>,
    pub capabilities: Vec<BitBakeServerCapability>,
}

impl BitBakeServerObservation {
    fn validate(&self) -> Result<(), BitBakeServerControllerError> {
        if self.server_identity.is_empty() || self.server_identity.len() > MAX_IDENTITY_BYTES {
            return Err(BitBakeServerControllerError::InvalidAdapterData(
                "server identity is empty or oversized".into(),
            ));
        }
        if self.capabilities.len() > MAX_SERVER_CAPABILITIES {
            return Err(BitBakeServerControllerError::InvalidAdapterData(
                "server capability list is oversized".into(),
            ));
        }
        if let BitBakeServerEndpoint::UnixSocket(path) = &self.endpoint
            && (!path.is_absolute()
                || path
                    .components()
                    .any(|component| matches!(component, std::path::Component::ParentDir)))
        {
            return Err(BitBakeServerControllerError::InvalidAdapterData(
                "server socket must be an absolute normalized path".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitBakeServerSession {
    pub server_identity: String,
    pub connection_identity: String,
}

impl BitBakeServerSession {
    fn validate(
        &self,
        observation: &BitBakeServerObservation,
    ) -> Result<(), BitBakeServerControllerError> {
        if self.server_identity != observation.server_identity {
            return Err(BitBakeServerControllerError::InvalidAdapterData(
                "connected session belongs to a different server".into(),
            ));
        }
        if self.connection_identity.is_empty()
            || self.connection_identity.len() > MAX_IDENTITY_BYTES
        {
            return Err(BitBakeServerControllerError::InvalidAdapterData(
                "connection identity is empty or oversized".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitBakeServerLifecycle {
    Unknown,
    Detecting,
    Unavailable,
    Available,
    Starting,
    Connecting,
    Connected,
    Disconnecting,
    Stopping,
    Restarting,
    Recovering,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitBakeServerControllerState {
    pub lifecycle: BitBakeServerLifecycle,
    pub generation: u64,
    pub observation: Option<BitBakeServerObservation>,
    pub connection_identity: Option<String>,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitBakeServerOperation {
    Detect,
    Start,
    Connect,
    Disconnect,
    Stop,
    Restart,
    Reconnect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitBakeDetection {
    Available,
    Unavailable,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("{message}")]
pub struct BitBakeServerAdapterError {
    pub message: String,
}

impl BitBakeServerAdapterError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[async_trait]
pub trait BitBakeServerAdapter: Send {
    async fn detect(
        &mut self,
        context: &BitBakeServerContext,
    ) -> Result<Option<BitBakeServerObservation>, BitBakeServerAdapterError>;

    async fn start(
        &mut self,
        context: &BitBakeServerContext,
    ) -> Result<BitBakeServerObservation, BitBakeServerAdapterError>;

    async fn connect(
        &mut self,
        context: &BitBakeServerContext,
        observation: &BitBakeServerObservation,
    ) -> Result<BitBakeServerSession, BitBakeServerAdapterError>;

    async fn disconnect(
        &mut self,
        session: &BitBakeServerSession,
    ) -> Result<(), BitBakeServerAdapterError>;

    async fn stop(
        &mut self,
        context: &BitBakeServerContext,
        observation: &BitBakeServerObservation,
    ) -> Result<(), BitBakeServerAdapterError>;
}

pub struct BitBakeServerController<A> {
    adapter: A,
    context: BitBakeServerContext,
    operation_timeout: Duration,
    state: BitBakeServerControllerState,
    session: Option<BitBakeServerSession>,
}

impl<A: BitBakeServerAdapter> BitBakeServerController<A> {
    pub fn new(
        adapter: A,
        context: BitBakeServerContext,
        operation_timeout: Duration,
    ) -> Result<Self, BitBakeServerControllerError> {
        context.validate()?;
        if operation_timeout.is_zero() {
            return Err(BitBakeServerControllerError::InvalidTimeout);
        }
        Ok(Self {
            adapter,
            context,
            operation_timeout,
            state: BitBakeServerControllerState {
                lifecycle: BitBakeServerLifecycle::Unknown,
                generation: 0,
                observation: None,
                connection_identity: None,
                diagnostic: None,
            },
            session: None,
        })
    }

    pub fn state(&self) -> &BitBakeServerControllerState {
        &self.state
    }

    pub fn into_adapter(self) -> A {
        self.adapter
    }

    pub async fn detect(&mut self) -> Result<BitBakeDetection, BitBakeServerControllerError> {
        self.require_not_busy(BitBakeServerOperation::Detect)?;
        self.transition(BitBakeServerLifecycle::Detecting, None)?;
        let result =
            tokio::time::timeout(self.operation_timeout, self.adapter.detect(&self.context)).await;
        match result {
            Err(_) => self.fail_timeout(BitBakeServerOperation::Detect),
            Ok(Err(error)) => self.fail_adapter(BitBakeServerOperation::Detect, error),
            Ok(Ok(Some(mut observation))) => {
                observation.capabilities.sort();
                observation.capabilities.dedup();
                observation.validate()?;
                self.state.observation = Some(observation);
                self.session = None;
                self.transition(BitBakeServerLifecycle::Available, None)?;
                Ok(BitBakeDetection::Available)
            }
            Ok(Ok(None)) => {
                self.state.observation = None;
                self.session = None;
                self.transition(BitBakeServerLifecycle::Unavailable, None)?;
                Ok(BitBakeDetection::Unavailable)
            }
        }
    }

    pub async fn start(&mut self) -> Result<(), BitBakeServerControllerError> {
        self.require_lifecycle(
            BitBakeServerOperation::Start,
            &[
                BitBakeServerLifecycle::Unknown,
                BitBakeServerLifecycle::Unavailable,
                BitBakeServerLifecycle::Failed,
            ],
        )?;
        self.transition(BitBakeServerLifecycle::Starting, None)?;
        let result =
            tokio::time::timeout(self.operation_timeout, self.adapter.start(&self.context)).await;
        match result {
            Err(_) => self.fail_timeout(BitBakeServerOperation::Start),
            Ok(Err(error)) => self.fail_adapter(BitBakeServerOperation::Start, error),
            Ok(Ok(mut observation)) => {
                observation.capabilities.sort();
                observation.capabilities.dedup();
                observation.validate()?;
                self.state.observation = Some(observation);
                self.session = None;
                self.transition(BitBakeServerLifecycle::Available, None)
            }
        }
    }

    pub async fn connect(&mut self) -> Result<(), BitBakeServerControllerError> {
        self.require_lifecycle(
            BitBakeServerOperation::Connect,
            &[BitBakeServerLifecycle::Available],
        )?;
        let observation = self
            .state
            .observation
            .clone()
            .ok_or(BitBakeServerControllerError::MissingObservation)?;
        self.transition(BitBakeServerLifecycle::Connecting, None)?;
        let result = tokio::time::timeout(
            self.operation_timeout,
            self.adapter.connect(&self.context, &observation),
        )
        .await;
        match result {
            Err(_) => self.fail_timeout(BitBakeServerOperation::Connect),
            Ok(Err(error)) => self.fail_adapter(BitBakeServerOperation::Connect, error),
            Ok(Ok(session)) => {
                session.validate(&observation)?;
                self.state.connection_identity = Some(session.connection_identity.clone());
                self.session = Some(session);
                self.transition(BitBakeServerLifecycle::Connected, None)
            }
        }
    }

    pub async fn disconnect(&mut self) -> Result<(), BitBakeServerControllerError> {
        self.require_lifecycle(
            BitBakeServerOperation::Disconnect,
            &[BitBakeServerLifecycle::Connected],
        )?;
        let session = self
            .session
            .clone()
            .ok_or(BitBakeServerControllerError::MissingSession)?;
        self.transition(BitBakeServerLifecycle::Disconnecting, None)?;
        let result =
            tokio::time::timeout(self.operation_timeout, self.adapter.disconnect(&session)).await;
        match result {
            Err(_) => self.fail_timeout(BitBakeServerOperation::Disconnect),
            Ok(Err(error)) => self.fail_adapter(BitBakeServerOperation::Disconnect, error),
            Ok(Ok(())) => {
                self.session = None;
                self.state.connection_identity = None;
                self.transition(BitBakeServerLifecycle::Available, None)
            }
        }
    }

    pub async fn stop(&mut self) -> Result<(), BitBakeServerControllerError> {
        self.require_lifecycle(
            BitBakeServerOperation::Stop,
            &[
                BitBakeServerLifecycle::Available,
                BitBakeServerLifecycle::Connected,
            ],
        )?;
        if self.session.is_some() {
            self.disconnect().await?;
        }
        let observation = self
            .state
            .observation
            .clone()
            .ok_or(BitBakeServerControllerError::MissingObservation)?;
        self.transition(BitBakeServerLifecycle::Stopping, None)?;
        let result = tokio::time::timeout(
            self.operation_timeout,
            self.adapter.stop(&self.context, &observation),
        )
        .await;
        match result {
            Err(_) => self.fail_timeout(BitBakeServerOperation::Stop),
            Ok(Err(error)) => self.fail_adapter(BitBakeServerOperation::Stop, error),
            Ok(Ok(())) => {
                self.state.observation = None;
                self.transition(BitBakeServerLifecycle::Unavailable, None)
            }
        }
    }

    pub async fn restart(&mut self) -> Result<(), BitBakeServerControllerError> {
        self.require_lifecycle(
            BitBakeServerOperation::Restart,
            &[
                BitBakeServerLifecycle::Available,
                BitBakeServerLifecycle::Connected,
            ],
        )?;
        let reconnect = self.session.is_some();
        self.transition(BitBakeServerLifecycle::Restarting, None)?;
        if let Some(session) = self.session.take() {
            self.timed_disconnect(BitBakeServerOperation::Restart, &session)
                .await?;
            self.state.connection_identity = None;
        }
        let observation = self
            .state
            .observation
            .clone()
            .ok_or(BitBakeServerControllerError::MissingObservation)?;
        self.timed_stop(BitBakeServerOperation::Restart, &observation)
            .await?;
        let observation = self.timed_start(BitBakeServerOperation::Restart).await?;
        self.state.observation = Some(observation.clone());
        if reconnect {
            let session = self
                .timed_connect(BitBakeServerOperation::Restart, &observation)
                .await?;
            self.state.connection_identity = Some(session.connection_identity.clone());
            self.session = Some(session);
            self.transition(BitBakeServerLifecycle::Connected, None)
        } else {
            self.transition(BitBakeServerLifecycle::Available, None)
        }
    }

    pub async fn reconnect(&mut self) -> Result<(), BitBakeServerControllerError> {
        self.require_lifecycle(
            BitBakeServerOperation::Reconnect,
            &[
                BitBakeServerLifecycle::Available,
                BitBakeServerLifecycle::Connected,
                BitBakeServerLifecycle::Failed,
            ],
        )?;
        let observation = self
            .state
            .observation
            .clone()
            .ok_or(BitBakeServerControllerError::MissingObservation)?;
        self.transition(BitBakeServerLifecycle::Recovering, None)?;
        if let Some(session) = self.session.take() {
            self.timed_disconnect(BitBakeServerOperation::Reconnect, &session)
                .await?;
            self.state.connection_identity = None;
        }
        let session = self
            .timed_connect(BitBakeServerOperation::Reconnect, &observation)
            .await?;
        self.state.connection_identity = Some(session.connection_identity.clone());
        self.session = Some(session);
        self.transition(BitBakeServerLifecycle::Connected, None)
    }

    async fn timed_start(
        &mut self,
        operation: BitBakeServerOperation,
    ) -> Result<BitBakeServerObservation, BitBakeServerControllerError> {
        match tokio::time::timeout(self.operation_timeout, self.adapter.start(&self.context)).await
        {
            Err(_) => self.fail_timeout(operation),
            Ok(Err(error)) => self.fail_adapter(operation, error),
            Ok(Ok(mut observation)) => {
                observation.capabilities.sort();
                observation.capabilities.dedup();
                observation.validate()?;
                Ok(observation)
            }
        }
    }

    async fn timed_connect(
        &mut self,
        operation: BitBakeServerOperation,
        observation: &BitBakeServerObservation,
    ) -> Result<BitBakeServerSession, BitBakeServerControllerError> {
        match tokio::time::timeout(
            self.operation_timeout,
            self.adapter.connect(&self.context, observation),
        )
        .await
        {
            Err(_) => self.fail_timeout(operation),
            Ok(Err(error)) => self.fail_adapter(operation, error),
            Ok(Ok(session)) => {
                session.validate(observation)?;
                Ok(session)
            }
        }
    }

    async fn timed_disconnect(
        &mut self,
        operation: BitBakeServerOperation,
        session: &BitBakeServerSession,
    ) -> Result<(), BitBakeServerControllerError> {
        match tokio::time::timeout(self.operation_timeout, self.adapter.disconnect(session)).await {
            Err(_) => self.fail_timeout(operation),
            Ok(Err(error)) => self.fail_adapter(operation, error),
            Ok(Ok(())) => Ok(()),
        }
    }

    async fn timed_stop(
        &mut self,
        operation: BitBakeServerOperation,
        observation: &BitBakeServerObservation,
    ) -> Result<(), BitBakeServerControllerError> {
        match tokio::time::timeout(
            self.operation_timeout,
            self.adapter.stop(&self.context, observation),
        )
        .await
        {
            Err(_) => self.fail_timeout(operation),
            Ok(Err(error)) => self.fail_adapter(operation, error),
            Ok(Ok(())) => Ok(()),
        }
    }

    fn require_not_busy(
        &self,
        operation: BitBakeServerOperation,
    ) -> Result<(), BitBakeServerControllerError> {
        if matches!(
            self.state.lifecycle,
            BitBakeServerLifecycle::Detecting
                | BitBakeServerLifecycle::Starting
                | BitBakeServerLifecycle::Connecting
                | BitBakeServerLifecycle::Disconnecting
                | BitBakeServerLifecycle::Stopping
                | BitBakeServerLifecycle::Restarting
                | BitBakeServerLifecycle::Recovering
        ) {
            return Err(BitBakeServerControllerError::InvalidTransition {
                operation,
                lifecycle: self.state.lifecycle,
            });
        }
        if self.state.lifecycle == BitBakeServerLifecycle::Connected {
            return Err(BitBakeServerControllerError::InvalidTransition {
                operation,
                lifecycle: self.state.lifecycle,
            });
        }
        Ok(())
    }

    fn require_lifecycle(
        &self,
        operation: BitBakeServerOperation,
        allowed: &[BitBakeServerLifecycle],
    ) -> Result<(), BitBakeServerControllerError> {
        if !allowed.contains(&self.state.lifecycle) {
            return Err(BitBakeServerControllerError::InvalidTransition {
                operation,
                lifecycle: self.state.lifecycle,
            });
        }
        Ok(())
    }

    fn transition(
        &mut self,
        lifecycle: BitBakeServerLifecycle,
        diagnostic: Option<String>,
    ) -> Result<(), BitBakeServerControllerError> {
        self.state.generation = self
            .state
            .generation
            .checked_add(1)
            .ok_or(BitBakeServerControllerError::GenerationExhausted)?;
        self.state.lifecycle = lifecycle;
        self.state.diagnostic = diagnostic;
        Ok(())
    }

    fn fail_timeout<T>(
        &mut self,
        operation: BitBakeServerOperation,
    ) -> Result<T, BitBakeServerControllerError> {
        self.transition(
            BitBakeServerLifecycle::Failed,
            Some(format!("{operation:?} timed out")),
        )?;
        Err(BitBakeServerControllerError::Timeout(operation))
    }

    fn fail_adapter<T>(
        &mut self,
        operation: BitBakeServerOperation,
        error: BitBakeServerAdapterError,
    ) -> Result<T, BitBakeServerControllerError> {
        self.transition(BitBakeServerLifecycle::Failed, Some(error.message.clone()))?;
        Err(BitBakeServerControllerError::Adapter {
            operation,
            message: error.message,
        })
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BitBakeServerControllerError {
    #[error("invalid BitBake server context: {0}")]
    InvalidContext(String),
    #[error("BitBake server timeout must be greater than zero")]
    InvalidTimeout,
    #[error("invalid BitBake server adapter data: {0}")]
    InvalidAdapterData(String),
    #[error("cannot {operation:?} while controller is {lifecycle:?}")]
    InvalidTransition {
        operation: BitBakeServerOperation,
        lifecycle: BitBakeServerLifecycle,
    },
    #[error("BitBake server observation is unavailable")]
    MissingObservation,
    #[error("BitBake server session is unavailable")]
    MissingSession,
    #[error("BitBake server {0:?} timed out")]
    Timeout(BitBakeServerOperation),
    #[error("BitBake server {operation:?} failed: {message}")]
    Adapter {
        operation: BitBakeServerOperation,
        message: String,
    },
    #[error("BitBake server controller generation is exhausted")]
    GenerationExhausted,
}

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
