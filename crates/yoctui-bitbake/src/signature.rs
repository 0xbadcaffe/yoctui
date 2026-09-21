use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use thiserror::Error;
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::Command,
    sync::Notify,
};
use yoctui_model::{
    DaemonCompatibilitySnapshot, MAX_SIGNATURE_DIFFERENCES, MAX_SIGNATURE_RECORDS,
    SignatureComparisonRequest, SignatureDifference, SignatureDifferenceCategory,
    SignatureIdentity, SignatureRecord, SignatureTarget, SignatureValue, compare_signature_records,
    normalize_signature_differences, normalize_signature_records,
};

use crate::BitBakeCommandPlanner;

const MAX_SIGNATURE_OUTPUT_BYTES: usize = 8 * 1024 * 1024;
const MAX_SIGNATURE_VARIABLES: usize = 4096;
const MAX_SIGNATURE_DEPENDENCIES: usize = 4096;
const MAX_SIGNATURE_LIMITATIONS: usize = 64;
const MAX_SIGNATURE_SCAN_ENTRIES: usize = 100_000;
const SIGNATURE_COMMAND_TIMEOUT: Duration = Duration::from_secs(120);

include!("signature/types_and_adapter.rs");
include!("signature/path_discovery.rs");
include!("signature/process_io.rs");
include!("signature/output_parsing.rs");

#[cfg(test)]
#[path = "tests/signature/mod.rs"]
mod tests;
