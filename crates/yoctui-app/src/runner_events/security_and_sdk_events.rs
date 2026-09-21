pub fn security_actions_for_mapper_event(
    event: SecurityMapperRunnerEvent,
    timestamp: SystemTime,
) -> Vec<Action> {
    let security = |action| Action::Security(action);
    match event {
        SecurityMapperRunnerEvent::Started { id } => {
            vec![security(SecurityAction::SessionRunning(id))]
        }
        SecurityMapperRunnerEvent::Output {
            id,
            stream,
            line,
            truncated,
        } => vec![security(SecurityAction::SessionOutput {
            id,
            stream,
            line,
            truncated,
        })],
        SecurityMapperRunnerEvent::Completed { id, .. } => {
            vec![security(SecurityAction::CompleteSession {
                id,
                result_paths: Vec::new(),
                finished_at: timestamp,
            })]
        }
        SecurityMapperRunnerEvent::Failed { id, exit_code } => {
            vec![security(SecurityAction::FailSession {
                id,
                message: exit_code.map_or_else(
                    || "Security package mapping failed without an exit code".into(),
                    |code| format!("Security package mapping failed with exit code {code}"),
                ),
                finished_at: timestamp,
            })]
        }
        SecurityMapperRunnerEvent::CancellationRequested { .. } => Vec::new(),
        SecurityMapperRunnerEvent::Cancelled {
            id,
            forced,
            exit_code: _,
        } => {
            let mut actions = Vec::new();
            if forced {
                actions.push(security(SecurityAction::SessionOutput {
                    id,
                    stream: SecurityOutputStream::Stderr,
                    line: "Security package mapping required forced termination".into(),
                    truncated: false,
                }));
            }
            actions.push(security(SecurityAction::CancelSession {
                id,
                finished_at: timestamp,
            }));
            actions
        }
        SecurityMapperRunnerEvent::CancellationRejected { id, message } => {
            vec![security(SecurityAction::RejectCancellation { id, message })]
        }
        SecurityMapperRunnerEvent::TimedOut { id, forced, .. } => {
            let mode = if forced { "forced" } else { "graceful" };
            vec![
                security(SecurityAction::SessionOutput {
                    id,
                    stream: SecurityOutputStream::Stderr,
                    line: format!(
                        "Security package mapping timed out; {mode} termination was used"
                    ),
                    truncated: false,
                }),
                security(SecurityAction::TimeoutSession {
                    id,
                    finished_at: timestamp,
                }),
            ]
        }
        SecurityMapperRunnerEvent::Lost { id, message } => {
            vec![security(SecurityAction::LoseSession {
                id,
                message,
                finished_at: timestamp,
            })]
        }
    }
}

pub fn sdk_actions_for_runner_event(
    id: SdkSessionId,
    event: SdkToolRunnerEvent,
    timestamp: SystemTime,
) -> Vec<Action> {
    match event {
        SdkToolRunnerEvent::Started => vec![
            Action::SdkSessionStarting {
                id,
                started_at: timestamp,
            },
            Action::SdkSessionRunning { id },
        ],
        SdkToolRunnerEvent::Output {
            stream,
            line,
            truncated,
        } => vec![Action::AppendSdkSessionOutput {
            id,
            stream,
            line,
            truncated,
            timestamp,
        }],
        SdkToolRunnerEvent::Completed { exit_code } => vec![Action::CompleteSdkSession {
            id,
            exit_code: exit_code.unwrap_or(0),
            artifacts: Vec::new(),
            finished_at: timestamp,
        }],
        SdkToolRunnerEvent::Failed { exit_code } => vec![Action::FailSdkSession {
            id,
            message: exit_code.map_or_else(
                || "SDK tool exited unsuccessfully without an exit code".into(),
                |code| format!("SDK tool exited unsuccessfully with exit code {code}"),
            ),
            exit_code,
            finished_at: timestamp,
        }],
        SdkToolRunnerEvent::Cancelled { forced, exit_code } => {
            let mut actions = Vec::new();
            if forced {
                actions.push(Action::AppendSdkSessionOutput {
                    id,
                    stream: SdkOutputStream::Stderr,
                    line: "SDK tool cancellation required forced termination".into(),
                    truncated: false,
                    timestamp,
                });
            }
            actions.push(Action::CancelSdkSession {
                id,
                exit_code,
                finished_at: timestamp,
            });
            actions
        }
        SdkToolRunnerEvent::CancellationRejected { message } => {
            vec![Action::RejectSdkSessionCancellation { id, message }]
        }
        SdkToolRunnerEvent::TimedOut { forced, exit_code } => {
            let mode = if forced { "forced" } else { "graceful" };
            vec![Action::FailSdkSession {
                id,
                message: format!("SDK tool timed out; {mode} termination was used"),
                exit_code,
                finished_at: timestamp,
            }]
        }
        SdkToolRunnerEvent::Lost { message } => vec![Action::LoseSdkSession {
            id,
            message,
            finished_at: timestamp,
        }],
    }
}
