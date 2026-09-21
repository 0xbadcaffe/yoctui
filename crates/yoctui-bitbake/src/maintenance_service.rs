use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    net::{IpAddr, SocketAddr, TcpStream},
    path::{Path, PathBuf},
    time::Duration,
};

use thiserror::Error;
use yoctui_model::{
    MAX_MAINTENANCE_LIMITATIONS, MAX_MAINTENANCE_OUTPUT, MAX_MAINTENANCE_PATHS,
    MAX_MAINTENANCE_TEXT_BYTES, MaintenanceCapabilitySnapshot, MaintenanceFileIdentity,
    MaintenanceMetadata, MaintenanceOperationPreview, MaintenanceSessionId, MaintenanceTool,
    MaintenanceToolCapability, MaintenanceToolInterface, PrServiceRequest, ServiceDiagnostic,
    ServiceEndpointDiagnostic, ServiceEndpointRole, ServiceKind, ServiceLocation,
    ServiceProcessEvidence, ServiceReachability, ServiceState,
};

use crate::maintenance_sstate::{MaintenanceSstateAdapterError, MaintenanceSstateCommandSpec};

const MAX_PROCESS_ENTRIES: usize = 4_096;
const MAX_PROCESS_NAME_BYTES: usize = 256;
const MAX_ENDPOINT_PROBE_TIMEOUT: Duration = Duration::from_secs(5);

include!("maintenance_service/capability_inspection.rs");
include!("maintenance_service/command_and_process_scan.rs");
include!("maintenance_service/endpoint_inspection.rs");

#[cfg(test)]
#[path = "tests/maintenance_service/mod.rs"]
mod tests;
