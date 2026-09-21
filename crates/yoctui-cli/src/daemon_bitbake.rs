use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize},
    },
    time::Duration,
};

use tokio::sync::mpsc;
use yoctui_bitbake::BackendEvent;
use yoctui_model::DaemonCompatibilitySnapshot;
use yoctui_protocol::daemon::JobId;

mod cancellation;
mod ingress;
mod lifecycle;
mod notification;

use notification::ActivityNotification;

const DEFAULT_CANCELLATION_TERMINAL_TIMEOUT: Duration = Duration::from_secs(3);
const BITBAKE_RELIABLE_EVENT_CAPACITY: usize = 512;
const BITBAKE_COSMETIC_EVENT_CAPACITY: usize = 512;
const COSMETIC_ACTIVITY_MIN_INTERVAL: Duration = Duration::from_millis(30);

#[derive(Debug, Clone)]
pub enum DaemonBitBakeEvent {
    Backend {
        job_id: JobId,
        event: Box<BackendEvent>,
    },
    Failed {
        job_id: JobId,
        message: String,
    },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DaemonBitBakePressure {
    pub reliable_enqueued: u64,
    pub cosmetic_enqueued: u64,
    pub cosmetic_dropped: u64,
    pub reliable_waits: u64,
    pub cancellation_stale_discarded: u64,
    pub maximum_queue_depth: usize,
    pub current_queue_depth: usize,
}

#[derive(Debug, Default)]
struct DaemonBitBakePressureShared {
    reliable_enqueued: AtomicU64,
    cosmetic_enqueued: AtomicU64,
    cosmetic_dropped: AtomicU64,
    reliable_waits: AtomicU64,
    cancellation_stale_discarded: AtomicU64,
    maximum_queue_depth: AtomicUsize,
}

pub struct DaemonBitBakeSupervisor {
    job_ids: crate::daemon_job_ids::DaemonJobIds,
    active: HashMap<JobId, mpsc::UnboundedSender<()>>,
    reliable_tx: mpsc::Sender<DaemonBitBakeEvent>,
    reliable_rx: mpsc::Receiver<DaemonBitBakeEvent>,
    cosmetic_tx: mpsc::Sender<DaemonBitBakeEvent>,
    cosmetic_rx: mpsc::Receiver<DaemonBitBakeEvent>,
    cancellation_terminal_tx: mpsc::Sender<DaemonBitBakeEvent>,
    cancellation_terminal_rx: mpsc::Receiver<DaemonBitBakeEvent>,
    post_cancellation_diagnostics: VecDeque<DaemonBitBakeEvent>,
    pressure: Arc<DaemonBitBakePressureShared>,
    compatibility: Option<DaemonCompatibilitySnapshot>,
    bridge_environment: Option<BTreeMap<String, String>>,
    cancellation_terminal_timeout: Duration,
    activity: Option<ActivityNotification>,
}

#[cfg(test)]
#[path = "tests/daemon_bitbake/mod.rs"]
mod tests;
