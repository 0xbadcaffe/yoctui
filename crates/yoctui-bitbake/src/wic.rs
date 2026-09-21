use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    ffi::OsString,
    fs,
    path::{Component, Path, PathBuf},
    process::Stdio,
    time::{Duration, Instant},
};

use crate::{WicRunnerEvent, WicRunnerOutputStream, output_text};
use serde::Deserialize;
use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, BufReader},
    process::{Child, Command},
};
use yoctui_model::{
    MAX_WIC_DEVICE_MOUNTS, MAX_WIC_DEVICES, MAX_WIC_KICKSTARTS, MAX_WIC_LIMITATIONS,
    MAX_WIC_SOURCE_BYTES, WicCapability, WicCreatePreview, WicCreateRequest, WicDevice,
    WicDeviceIdentity, WicDeviceInventoryRequest, WicKickstart, WicKickstartIdentity, WicOutput,
    WicOutputIdentity, WicOutputKind, WicPartitionSummary, WicWriteRequest,
    normalize_wic_capability, normalize_wic_devices, normalize_wic_limitations,
};
use yoctui_utils::is_transient_spawn_error;

const MAX_WIC_LIST_BYTES: u64 = 256 * 1024;
const WIC_INSPECTION_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_WIC_LINE_BYTES: usize = 64 * 1024;
const MAX_WIC_OUTPUT_ENTRIES: usize = 4_096;
const WIC_EVENT_CHANNEL_CAPACITY: usize = 256;
const MAX_WIC_DEVICE_JSON_BYTES: u64 = 1024 * 1024;
const MAX_WIC_DEVICE_RECORDS: usize = 512;
const MAX_WIC_DEVICE_PATH_BYTES: usize = 4_096;
const WIC_DEVICE_INSPECTION_TIMEOUT: Duration = Duration::from_secs(10);
const WIC_SPAWN_ATTEMPTS: usize = 4;
const WIC_SPAWN_RETRY_DELAY: Duration = Duration::from_millis(5);
type WicOutputSnapshot = BTreeMap<PathBuf, (u64, u128)>;
type WicOutputScan = (WicOutputSnapshot, Vec<String>);

include!("wic/capability_and_kickstart.rs");
include!("wic/commands_and_device_types.rs");
include!("wic/device_discovery.rs");
include!("wic/job_runner.rs");
include!("wic/output_scan.rs");

#[cfg(test)]
#[path = "tests/wic/mod.rs"]
mod tests;
