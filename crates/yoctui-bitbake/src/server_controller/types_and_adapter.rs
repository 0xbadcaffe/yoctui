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
