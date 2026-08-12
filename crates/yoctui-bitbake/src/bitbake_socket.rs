use crate::{
    BackendError, BitBakeBackend, BitBakeServerAdapter, BitBakeServerAdapterError,
    BitBakeServerCapability, BitBakeServerContext, BitBakeServerEndpoint, BitBakeServerObservation,
    BitBakeServerSession, BridgeBackend,
};
use async_trait::async_trait;
use std::{
    collections::BTreeMap,
    ffi::OsString,
    os::unix::fs::{FileTypeExt, MetadataExt},
    path::{Path, PathBuf},
    time::Duration,
};

const SOCKET_REMOVAL_TIMEOUT: Duration = Duration::from_secs(5);

/// Adapter for BitBake's supported Tinfoil/process-server connection. Rust
/// retains typed ownership while the bridge imports the workspace's own
/// BitBake Python modules, including its SCM_RIGHTS/pickle implementation.
pub struct BitBakeSocketAdapter {
    python: OsString,
    bridge_script: PathBuf,
    environment: BTreeMap<String, String>,
    pending: Option<BridgeBackend>,
    connected: Option<BridgeBackend>,
    next_connection: u64,
}

impl BitBakeSocketAdapter {
    pub fn new(
        python: impl Into<OsString>,
        bridge_script: PathBuf,
        environment: BTreeMap<String, String>,
    ) -> Result<Self, BitBakeServerAdapterError> {
        if !bridge_script.is_absolute() {
            return Err(BitBakeServerAdapterError::new(
                "BitBake bridge script must be an absolute path",
            ));
        }
        Ok(Self {
            python: python.into(),
            bridge_script,
            environment,
            pending: None,
            connected: None,
            next_connection: 1,
        })
    }

    pub fn has_connected_transport(&self) -> bool {
        self.connected.is_some()
    }

    fn socket_path(context: &BitBakeServerContext) -> PathBuf {
        context.build_dir.join("bitbake.sock")
    }

    fn has_remote_server(&self) -> bool {
        self.environment
            .get("BBSERVER")
            .is_some_and(|value| !value.trim().is_empty())
    }

    async fn validate_existing_socket(path: &Path) -> Result<bool, BitBakeServerAdapterError> {
        match tokio::fs::symlink_metadata(path).await {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(BitBakeServerAdapterError::new(error.to_string())),
            Ok(metadata) => {
                if !metadata.file_type().is_socket() || metadata.file_type().is_symlink() {
                    return Err(BitBakeServerAdapterError::new(format!(
                        "{} is not a non-symlink Unix socket",
                        path.display()
                    )));
                }
                if metadata.uid() != effective_uid() {
                    return Err(BitBakeServerAdapterError::new(format!(
                        "{} is owned by another UID",
                        path.display()
                    )));
                }
                Ok(true)
            }
        }
    }

    async fn spawn_bridge(
        &self,
        context: &BitBakeServerContext,
    ) -> Result<BridgeBackend, BitBakeServerAdapterError> {
        let python = self.python.to_string_lossy();
        BridgeBackend::spawn_with_environment(
            &python,
            self.bridge_script.clone(),
            context.build_dir.clone(),
            self.environment.clone(),
        )
        .await
        .map_err(adapter_backend_error)
    }

    async fn connect_and_observe(
        &self,
        context: &BitBakeServerContext,
    ) -> Result<(BridgeBackend, BitBakeServerObservation), BitBakeServerAdapterError> {
        let mut backend = self.spawn_bridge(context).await?;
        let workspace = backend
            .inspect_workspace()
            .await
            .map_err(adapter_backend_error)?;
        if workspace.build_dir.as_deref() != Some(context.build_dir.as_path()) {
            return Err(BitBakeServerAdapterError::new(format!(
                "BitBake reported build directory {:?}, expected {}",
                workspace.build_dir,
                context.build_dir.display()
            )));
        }
        let observation = self.observation(context, workspace.bitbake_version).await?;
        Ok((backend, observation))
    }

    async fn observation(
        &self,
        context: &BitBakeServerContext,
        version: Option<String>,
    ) -> Result<BitBakeServerObservation, BitBakeServerAdapterError> {
        let socket = Self::socket_path(context);
        let (endpoint, server_identity) = match tokio::fs::symlink_metadata(&socket).await {
            Ok(metadata) => {
                if !metadata.file_type().is_socket() || metadata.file_type().is_symlink() {
                    return Err(BitBakeServerAdapterError::new(format!(
                        "{} is not a non-symlink Unix socket",
                        socket.display()
                    )));
                }
                if metadata.uid() != effective_uid() {
                    return Err(BitBakeServerAdapterError::new(format!(
                        "{} is owned by another UID",
                        socket.display()
                    )));
                }
                (
                    BitBakeServerEndpoint::UnixSocket(socket),
                    format!("unix:{}:{}", metadata.dev(), metadata.ino()),
                )
            }
            Err(error)
                if error.kind() == std::io::ErrorKind::NotFound && self.has_remote_server() =>
            {
                let remote = self.environment["BBSERVER"].trim();
                if remote.len() > 512 || remote.chars().any(char::is_control) {
                    return Err(BitBakeServerAdapterError::new(
                        "BBSERVER identity is empty, oversized, or contains control characters",
                    ));
                }
                (
                    BitBakeServerEndpoint::Managed("BBSERVER".into()),
                    format!("remote:{remote}"),
                )
            }
            Err(error) => return Err(BitBakeServerAdapterError::new(error.to_string())),
        };
        Ok(BitBakeServerObservation {
            endpoint,
            server_identity,
            version,
            capabilities: vec![
                BitBakeServerCapability::CommandChannel,
                BitBakeServerCapability::EventStream,
                BitBakeServerCapability::Metadata,
                BitBakeServerCapability::BuildControl,
                BitBakeServerCapability::Cancellation,
                BitBakeServerCapability::ServerStop,
                BitBakeServerCapability::ServerRestart,
            ],
        })
    }

    async fn terminate_backend(
        backend: &mut BridgeBackend,
    ) -> Result<(), BitBakeServerAdapterError> {
        backend
            .terminate_server()
            .await
            .map_err(adapter_backend_error)
    }

    async fn wait_for_socket_removal(path: &Path) -> Result<(), BitBakeServerAdapterError> {
        let deadline = tokio::time::Instant::now() + SOCKET_REMOVAL_TIMEOUT;
        loop {
            match tokio::fs::symlink_metadata(path).await {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
                Err(error) => return Err(BitBakeServerAdapterError::new(error.to_string())),
                Ok(_) if tokio::time::Instant::now() >= deadline => {
                    return Err(BitBakeServerAdapterError::new(format!(
                        "BitBake server socket {} remained after termination",
                        path.display()
                    )));
                }
                Ok(_) => tokio::time::sleep(Duration::from_millis(25)).await,
            }
        }
    }
}

#[async_trait]
impl BitBakeServerAdapter for BitBakeSocketAdapter {
    async fn detect(
        &mut self,
        context: &BitBakeServerContext,
    ) -> Result<Option<BitBakeServerObservation>, BitBakeServerAdapterError> {
        if self.pending.is_some() || self.connected.is_some() {
            return Err(BitBakeServerAdapterError::new(
                "a BitBake socket transport is already owned",
            ));
        }
        let socket_exists = Self::validate_existing_socket(&Self::socket_path(context)).await?;
        if !socket_exists && !self.has_remote_server() {
            return Ok(None);
        }
        let (backend, observation) = self.connect_and_observe(context).await?;
        self.pending = Some(backend);
        Ok(Some(observation))
    }

    async fn start(
        &mut self,
        context: &BitBakeServerContext,
    ) -> Result<BitBakeServerObservation, BitBakeServerAdapterError> {
        if self.pending.is_some() || self.connected.is_some() {
            return Err(BitBakeServerAdapterError::new(
                "a BitBake socket transport is already owned",
            ));
        }
        let (backend, observation) = self.connect_and_observe(context).await?;
        self.pending = Some(backend);
        Ok(observation)
    }

    async fn connect(
        &mut self,
        context: &BitBakeServerContext,
        observation: &BitBakeServerObservation,
    ) -> Result<BitBakeServerSession, BitBakeServerAdapterError> {
        if self.connected.is_some() {
            return Err(BitBakeServerAdapterError::new(
                "BitBake socket transport is already connected",
            ));
        }
        let backend = if let Some(backend) = self.pending.take() {
            backend
        } else {
            let (backend, actual) = self.connect_and_observe(context).await?;
            if actual.server_identity != observation.server_identity {
                return Err(BitBakeServerAdapterError::new(
                    "BitBake server identity changed before connection",
                ));
            }
            backend
        };
        let connection_identity = format!("tinfoil-{}", self.next_connection);
        self.next_connection = self.next_connection.saturating_add(1);
        self.connected = Some(backend);
        Ok(BitBakeServerSession {
            server_identity: observation.server_identity.clone(),
            connection_identity,
        })
    }

    async fn disconnect(
        &mut self,
        _session: &BitBakeServerSession,
    ) -> Result<(), BitBakeServerAdapterError> {
        let mut backend = self.connected.take().ok_or_else(|| {
            BitBakeServerAdapterError::new("BitBake socket transport is not connected")
        })?;
        backend.shutdown().await.map_err(adapter_backend_error)
    }

    async fn stop(
        &mut self,
        context: &BitBakeServerContext,
        _observation: &BitBakeServerObservation,
    ) -> Result<(), BitBakeServerAdapterError> {
        if self.connected.is_some() {
            return Err(BitBakeServerAdapterError::new(
                "disconnect the BitBake socket transport before stopping its server",
            ));
        }
        let socket = Self::socket_path(context);
        let mut backend = if let Some(backend) = self.pending.take() {
            backend
        } else {
            match tokio::fs::symlink_metadata(&socket).await {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
                Err(error) => return Err(BitBakeServerAdapterError::new(error.to_string())),
                Ok(_) => self.spawn_bridge(context).await?,
            }
        };
        Self::terminate_backend(&mut backend).await?;
        if matches!(_observation.endpoint, BitBakeServerEndpoint::UnixSocket(_)) {
            Self::wait_for_socket_removal(&socket).await?;
        }
        Ok(())
    }
}

fn adapter_backend_error(error: BackendError) -> BitBakeServerAdapterError {
    BitBakeServerAdapterError::new(error.to_string())
}

fn effective_uid() -> u32 {
    // SAFETY: geteuid has no preconditions and does not modify memory.
    unsafe { libc::geteuid() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BitBakeServerController, BitBakeServerLifecycle};
    use std::{fs, os::unix::fs::PermissionsExt};

    fn fixture() -> (PathBuf, PathBuf, BitBakeServerContext) {
        let nonce = std::time::SystemTime::UNIX_EPOCH
            .elapsed()
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "yoctui-bitbake-socket-{}-{nonce}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("build/conf")).unwrap();
        fs::create_dir_all(root.join("source")).unwrap();
        let script = root.join("fake-bridge.py");
        fs::write(
            &script,
            r#"#!/usr/bin/env python3
import json, os, socket, sys
path = os.path.join(os.getcwd(), "bitbake.sock")
try: os.unlink(path)
except FileNotFoundError: pass
server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
server.bind(path)
sequence = 0
for raw in sys.stdin:
    request = json.loads(raw)
    kind = request["message"]["type"]
    sequence += 1
    if kind == "hello": message = {"type":"hello_ack","bitbake_version":"2.8.1"}
    elif kind == "inspect_workspace": message = {"type":"workspace","data":{"build_dir":os.getcwd(),"source_dir":os.path.dirname(os.getcwd()),"variables":{},"bitbake_version":"2.8.1","layers":[],"recipes":[]}}
    elif kind == "shutdown": message = {"type":"bridge_shutdown"}
    elif kind == "terminate_server": message = {"type":"server_terminated"}
    else: message = {"type":"command_failed","code":"unsupported","message":kind}
    print(json.dumps({"protocol_version":1,"sequence":sequence,"correlation_id":request.get("correlation_id"),"message":message}), flush=True)
    if kind in ("shutdown", "terminate_server"):
        if kind == "terminate_server":
            server.close()
            try: os.unlink(path)
            except FileNotFoundError: pass
        break
"#,
        )
        .unwrap();
        let mut permissions = fs::metadata(&script).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&script, permissions).unwrap();
        let context = BitBakeServerContext {
            source_dir: root.join("source"),
            build_dir: root.join("build"),
            init_script: root.join("source/oe-init-build-env"),
        };
        (root, script, context)
    }

    #[tokio::test]
    async fn bitbake_socket_starts_connects_correlates_and_stops_supported_server() {
        let (root, script, context) = fixture();
        let adapter = BitBakeSocketAdapter::new("python3", script, BTreeMap::new()).unwrap();
        let mut controller =
            BitBakeServerController::new(adapter, context.clone(), Duration::from_secs(2)).unwrap();
        assert_eq!(
            controller.detect().await.unwrap(),
            crate::BitBakeDetection::Unavailable
        );
        controller.start().await.unwrap();
        let observation = controller.state().observation.as_ref().unwrap();
        assert_eq!(observation.version.as_deref(), Some("2.8.1"));
        assert!(
            observation
                .capabilities
                .contains(&BitBakeServerCapability::EventStream)
        );
        controller.connect().await.unwrap();
        assert_eq!(
            controller.state().lifecycle,
            BitBakeServerLifecycle::Connected
        );
        assert!(
            controller
                .state()
                .connection_identity
                .as_deref()
                .unwrap()
                .starts_with("tinfoil-")
        );
        controller.stop().await.unwrap();
        assert_eq!(
            controller.state().lifecycle,
            BitBakeServerLifecycle::Unavailable
        );
        assert!(!context.build_dir.join("bitbake.sock").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn bitbake_socket_rejects_non_socket_and_reports_server_loss() {
        let (root, script, context) = fixture();
        fs::write(context.build_dir.join("bitbake.sock"), "not a socket").unwrap();
        let adapter =
            BitBakeSocketAdapter::new("python3", script.clone(), BTreeMap::new()).unwrap();
        let mut controller =
            BitBakeServerController::new(adapter, context.clone(), Duration::from_secs(2)).unwrap();
        assert!(controller.detect().await.is_err());
        assert_eq!(controller.state().lifecycle, BitBakeServerLifecycle::Failed);

        fs::remove_file(context.build_dir.join("bitbake.sock")).unwrap();
        let body = fs::read_to_string(&script)
            .unwrap()
            .replace("request.get(\"correlation_id\")", "\"unknown-correlation\"");
        fs::write(&script, body).unwrap();
        let adapter = BitBakeSocketAdapter::new("python3", script, BTreeMap::new()).unwrap();
        let mut controller =
            BitBakeServerController::new(adapter, context, Duration::from_secs(2)).unwrap();
        assert!(controller.start().await.is_err());
        assert_eq!(controller.state().lifecycle, BitBakeServerLifecycle::Failed);
        assert!(
            controller
                .state()
                .diagnostic
                .as_deref()
                .is_some_and(|message| message.contains("unknown correlation"))
        );
        fs::remove_dir_all(root).unwrap();
    }
}
