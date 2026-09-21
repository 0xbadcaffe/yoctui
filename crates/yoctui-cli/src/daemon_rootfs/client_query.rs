use anyhow::{Context, Result, ensure};
use std::{path::Path, time::Duration};
use yoctui_protocol::{
    daemon::DaemonInstanceId,
    rootfs::{RootfsSourcesData, RootfsSourcesRequestData},
};

use super::QUERY_TIMEOUT;

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

pub(super) fn request_sources_at(
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
