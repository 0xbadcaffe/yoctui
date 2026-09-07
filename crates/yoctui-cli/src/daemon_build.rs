//! Safe optimistic retries for one CLI build submission, never for lost replies.
use anyhow::{Context, Result, bail, ensure};
use std::time::{Duration, Instant};
use yoctui_protocol::{daemon::*, daemon_ipc::DaemonConnection};

const MAX_SUBMISSIONS: u64 = 3;
const MAX_INTERLEAVED_MESSAGES: usize = 4096;
const RECEIVE_DEADLINE: Duration = Duration::from_secs(10);

fn response(connection: &mut DaemonConnection) -> Result<ServerMessage> {
    let deadline = Instant::now() + RECEIVE_DEADLINE;
    for _ in 0..MAX_INTERLEAVED_MESSAGES {
        let remaining = deadline.saturating_duration_since(Instant::now());
        ensure!(
            !remaining.is_zero(),
            "daemon build reply deadline exceeded; inspect daemon status before retrying"
        );
        connection.set_read_timeout(Some(remaining))?;
        match connection.receive::<ServerMessage>()
            .context("daemon build reply unavailable; submission may have been accepted, inspect daemon status before retrying")? {
            ServerMessage::Event(_) | ServerMessage::Snapshot(_) | ServerMessage::ResyncRequired { .. } => {},
            ServerMessage::Ping { nonce, .. } => connection.send(&ClientMessage::Pong { nonce })?,
            ServerMessage::Error(error) => bail!("daemon build protocol error {:?}: {}", error.code, error.message),
            message => return Ok(message),
        }
    }
    bail!(
        "daemon build reply exceeded the interleaved-event bound; inspect daemon status before retrying"
    )
}

pub fn start(
    connection: &mut DaemonConnection,
    initial: DaemonSnapshot,
    targets: Vec<String>,
) -> Result<CommandOutcome> {
    let mut generation = initial.generation;
    for attempt in 1..=MAX_SUBMISSIONS {
        let request_id = RequestId(attempt);
        connection
            .send(&ClientMessage::Command(CommandRequest {
                request_id,
                expected_generation: Some(generation),
                command: DaemonCommand::StartBuild {
                    targets: targets.clone(),
                    task: None,
                    force: false,
                },
            }))
            .context("daemon build send failed; inspect daemon status before retrying")?;
        let ServerMessage::CommandResult(result) = response(connection)? else {
            bail!("unexpected daemon build reply; submission is not safe to retry automatically");
        };
        ensure!(
            result.request_id == request_id,
            "daemon build reply belongs to a different request; not retrying"
        );
        match result.outcome {
            outcome @ (CommandOutcome::Accepted | CommandOutcome::Completed) => return Ok(outcome),
            CommandOutcome::Rejected {
                code: ProtocolErrorCode::StaleGeneration,
                current_generation,
                ..
            } if attempt < MAX_SUBMISSIONS => {
                connection.send(&ClientMessage::Attach {
                    workspace: None,
                    resume: None,
                    subscription: Subscription {
                        state: true,
                        jobs: true,
                        logs: true,
                        pty_sessions: vec![],
                    },
                })?;
                let ServerMessage::Attached { snapshot, .. } = response(connection)? else {
                    bail!("daemon did not refresh the stale build snapshot");
                };
                ensure!(
                    snapshot.daemon_instance_id == initial.daemon_instance_id
                        && snapshot.workspace == initial.workspace
                        && snapshot.compatibility == initial.compatibility,
                    "daemon build environment or authority changed; inspect it before retrying"
                );
                ensure!(
                    snapshot.generation >= current_generation,
                    "daemon returned an outdated build snapshot"
                );
                generation = snapshot.generation;
            }
            CommandOutcome::Rejected { code, message, .. } => bail!(
                "daemon build rejected ({code:?}): {message} (attempt {attempt}/{MAX_SUBMISSIONS})"
            ),
            CommandOutcome::ConfirmationRequired { .. } => bail!(
                "daemon build requires explicit interactive confirmation; no build was automatically confirmed"
            ),
        }
    }
    unreachable!("the final rejection cannot retry")
}
