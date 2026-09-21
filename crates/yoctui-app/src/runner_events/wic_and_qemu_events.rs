pub fn wic_capability_action(capability: WicCapability) -> Action {
    Action::WicCapabilityLoaded(capability)
}

pub fn wic_device_inventory_action(response: WicDeviceInventoryResponse) -> Action {
    Action::WicDeviceInventoryLoaded {
        request: response.request,
        devices: response.devices,
        limitations: response.limitations,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WicSessionEvent {
    Starting,
    Started,
    Output {
        stream: WicOutputStream,
        line: String,
        truncated: bool,
    },
    Completed {
        exit_code: i32,
        outputs: Vec<WicOutput>,
        limitations: Vec<String>,
    },
    Failed {
        message: String,
        exit_code: Option<i32>,
    },
    Cancelled {
        forced: bool,
        exit_code: Option<i32>,
    },
    CancellationRejected {
        message: String,
    },
    Lost {
        message: String,
    },
}

pub fn wic_actions_for_session_event(
    id: WicSessionId,
    event: WicSessionEvent,
    timestamp: SystemTime,
) -> Vec<Action> {
    match event {
        WicSessionEvent::Starting => vec![Action::WicSessionStarting {
            id,
            started_at: timestamp,
        }],
        WicSessionEvent::Started => vec![Action::WicSessionRunning { id }],
        WicSessionEvent::Output {
            stream,
            line,
            truncated,
        } => vec![Action::AppendWicSessionOutput {
            id,
            stream,
            line,
            truncated,
            timestamp,
        }],
        WicSessionEvent::Completed {
            exit_code,
            outputs,
            limitations,
        } => vec![Action::CompleteWicSession {
            id,
            exit_code,
            outputs,
            limitations,
            finished_at: timestamp,
        }],
        WicSessionEvent::Failed { message, exit_code } => vec![Action::FailWicSession {
            id,
            message,
            exit_code,
            finished_at: timestamp,
        }],
        WicSessionEvent::Cancelled { forced, exit_code } => {
            let mut actions = Vec::new();
            if forced {
                actions.push(Action::AppendWicSessionOutput {
                    id,
                    stream: WicOutputStream::Stderr,
                    line: "Wic cancellation required forced termination".into(),
                    truncated: false,
                    timestamp,
                });
            }
            actions.push(Action::CancelWicSession {
                id,
                exit_code,
                finished_at: timestamp,
            });
            actions
        }
        WicSessionEvent::CancellationRejected { message } => {
            vec![Action::RejectWicSessionCancellation { id, message }]
        }
        WicSessionEvent::Lost { message } => vec![Action::LoseWicSession {
            id,
            message,
            finished_at: timestamp,
        }],
    }
}

pub fn wic_actions_for_runner_event(
    id: WicSessionId,
    event: WicRunnerEvent,
    timestamp: SystemTime,
) -> Vec<Action> {
    let event = match event {
        WicRunnerEvent::Starting => WicSessionEvent::Starting,
        WicRunnerEvent::Started => WicSessionEvent::Started,
        WicRunnerEvent::Output {
            stream,
            line,
            truncated,
        } => WicSessionEvent::Output {
            stream: match stream {
                WicRunnerOutputStream::Stdout => WicOutputStream::Stdout,
                WicRunnerOutputStream::Stderr => WicOutputStream::Stderr,
            },
            line,
            truncated,
        },
        WicRunnerEvent::Completed {
            exit_code,
            outputs,
            limitations,
        } => WicSessionEvent::Completed {
            exit_code,
            outputs,
            limitations,
        },
        WicRunnerEvent::Failed { message, exit_code } => {
            WicSessionEvent::Failed { message, exit_code }
        }
        WicRunnerEvent::Cancelled { forced, exit_code } => {
            WicSessionEvent::Cancelled { forced, exit_code }
        }
        WicRunnerEvent::CancellationRejected { message } => {
            WicSessionEvent::CancellationRejected { message }
        }
        WicRunnerEvent::Lost { message } => WicSessionEvent::Lost { message },
    };
    wic_actions_for_session_event(id, event, timestamp)
}

pub fn qemu_actions_for_runner_event(
    id: QemuSessionId,
    event: QemuRunnerEvent,
    timestamp: SystemTime,
) -> Vec<Action> {
    match event {
        QemuRunnerEvent::Starting => vec![Action::QemuSessionStarting {
            id,
            started_at: timestamp,
        }],
        QemuRunnerEvent::Started => vec![Action::QemuSessionRunning { id }],
        QemuRunnerEvent::Output {
            stream,
            line,
            truncated,
        } => vec![Action::AppendQemuSessionOutput {
            id,
            stream: match stream {
                QemuRunnerOutputStream::Stdout => QemuOutputStream::Stdout,
                QemuRunnerOutputStream::Stderr => QemuOutputStream::Stderr,
            },
            line,
            truncated,
            timestamp,
        }],
        QemuRunnerEvent::Completed { exit_code } => vec![Action::CompleteQemuSession {
            id,
            exit_code,
            finished_at: timestamp,
        }],
        QemuRunnerEvent::Failed { message, exit_code } => vec![Action::FailQemuSession {
            id,
            message,
            exit_code,
            finished_at: timestamp,
        }],
        QemuRunnerEvent::Cancelled { forced, exit_code } => {
            let mut actions = Vec::new();
            if forced {
                actions.push(Action::AppendQemuSessionOutput {
                    id,
                    stream: QemuOutputStream::Stderr,
                    line: "runqemu cancellation required forced termination".into(),
                    truncated: false,
                    timestamp,
                });
            }
            actions.push(Action::CancelQemuSession {
                id,
                exit_code,
                finished_at: timestamp,
            });
            actions
        }
        QemuRunnerEvent::CancellationRejected { message } => {
            vec![Action::RejectQemuSessionCancellation { id, message }]
        }
        QemuRunnerEvent::Lost { message } => vec![Action::LoseQemuSession {
            id,
            message,
            finished_at: timestamp,
        }],
    }
}
