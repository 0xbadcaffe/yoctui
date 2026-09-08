//! Bounded, connection-owned read-only image metadata. Never broadcast sources.
use anyhow::{Context, Result, ensure};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::sync::{Semaphore, oneshot};
use yoctui_bitbake::BitBakeBackend;
use yoctui_model::DaemonCompatibilitySnapshot;
use yoctui_protocol::{
    daemon::{DaemonInstanceId, RequestId},
    rootfs::{RootfsSourcesData, RootfsSourcesRequestData},
};

pub const QUERY_TIMEOUT: Duration = Duration::from_secs(120);

pub fn query_for_app(
    app: &yoctui_model::App,
    request: &yoctui_model::RootfsCompositionRequest,
) -> Result<RootfsSourcesRequestData> {
    ensure!(
        app.daemon.status == yoctui_model::ClientReplicaStatus::Current,
        "rootfs daemon authority is not current"
    );
    let instance = app
        .daemon
        .instance_id
        .context("rootfs daemon authority is unavailable")?;
    let compatibility = app
        .workspace_compatibility
        .authority()
        .context("rootfs compatibility authority is unavailable")?;
    let query = RootfsSourcesRequestData {
        request: yoctui_protocol::rootfs::RootfsCompositionRequestData {
            generation: request.generation,
            image: yoctui_protocol::rootfs::RootfsImageIdentityData {
                machine: request.image.machine.clone(),
                image: request.image.image.clone(),
                path: request.image.path.to_string_lossy().into_owned(),
            },
        },
        daemon_instance_id: DaemonInstanceId(instance.0),
        compatibility_generation: compatibility.snapshot.generation,
    };
    query.validate()?;
    Ok(query)
}

/// Runs only in a blocking worker, never on the terminal input loop.
pub fn request_sources(
    query: &RootfsSourcesRequestData,
    build: &Path,
    cancellation: &yoctui_bitbake::RootfsCompositionCancellation,
) -> Result<RootfsSourcesData> {
    request_sources_at(
        query,
        build,
        cancellation,
        &yoctui_protocol::daemon_ipc::runtime_paths()?,
    )
}

fn request_sources_at(
    query: &RootfsSourcesRequestData,
    build: &Path,
    cancellation: &yoctui_bitbake::RootfsCompositionCancellation,
    paths: &yoctui_protocol::daemon_ipc::RuntimePaths,
) -> Result<RootfsSourcesData> {
    use std::time::Instant;
    use yoctui_protocol::{daemon::*, daemon_ipc::DaemonConnection};
    let mut connection = DaemonConnection::connect(paths, Duration::from_secs(1))?;
    connection.set_timeout(Some(Duration::from_secs(10)))?;
    static NEXT_CLIENT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let counter = NEXT_CLIENT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut client_id = [0; 16];
    client_id[..4].copy_from_slice(b"root");
    client_id[4..8].copy_from_slice(&std::process::id().to_le_bytes());
    client_id[8..].copy_from_slice(&counter.to_le_bytes());
    connection.send(&ClientMessage::Hello(ClientHello {
        minimum_version: ProtocolVersion::CURRENT,
        maximum_version: ProtocolVersion::CURRENT,
        client_id: ClientId(client_id),
        client_name: "yoctui-rootfs-query".into(),
        capabilities: vec![Capability::StateSnapshots, Capability::RootfsSources],
    }))?;
    let ServerMessage::Hello(hello) = connection.receive()? else {
        anyhow::bail!("unexpected rootfs daemon handshake");
    };
    ensure!(
        hello.daemon_instance_id == query.daemon_instance_id,
        "rootfs daemon instance changed"
    );
    ensure!(
        hello.selected_version == ProtocolVersion::CURRENT,
        "rootfs daemon protocol version changed"
    );
    ensure!(
        hello.capabilities.contains(&Capability::RootfsSources),
        "daemon does not support rootfs source queries; upgrade the daemon"
    );
    let deadline = Instant::now() + QUERY_TIMEOUT + Duration::from_secs(45);
    let mut remaining_messages = 8192;
    for attempt in 1..=3 {
        connection.send(&ClientMessage::Attach {
            workspace: None,
            resume: None,
            subscription: Subscription {
                state: false,
                jobs: false,
                logs: false,
                pty_sessions: vec![],
            },
        })?;
        let snapshot = loop {
            match receive_query(
                &mut connection,
                deadline,
                cancellation,
                &mut remaining_messages,
            )? {
                ServerMessage::Attached { snapshot, .. } => break snapshot,
                ServerMessage::Event(_)
                | ServerMessage::Snapshot(_)
                | ServerMessage::ResyncRequired { .. } => {}
                _ => anyhow::bail!("unexpected rootfs attach reply"),
            }
        };
        ensure!(
            snapshot.daemon_instance_id == query.daemon_instance_id,
            "rootfs daemon instance changed"
        );
        let compatibility = snapshot
            .compatibility
            .as_ref()
            .context("rootfs daemon compatibility is unavailable")?;
        ensure!(
            compatibility.generation == query.compatibility_generation,
            "rootfs daemon compatibility changed"
        );
        ensure!(
            matches!(&compatibility.environment.build_directory, CompatibilityDetected::Detected { value, .. } if Path::new(value) == build),
            "rootfs daemon build directory changed"
        );
        let request_id = RequestId(attempt);
        connection.send(&ClientMessage::Command(CommandRequest {
            request_id,
            expected_generation: Some(snapshot.generation),
            command: DaemonCommand::InspectRootfsSources {
                query: query.clone(),
            },
        }))?;
        let outcome = loop {
            match receive_query(
                &mut connection,
                deadline,
                cancellation,
                &mut remaining_messages,
            )? {
                ServerMessage::CommandResult(result) if result.request_id == request_id => {
                    break result.outcome;
                }
                ServerMessage::Event(_)
                | ServerMessage::Snapshot(_)
                | ServerMessage::ResyncRequired { .. } => {}
                _ => anyhow::bail!("unexpected rootfs metadata reply"),
            }
        };
        match outcome {
            CommandOutcome::RootfsSources { sources } => {
                sources.validate()?;
                ensure!(
                    sources.query == *query,
                    "rootfs source reply belongs to another image/request"
                );
                let _ = connection.send(&ClientMessage::Detach);
                return Ok(*sources);
            }
            CommandOutcome::Rejected {
                code: ProtocolErrorCode::StaleGeneration,
                ..
            } if attempt < 3 => {}
            CommandOutcome::Rejected { message, .. } => anyhow::bail!("{message}"),
            _ => anyhow::bail!("unexpected rootfs metadata command outcome"),
        }
    }
    anyhow::bail!("rootfs source authority kept changing; refresh and retry")
}

fn receive_query(
    connection: &mut yoctui_protocol::daemon_ipc::DaemonConnection,
    deadline: std::time::Instant,
    cancellation: &yoctui_bitbake::RootfsCompositionCancellation,
    remaining_messages: &mut usize,
) -> Result<yoctui_protocol::daemon::ServerMessage> {
    use yoctui_protocol::daemon::{ClientMessage, ServerMessage};
    loop {
        ensure!(
            !cancellation.is_cancelled(),
            "rootfs source query cancelled"
        );
        ensure!(
            std::time::Instant::now() < deadline,
            "rootfs source query timed out"
        );
        if !connection.is_readable()? {
            std::thread::sleep(Duration::from_millis(10));
            continue;
        }
        ensure!(
            *remaining_messages > 0,
            "rootfs query exceeded the interleaved-message bound"
        );
        *remaining_messages -= 1;
        match connection.receive()? {
            ServerMessage::Ping { nonce, .. } => connection.send(&ClientMessage::Pong { nonce })?,
            ServerMessage::Error(error) => {
                anyhow::bail!("rootfs protocol error: {}", error.message)
            }
            message => return Ok(message),
        }
    }
}

pub fn validate_authority(
    query: &RootfsSourcesRequestData,
    instance: DaemonInstanceId,
    compatibility: &DaemonCompatibilitySnapshot,
) -> Result<PathBuf> {
    query.validate()?;
    ensure!(
        query.daemon_instance_id == instance,
        "rootfs query belongs to another daemon instance"
    );
    ensure!(
        query.compatibility_generation == compatibility.snapshot.generation,
        "rootfs query compatibility generation is stale"
    );
    let environment = &compatibility.snapshot.environment;
    let build = environment
        .build_directory
        .value()
        .context("rootfs query has no build authority")?;
    ensure!(
        environment.machine.value() == Some(&query.request.image.machine),
        "rootfs query machine does not match daemon authority"
    );
    let artifact = Path::new(&query.request.image.path);
    let metadata =
        std::fs::symlink_metadata(artifact).context("selected rootfs artifact is unavailable")?;
    ensure!(
        metadata.is_file() && !metadata.file_type().is_symlink(),
        "selected rootfs artifact must be a regular file"
    );
    let canonical_build = build.canonicalize()?;
    ensure!(
        artifact.canonicalize()?.starts_with(&canonical_build),
        "selected rootfs artifact escapes daemon build directory"
    );
    Ok(build.clone())
}

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

async fn acquire(
    query: RootfsSourcesRequestData,
    build: PathBuf,
    compatibility: DaemonCompatibilitySnapshot,
    environment: BTreeMap<String, String>,
    mut cancelled: oneshot::Receiver<()>,
    query_timeout: Duration,
) -> Result<RootfsSourcesData> {
    if compatibility
        .implementations
        .get(&yoctui_model::CapabilityId::BitBakeGetVar)
        .is_some_and(|selected| {
            matches!(
                selected.id.as_str(),
                yoctui_bitbake::BITBAKE_GETVAR_UTILITY_IMPLEMENTATION
                    | yoctui_bitbake::BITBAKE_GETVAR_ENVIRONMENT_IMPLEMENTATION
            )
        })
    {
        let planner = yoctui_bitbake::BitBakeCommandPlanner::new(
            &compatibility,
            query.compatibility_generation,
            &build,
        )?;
        let deadline = tokio::time::Instant::now() + query_timeout;
        let mut values = Vec::with_capacity(3);
        for name in ["IMAGE_MANIFEST", "PKGDATA_DIR", "IMAGE_ROOTFS"] {
            let command = planner.get_variable(name, Some(&query.request.image.image))?;
            let utility =
                command.implementation == yoctui_bitbake::BITBAKE_GETVAR_UTILITY_IMPLEMENTATION;
            let output =
                run_source_command(command, &build, &environment, &mut cancelled, deadline).await?;
            let value = if utility {
                Some(output.trim())
            } else {
                let prefix = format!("{name}=");
                output
                    .lines()
                    .rev()
                    .find_map(|line| line.strip_prefix(&prefix))
                    .map(|value| {
                        value
                            .strip_prefix('"')
                            .and_then(|v| v.strip_suffix('"'))
                            .unwrap_or(value)
                    })
            };
            values.push(value.filter(|v| !v.is_empty()).map(str::to_owned));
        }
        let sources = RootfsSourcesData {
            query,
            image_manifest: values[0].take(),
            pkgdata_dir: values[1].take(),
            image_rootfs: values[2].take(),
        };
        sources.validate()?;
        return Ok(sources);
    }
    let python = environment
        .get("PYTHON")
        .map(String::as_str)
        .unwrap_or("python3");
    let startup = crate::spawn_configured_bridge_with_compatibility(
        python,
        build,
        Some(environment.clone()),
        compatibility,
    );
    let mut backend = tokio::select! {
        biased;
        _ = &mut cancelled => anyhow::bail!("rootfs metadata query cancelled"),
        result = tokio::time::timeout(Duration::from_secs(30), startup) => result.context("rootfs bridge handshake timed out")??,
    };
    let result = tokio::select! {
        biased;
        _ = &mut cancelled => Err(anyhow::anyhow!("rootfs metadata query cancelled")),
        result = tokio::time::timeout(query_timeout, async {
            let recipe = Some(query.request.image.image.clone());
            let image_manifest = backend.get_variable("IMAGE_MANIFEST".into(), recipe.clone()).await?.value;
            let pkgdata_dir = backend.get_variable("PKGDATA_DIR".into(), recipe.clone()).await?.value;
            let image_rootfs = backend.get_variable("IMAGE_ROOTFS".into(), recipe).await?.value;
            let sources = RootfsSourcesData { query, image_manifest, pkgdata_dir, image_rootfs };
            sources.validate()?;
            Ok::<_, anyhow::Error>(sources)
        }) => result.context("rootfs metadata query timed out").and_then(|result| result),
    };
    if result.is_err()
        || !matches!(
            tokio::time::timeout(Duration::from_secs(5), backend.shutdown()).await,
            Ok(Ok(()))
        )
    {
        backend.interrupt_metadata().await;
    }
    result
}

async fn run_source_command(
    command: yoctui_bitbake::AuthorizedBitBakeCommand,
    build: &Path,
    environment: &BTreeMap<String, String>,
    cancelled: &mut oneshot::Receiver<()>,
    deadline: tokio::time::Instant,
) -> Result<String> {
    use std::process::Stdio;
    use tokio::io::AsyncReadExt;
    async fn bounded(mut reader: impl tokio::io::AsyncRead + Unpin) -> Result<Vec<u8>> {
        const LIMIT: usize = 16 * 1024 * 1024;
        let mut bytes = Vec::new();
        let mut buffer = [0; 8192];
        loop {
            let count = reader.read(&mut buffer).await?;
            if count == 0 {
                return Ok(bytes);
            }
            ensure!(
                bytes.len() + count <= LIMIT,
                "rootfs metadata query exceeded the 16 MiB output bound"
            );
            bytes.extend_from_slice(&buffer[..count]);
        }
    }
    ensure!(
        tokio::time::Instant::now() < deadline,
        "rootfs metadata query timed out"
    );
    ensure!(
        matches!(
            cancelled.try_recv(),
            Err(oneshot::error::TryRecvError::Empty)
        ),
        "rootfs metadata query cancelled"
    );
    let mut child = tokio::process::Command::new(command.executable)
        .args(command.arguments)
        .current_dir(build)
        .env_clear()
        .envs(environment)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .kill_on_drop(true)
        .spawn()
        .context("could not start authorized rootfs variable query")?;
    let group = child
        .id()
        .context("rootfs query child identity unavailable")? as i32;
    let stdout = child.stdout.take().context("rootfs stdout unavailable")?;
    let stderr = child.stderr.take().context("rootfs stderr unavailable")?;
    let result = tokio::select! {
        biased;
        _ = cancelled => Err(anyhow::anyhow!("rootfs metadata query cancelled")),
        _ = tokio::time::sleep_until(deadline) => Err(anyhow::anyhow!("rootfs metadata query timed out")),
        result = async { tokio::try_join!(bounded(stdout), bounded(stderr), async { Ok::<_, anyhow::Error>(child.wait().await?) }) } => result,
    };
    let (stdout, stderr, status) = match result {
        Ok(output) => output,
        Err(error) => {
            // This process group belongs exclusively to this read-only query.
            let _ = unsafe { libc::kill(-group, libc::SIGINT) };
            if tokio::time::timeout(Duration::from_secs(5), child.wait())
                .await
                .is_err()
            {
                let _ = unsafe { libc::kill(-group, libc::SIGKILL) };
                let _ = child.kill().await;
                let _ = child.wait().await;
            }
            return Err(error);
        }
    };
    ensure!(
        status.success(),
        "rootfs variable query exited with {status}: {}",
        String::from_utf8_lossy(&stderr[..stderr.len().min(4096)]).trim()
    );
    String::from_utf8(stdout).context("rootfs variable query returned non-UTF-8 output")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        os::unix::fs::PermissionsExt,
        time::{SystemTime, UNIX_EPOCH},
    };
    use yoctui_model::*;

    fn fixture() -> (
        PathBuf,
        RootfsSourcesRequestData,
        DaemonCompatibilitySnapshot,
        BTreeMap<String, String>,
    ) {
        let build = std::env::temp_dir().join(format!(
            "yoctui-rootfs-query-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&build).unwrap();
        let artifact = build.join("image.manifest");
        fs::write(&artifact, "busybox machine 1\n").unwrap();
        let fake = build.join("python-fixture");
        fs::write(&fake, r#"#!/usr/bin/python3
import json, os, sys, time
from pathlib import Path
sequence=0
for line in sys.stdin:
    envelope=json.loads(line); command=envelope['message']; kind=command['type']
    if kind=='hello':
        authority=command['compatibility']
        result={'type':'hello_ack','bitbake_version':'2.18.0','compatibility_generation':authority['generation'],'capabilities':[x['id'] for x in authority['capabilities']]}
    elif kind=='get_variable':
        assert command['recipe']=='image'
        Path('query.pid').write_text(str(os.getpid()))
        mode=os.environ.get('ROOTFS_TEST_MODE','ok')
        if mode=='hang': time.sleep(60)
        if mode=='error': result={'type':'command_failed','code':'fixture_error','message':'exact image query failed'}
        else:
            value=None if mode=='absent' else str(Path.cwd()/({'IMAGE_MANIFEST':'image.manifest','PKGDATA_DIR':'pkgdata','IMAGE_ROOTFS':'retained-rootfs'}[command['name']]))
            result={'type':'variable','name':command['name'],'recipe':command['recipe'],'value':value,'provenance':None}
    elif kind=='shutdown': result={'type':'bridge_shutdown'}
    else: raise RuntimeError(kind)
    sequence+=1
    print(json.dumps({'protocol_version':1,'sequence':sequence,'correlation_id':envelope['correlation_id'],'message':result}), flush=True)
    if kind=='shutdown': break
"#).unwrap();
        fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
        let mut compatibility = yoctui_bitbake::release_capability_fixtures()
            .into_iter()
            .find(|f| f.role == yoctui_bitbake::CompatibilityFixtureRole::CurrentStableCandidate)
            .unwrap()
            .command_authority(9);
        compatibility.snapshot.environment.build_directory =
            AuthoritativeValue::detected(build.clone(), IdentityAuthority::InitializedEnvironment);
        compatibility.snapshot.environment.machine =
            AuthoritativeValue::detected("machine".into(), IdentityAuthority::BitBakeDatastore);
        let record = compatibility
            .snapshot
            .capabilities
            .iter_mut()
            .find(|r| r.id == CapabilityId::BitBakeGetVar)
            .unwrap();
        record.state = CapabilityState::Available;
        compatibility.implementations.insert(
            CapabilityId::BitBakeGetVar,
            CapabilityImplementation {
                id: "tinfoil.getvar".into(),
                kind: CapabilityImplementationKind::BackendApi,
            },
        );
        let query = RootfsSourcesRequestData {
            request: yoctui_protocol::rootfs::RootfsCompositionRequestData {
                generation: 3,
                image: yoctui_protocol::rootfs::RootfsImageIdentityData {
                    machine: "machine".into(),
                    image: "image".into(),
                    path: artifact.display().to_string(),
                },
            },
            daemon_instance_id: DaemonInstanceId([4; 16]),
            compatibility_generation: 9,
        };
        (
            build,
            query,
            compatibility,
            BTreeMap::from([("PYTHON".into(), fake.display().to_string())]),
        )
    }

    #[test]
    fn rootfs_query_validates_full_instance_generation_machine_and_containment() {
        let (build, query, compatibility, _) = fixture();
        assert_eq!(
            validate_authority(&query, query.daemon_instance_id, &compatibility).unwrap(),
            build
        );
        let mut different = query.daemon_instance_id;
        different.0[15] ^= 1;
        assert!(validate_authority(&query, different, &compatibility).is_err());
        let mut bad = query.clone();
        bad.compatibility_generation += 1;
        assert!(validate_authority(&bad, query.daemon_instance_id, &compatibility).is_err());
        let mut bad = query.clone();
        bad.request.image.machine = "other".into();
        assert!(validate_authority(&bad, query.daemon_instance_id, &compatibility).is_err());
        let link = build.join("linked");
        std::os::unix::fs::symlink("image.manifest", &link).unwrap();
        let mut bad = query.clone();
        bad.request.image.path = link.display().to_string();
        assert!(validate_authority(&bad, query.daemon_instance_id, &compatibility).is_err());
        let mut bad = query.clone();
        bad.request.image.path = "/etc/passwd".into();
        assert!(validate_authority(&bad, query.daemon_instance_id, &compatibility).is_err());
        fs::remove_dir_all(build).unwrap();
    }

    #[test]
    fn rootfs_client_query_requires_current_full_identity_and_compatibility() {
        let (build, query, compatibility, _) = fixture();
        let request = RootfsCompositionRequest {
            generation: query.request.generation,
            image: ImageArtifactIdentity {
                machine: "machine".into(),
                image: "image".into(),
                path: query.request.image.path.clone().into(),
            },
        };
        let mut app = App::new(16, 4096);
        assert!(query_for_app(&app, &request).is_err());
        app.daemon.status = ClientReplicaStatus::Current;
        app.daemon.instance_id = Some(DaemonModelInstanceId(query.daemon_instance_id.0));
        assert!(query_for_app(&app, &request).is_err());
        app.workspace_compatibility.install(compatibility).unwrap();
        assert_eq!(query_for_app(&app, &request).unwrap(), query);
        app.daemon.status = ClientReplicaStatus::Stale;
        assert!(query_for_app(&app, &request).is_err());
        app.daemon.status = ClientReplicaStatus::Current;
        app.daemon.instance_id.as_mut().unwrap().0[15] ^= 1;
        assert_ne!(query_for_app(&app, &request).unwrap(), query);
        fs::remove_dir_all(build).unwrap();
    }

    #[tokio::test]
    async fn rootfs_query_honors_command_implementation_and_bounds_owned_process() {
        for implementation in ["bitbake_getvar.argv", "bitbake.environment_lookup"] {
            for mode in ["ok", "absent", "error", "hang", "cancel", "oversized"] {
                let (build, query, mut compatibility, mut environment) = fixture();
                let executable = build.join("getvar-fixture");
                fs::write(&executable, r#"#!/usr/bin/python3
import os, sys, time
from pathlib import Path
Path('command.pid').write_text(str(os.getpid()))
assert os.environ['ROOTFS_MARKER']=='daemon-environment'
mode=os.environ['ROOTFS_TEST_MODE']
assert sys.argv[1:]==['-e','image'] or (sys.argv[1:4]==['--value','--recipe','image'] and len(sys.argv)==5)
if mode in ('hang','cancel'): time.sleep(60)
if mode=='error': sys.exit('fixture command error')
if mode=='oversized':
    sys.stdout.write('x' * (17 * 1024 * 1024)); sys.stdout.flush(); time.sleep(60)
values={k:str(Path.cwd()/v) for k,v in [('IMAGE_MANIFEST','image.manifest'),('PKGDATA_DIR','pkgdata'),('IMAGE_ROOTFS','retained-rootfs')]}
if sys.argv[1]=='-e':
    for name,value in values.items(): print(name+'="'+('' if mode=='absent' else value)+'"')
else: print('' if mode=='absent' else values[sys.argv[-1]])
"#).unwrap();
                fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
                let selected = compatibility
                    .implementations
                    .get_mut(&CapabilityId::BitBakeGetVar)
                    .unwrap();
                selected.id = implementation.into();
                selected.kind = CapabilityImplementationKind::Command;
                if let AuthoritativeValue::Detected { value: tools, .. } =
                    &mut compatibility.snapshot.environment.available_tools
                {
                    for tool in tools {
                        if matches!(tool.id.as_str(), "bitbake-getvar" | "bitbake") {
                            tool.executable = executable.clone();
                        }
                    }
                } else {
                    panic!("fixture tools must be detected");
                }
                environment.insert("ROOTFS_MARKER".into(), "daemon-environment".into());
                environment.insert("ROOTFS_TEST_MODE".into(), mode.into());
                let (cancel, cancelled) = oneshot::channel();
                let cancel_build = build.clone();
                let cancellation = tokio::spawn(async move {
                    if mode == "cancel" {
                        for _ in 0..100 {
                            if cancel_build.join("command.pid").exists() {
                                break;
                            }
                            tokio::time::sleep(Duration::from_millis(10)).await;
                        }
                        let _ = cancel.send(());
                    } else {
                        tokio::time::sleep(Duration::from_secs(10)).await;
                        drop(cancel);
                    }
                });
                let result = acquire(
                    query.clone(),
                    build.clone(),
                    compatibility,
                    environment,
                    cancelled,
                    if mode == "hang" {
                        Duration::from_millis(300)
                    } else {
                        Duration::from_secs(5)
                    },
                )
                .await;
                cancellation.abort();
                if matches!(mode, "ok" | "absent") {
                    let sources = result.unwrap();
                    assert_eq!(sources.query, query);
                    assert_eq!(
                        sources.image_rootfs,
                        (mode == "ok").then(|| build.join("retained-rootfs").display().to_string())
                    );
                } else {
                    let error = format!("{:#}", result.unwrap_err());
                    assert!(
                        error.contains(match mode {
                            "error" => "fixture command error",
                            "hang" => "timed out",
                            "cancel" => "cancelled",
                            _ => "output bound",
                        }),
                        "{error}"
                    );
                }
                let pid: i32 = fs::read_to_string(build.join("command.pid"))
                    .unwrap()
                    .parse()
                    .unwrap();
                assert_eq!(
                    unsafe { libc::waitpid(pid, std::ptr::null_mut(), libc::WNOHANG) },
                    -1
                );
                assert!(!Path::new(&format!("/proc/{pid}")).exists());
                fs::remove_dir_all(build).unwrap();
            }
        }
    }

    #[tokio::test]
    async fn rootfs_query_fake_bridge_returns_exact_or_absent_sources_and_failures() {
        for mode in ["ok", "absent", "error"] {
            let (build, query, compatibility, mut environment) = fixture();
            environment.insert("ROOTFS_TEST_MODE".into(), mode.into());
            let (_cancel, cancelled) = oneshot::channel();
            let result = acquire(
                query.clone(),
                build.clone(),
                compatibility,
                environment,
                cancelled,
                Duration::from_secs(2),
            )
            .await;
            if mode == "error" {
                assert!(result.unwrap_err().to_string().contains("fixture_error"));
            } else {
                let sources = result.unwrap();
                assert_eq!(sources.query, query);
                assert_eq!(
                    sources.image_rootfs,
                    (mode == "ok").then(|| build.join("retained-rootfs").display().to_string())
                );
            }
            let pid: i32 = fs::read_to_string(build.join("query.pid"))
                .unwrap()
                .parse()
                .unwrap();
            assert_eq!(
                unsafe { libc::waitpid(pid, std::ptr::null_mut(), libc::WNOHANG) },
                -1
            );
            fs::remove_dir_all(build).unwrap();
        }
    }

    #[test]
    fn rootfs_client_ipc_negotiates_retries_only_stale_and_checks_reply_identity() {
        use yoctui_protocol::{
            daemon::*,
            daemon_ipc::{DaemonListener, runtime_paths_for},
        };
        for mode in [
            "normal",
            "old",
            "wrong-instance",
            "wrong-reply",
            "stale",
            "disconnect",
        ] {
            let (build, query, compatibility, _) = fixture();
            fs::create_dir(build.join("runtime")).unwrap();
            fs::set_permissions(build.join("runtime"), fs::Permissions::from_mode(0o700)).unwrap();
            let paths =
                runtime_paths_for(build.join("runtime"), unsafe { libc::geteuid() }).unwrap();
            let listener = DaemonListener::bind(&paths).unwrap();
            let worker_query = query.clone();
            let mut state = DaemonGlobalState::new(
                DaemonModelInstanceId(query.daemon_instance_id.0),
                1,
                "fixture".into(),
                DaemonStateLimits::default(),
            )
            .unwrap();
            state.compatibility = Some(compatibility);
            let snapshot = yoctui_app::daemon_protocol_snapshot(&state);
            let server = std::thread::spawn(move || {
                let mut connection = listener.accept(Duration::from_secs(3)).unwrap();
                connection
                    .set_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                assert!(matches!(
                    connection.receive::<ClientMessage>().unwrap(),
                    ClientMessage::Hello(_)
                ));
                let mut instance = worker_query.daemon_instance_id;
                if mode == "wrong-instance" {
                    instance.0[15] ^= 1;
                }
                connection
                    .send(&ServerMessage::Hello(DaemonHello {
                        selected_version: ProtocolVersion::CURRENT,
                        daemon_instance_id: instance,
                        boot_id: "fixture".into(),
                        capabilities: if mode == "old" {
                            vec![]
                        } else {
                            vec![Capability::RootfsSources]
                        },
                        limits: ProtocolLimits {
                            maximum_frame_bytes: MAX_FRAME_BYTES as u32,
                            maximum_snapshot_bytes: MAX_FRAME_BYTES as u32,
                            maximum_pending_requests: 8,
                            maximum_queue_depth: 16,
                            maximum_terminal_rows: 512,
                            maximum_terminal_columns: 512,
                            maximum_clients: 32,
                            maximum_pty_sessions: 64,
                            maximum_scrollback_lines: 100000,
                            maximum_utility_output_bytes: 4 * 1024 * 1024,
                        },
                    }))
                    .unwrap();
                if matches!(mode, "old" | "wrong-instance") {
                    return;
                }
                let mut commands = 0;
                loop {
                    match connection.receive::<ClientMessage>().unwrap() {
                        ClientMessage::Attach { .. } => connection
                            .send(&ServerMessage::Attached {
                                snapshot: snapshot.clone(),
                                replayed_through: snapshot.sequence,
                            })
                            .unwrap(),
                        ClientMessage::Command(request) => {
                            commands += 1;
                            assert_eq!(request.expected_generation, Some(snapshot.generation));
                            assert_eq!(
                                request.command,
                                DaemonCommand::InspectRootfsSources {
                                    query: worker_query.clone()
                                }
                            );
                            if mode == "disconnect" {
                                return;
                            }
                            let outcome = if mode == "stale" && commands == 1 {
                                CommandOutcome::Rejected {
                                    code: ProtocolErrorCode::StaleGeneration,
                                    message: "refresh".into(),
                                    current_generation: snapshot.generation,
                                }
                            } else {
                                let mut query = worker_query.clone();
                                if mode == "wrong-reply" {
                                    query.request.generation += 1;
                                }
                                CommandOutcome::RootfsSources {
                                    sources: Box::new(RootfsSourcesData {
                                        query,
                                        image_manifest: None,
                                        pkgdata_dir: None,
                                        image_rootfs: Some("/build/retained-rootfs".into()),
                                    }),
                                }
                            };
                            connection
                                .send(&ServerMessage::CommandResult(CommandResult {
                                    request_id: request.request_id,
                                    outcome,
                                }))
                                .unwrap();
                            if mode != "stale" || commands == 2 {
                                break;
                            }
                        }
                        other => panic!("unexpected client message: {other:?}"),
                    }
                }
                assert_eq!(commands, if mode == "stale" { 2 } else { 1 });
            });
            let result = request_sources_at(
                &query,
                &build,
                &yoctui_bitbake::RootfsCompositionCancellation::default(),
                &paths,
            );
            if matches!(mode, "normal" | "stale") {
                assert_eq!(result.unwrap().query, query);
            } else {
                assert!(result.is_err(), "{mode}");
            }
            server.join().unwrap();
            fs::remove_dir_all(build).unwrap();
        }
    }

    #[tokio::test]
    async fn rootfs_query_cancellation_timeout_and_single_worker_bound_reap_bridge() {
        let (build, query, compatibility, mut environment) = fixture();
        environment.insert("ROOTFS_TEST_MODE".into(), "hang".into());
        let permit = Arc::new(Semaphore::new(1));
        let mut pending = PendingQuery::start(
            RequestId(1),
            query.clone(),
            query.daemon_instance_id,
            compatibility.clone(),
            environment.clone(),
            permit.clone(),
        )
        .unwrap();
        assert!(
            PendingQuery::start(
                RequestId(2),
                query.clone(),
                query.daemon_instance_id,
                compatibility.clone(),
                environment.clone(),
                permit.clone()
            )
            .is_err()
        );
        tokio::time::timeout(Duration::from_secs(3), async {
            while !build.join("query.pid").is_file() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        pending.shutdown().await;
        assert!(pending.try_result().unwrap().is_err());
        assert_eq!(permit.available_permits(), 1);
        let (_cancel, cancelled) = oneshot::channel();
        let result = acquire(
            query,
            build.clone(),
            compatibility,
            environment,
            cancelled,
            Duration::from_millis(100),
        )
        .await;
        assert!(result.unwrap_err().to_string().contains("timed out"));
        let pid: i32 = fs::read_to_string(build.join("query.pid"))
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(
            unsafe { libc::waitpid(pid, std::ptr::null_mut(), libc::WNOHANG) },
            -1
        );
        fs::remove_dir_all(build).unwrap();
    }
}
