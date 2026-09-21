use std::{collections::HashMap, sync::mpsc::SyncSender, time::Duration};

use yoctui_model::{PtyClientId, PtyDimensions, PtySessionId};

mod child_runtime;
mod request_validation;
mod supervisor;
mod terminal_mapping;

const PTY_SCREEN_MIN_INTERVAL: Duration = Duration::from_millis(33);
const PTY_TERMINATION_TIMEOUT: Duration = Duration::from_secs(2);
const PTY_CONTROL_RESPONSE_TIMEOUT: Duration = Duration::from_secs(5);
const GENERIC_PTY_ID_LIMIT: u64 = 1 << 60;
const RAW_PTY_NAMESPACE: u64 = 6 << 60;

#[derive(Debug)]
enum Control {
    Attach(PtyClientId),
    Detach(PtyClientId),
    Take(PtyClientId, u64),
    Release(PtyClientId, u64),
    Input(PtyClientId, u64, Vec<u8>),
    Resize(PtyClientId, u64, PtyDimensions),
    Snapshot(usize),
    Rename(String),
    Terminate,
}

#[derive(Debug)]
enum Response {
    Epoch(u64),
    Unit,
    Screen(Box<yoctui_protocol::daemon::PtyScreenSnapshot>),
}

type ControlReply = SyncSender<Result<Response, String>>;
type ControlMessage = (Control, ControlReply);

#[derive(Debug)]
pub enum DaemonPtyEvent {
    Started {
        session_id: PtySessionId,
        snapshot: yoctui_protocol::daemon::PtySessionSummary,
    },
    Output {
        session_id: PtySessionId,
        bytes: Vec<u8>,
        screen: Option<yoctui_protocol::daemon::PtyScreenSnapshot>,
    },
    Exited {
        session_id: PtySessionId,
        exit_code: Option<i32>,
        screen: Option<yoctui_protocol::daemon::PtyScreenSnapshot>,
    },
    Lost {
        session_id: PtySessionId,
        message: String,
    },
    Changed {
        session_id: PtySessionId,
        snapshot: yoctui_protocol::daemon::PtySessionSummary,
    },
}

struct SessionHandle {
    control: tokio::sync::mpsc::UnboundedSender<ControlMessage>,
}

pub struct DaemonPtySupervisor {
    sessions: HashMap<PtySessionId, SessionHandle>,
    tx: tokio::sync::mpsc::UnboundedSender<DaemonPtyEvent>,
    rx: tokio::sync::mpsc::UnboundedReceiver<DaemonPtyEvent>,
}

#[cfg(test)]
use request_validation::{ensure_interactive_terminal_environment, wire_spec};
#[cfg(test)]
use terminal_mapping::terminal_to_wire;

#[cfg(test)]
#[path = "tests/daemon_pty/mod.rs"]
mod tests;
