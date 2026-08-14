use yoctui_bitbake::{MaintenanceSstateCapabilityInput, MaintenanceSstateCapabilityInspector};
use yoctui_protocol::daemon::DaemonMaintenanceSnapshot;

pub fn inspect(
    request: u64,
    build_directory: String,
    sstate_directory: Option<String>,
    tmp_directory: Option<String>,
    stamps_directories: Vec<String>,
    executable_search_path: Vec<String>,
) -> Result<DaemonMaintenanceSnapshot, String> {
    if request == 0 {
        return Err("maintenance capability request is invalid".into());
    }
    let snapshot =
        MaintenanceSstateCapabilityInspector::inspect(MaintenanceSstateCapabilityInput {
            build_dir: build_directory.into(),
            sstate_dir: sstate_directory.map(Into::into),
            tmp_dir: tmp_directory.map(Into::into),
            stamps_dirs: stamps_directories.into_iter().map(Into::into).collect(),
            executable_search_path: executable_search_path.into_iter().map(Into::into).collect(),
        })
        .map_err(|error| error.to_string())?;
    Ok(DaemonMaintenanceSnapshot {
        request,
        tools: snapshot
            .tools
            .iter()
            .map(|tool| format!("{:?}", tool.tool()))
            .collect(),
        limitations: snapshot.limitations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_runtime_maintenance_rejects_invalid_request() {
        assert!(inspect(0, "/build".into(), None, None, Vec::new(), Vec::new()).is_err());
    }
}
