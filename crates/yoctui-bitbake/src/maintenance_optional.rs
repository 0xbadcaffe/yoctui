use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use thiserror::Error;
use yoctui_model::{
    MAX_MAINTENANCE_LIMITATIONS, MAX_MAINTENANCE_OUTPUT, MAX_MAINTENANCE_PATHS,
    MAX_MAINTENANCE_TEXT_BYTES, MaintenanceCapabilitySnapshot, MaintenanceFileIdentity,
    MaintenanceIntegrationsSnapshot, MaintenanceMetadata, MaintenanceTool,
    MaintenanceToolCapability, MaintenanceToolInterface, ServiceProcessEvidence,
};
pub use yoctui_model::{
    MaintenanceDirectoryIdentity, MaintenanceGitWorktreeIdentity, OptionalErrorReportIntegration,
    OptionalIntegrationState, OptionalPullRequestIntegration, OptionalRepoManifestIntegration,
    OptionalToasterIntegration,
};

const MAX_PROCESS_ENTRIES: usize = 4_096;
const MAX_PROCESS_BYTES: usize = 512;

include!("maintenance_optional/capability_inspection.rs");
include!("maintenance_optional/tool_discovery.rs");
include!("maintenance_optional/identity_validation.rs");

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[cfg(unix)]
    use std::os::unix::fs::symlink;

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "yoctui-maintenance-optional-{name}-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(fs::canonicalize(path).unwrap())
        }

        fn join(&self, path: &str) -> PathBuf {
            self.0.join(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn executable(path: &Path) {
        crate::test_support::write_executable(path, "#!/bin/sh\nexit 0\n");
    }

    fn process(root: &Path, pid: u32, comm: &str, cmdline: &[u8]) {
        let directory = root.join(pid.to_string());
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("comm"), format!("{comm}\n")).unwrap();
        fs::write(directory.join("cmdline"), cmdline).unwrap();
    }

    fn complete_fixture(fixture: &TestDirectory) -> MaintenanceOptionalCapabilityInput {
        for directory in [
            "build",
            "tools",
            "work/.git",
            "repo/.repo",
            "proc",
            "config",
        ] {
            fs::create_dir_all(fixture.join(directory)).unwrap();
        }
        for tool in [
            "create-pull-request",
            "send-pull-request",
            "send-error-report",
            "toaster",
            "repo",
        ] {
            executable(&fixture.join(&format!("tools/{tool}")));
        }
        fs::write(fixture.join("work/.git/HEAD"), "ref: refs/heads/main\n").unwrap();
        fs::write(fixture.join("report.json"), "{}\n").unwrap();
        fs::write(fixture.join("repo/.repo/manifest.xml"), "<manifest/>\n").unwrap();
        fs::write(fixture.join("config/toaster.conf"), "setting=true\n").unwrap();
        process(
            &fixture.join("proc"),
            42,
            "sh",
            b"/bin/sh\0/tools/toaster\0start\0",
        );
        MaintenanceOptionalCapabilityInput {
            build_dir: fixture.join("build"),
            executable_search_path: vec![fixture.join("tools")],
            git_worktree_candidates: vec![fixture.join("work")],
            error_report_candidates: vec![fixture.join("report.json")],
            repo_workspace_candidates: vec![fixture.join("repo")],
            toaster_configuration_candidates: vec![fixture.join("config/toaster.conf")],
            process_root: fixture.join("proc"),
        }
    }

    #[test]
    fn maintenance_optional_detects_complete_capabilities_without_side_effects() {
        let fixture = TestDirectory::new("complete");
        let inspection =
            MaintenanceOptionalCapabilityInspector::inspect(complete_fixture(&fixture)).unwrap();
        assert_eq!(
            inspection.pull_request.state,
            OptionalIntegrationState::Available
        );
        assert_eq!(
            inspection.error_report.state,
            OptionalIntegrationState::Available
        );
        assert_eq!(
            inspection.repo_manifest.state,
            OptionalIntegrationState::Available
        );
        assert_eq!(
            inspection.toaster.state,
            OptionalIntegrationState::Available
        );
        assert_eq!(inspection.toaster.observed_processes.len(), 1);
        assert_eq!(inspection.toaster.observed_processes[0].pid, 42);
        assert!(
            inspection
                .toaster
                .limitations
                .iter()
                .any(|value| value.contains("observational"))
        );
        assert!(
            inspection
                .capability
                .tools
                .iter()
                .all(|capability| matches!(
                    capability,
                    MaintenanceToolCapability::Available {
                        interface: MaintenanceToolInterface::DetectionOnly,
                        ..
                    }
                ))
        );
        inspection.revalidate().unwrap();
    }

    #[test]
    fn maintenance_optional_preserves_missing_and_partial_states() {
        let fixture = TestDirectory::new("partial");
        let mut input = complete_fixture(&fixture);
        fs::remove_file(fixture.join("tools/send-pull-request")).unwrap();
        fs::remove_file(fixture.join("report.json")).unwrap();
        fs::remove_file(fixture.join("tools/repo")).unwrap();
        fs::remove_file(fixture.join("config/toaster.conf")).unwrap();
        input.toaster_configuration_candidates.clear();
        let inspection = MaintenanceOptionalCapabilityInspector::inspect(input).unwrap();
        assert_eq!(
            inspection.pull_request.state,
            OptionalIntegrationState::Partial
        );
        assert_eq!(
            inspection.error_report.state,
            OptionalIntegrationState::Partial
        );
        assert_eq!(
            inspection.repo_manifest.state,
            OptionalIntegrationState::Partial
        );
        assert_eq!(inspection.toaster.state, OptionalIntegrationState::Partial);
        assert!(
            inspection
                .pull_request
                .limitations
                .iter()
                .any(|value| value.contains("send-pull-request"))
        );
    }

    #[test]
    fn maintenance_optional_reports_fully_unavailable_inputs() {
        let fixture = TestDirectory::new("missing");
        for directory in ["build", "tools", "proc"] {
            fs::create_dir_all(fixture.join(directory)).unwrap();
        }
        let inspection =
            MaintenanceOptionalCapabilityInspector::inspect(MaintenanceOptionalCapabilityInput {
                build_dir: fixture.join("build"),
                executable_search_path: vec![fixture.join("tools")],
                git_worktree_candidates: Vec::new(),
                error_report_candidates: Vec::new(),
                repo_workspace_candidates: Vec::new(),
                toaster_configuration_candidates: Vec::new(),
                process_root: fixture.join("proc"),
            })
            .unwrap();
        assert_eq!(
            inspection.pull_request.state,
            OptionalIntegrationState::Unavailable
        );
        assert_eq!(
            inspection.error_report.state,
            OptionalIntegrationState::Unavailable
        );
        assert_eq!(
            inspection.repo_manifest.state,
            OptionalIntegrationState::Unavailable
        );
        assert_eq!(
            inspection.toaster.state,
            OptionalIntegrationState::Unavailable
        );
        assert!(
            inspection
                .capability
                .tools
                .iter()
                .all(|capability| matches!(
                    capability,
                    MaintenanceToolCapability::Unavailable { .. }
                ))
        );
    }

    #[cfg(unix)]
    #[test]
    fn maintenance_optional_rejects_symlinked_helpers_and_escaped_manifests() {
        let fixture = TestDirectory::new("unsafe");
        let input = complete_fixture(&fixture);
        fs::rename(
            fixture.join("tools/create-pull-request"),
            fixture.join("real-create"),
        )
        .unwrap();
        symlink(
            fixture.join("real-create"),
            fixture.join("tools/create-pull-request"),
        )
        .unwrap();
        fs::remove_file(fixture.join("repo/.repo/manifest.xml")).unwrap();
        fs::write(fixture.join("outside.xml"), "<manifest/>\n").unwrap();
        symlink(
            fixture.join("outside.xml"),
            fixture.join("repo/.repo/manifest.xml"),
        )
        .unwrap();
        let inspection = MaintenanceOptionalCapabilityInspector::inspect(input).unwrap();
        assert!(
            !inspection
                .capability
                .supports(MaintenanceTool::CreatePullRequest)
        );
        assert_eq!(
            inspection.pull_request.state,
            OptionalIntegrationState::Partial
        );
        assert_eq!(
            inspection.repo_manifest.state,
            OptionalIntegrationState::Partial
        );
        assert!(
            inspection
                .limitations
                .iter()
                .any(|value| value.contains("unsafe executable"))
        );
        assert!(
            inspection
                .limitations
                .iter()
                .any(|value| value.contains("unsafe repo workspace"))
        );
    }

    #[test]
    fn maintenance_optional_revalidation_rejects_tampered_evidence() {
        let fixture = TestDirectory::new("tampered");
        let inspection =
            MaintenanceOptionalCapabilityInspector::inspect(complete_fixture(&fixture)).unwrap();
        fs::write(fixture.join("report.json"), "{\"changed\":true}\n").unwrap();
        assert!(matches!(
            inspection.revalidate(),
            Err(MaintenanceOptionalAdapterError::StaleEvidence(path))
                if path == fixture.join("report.json")
        ));
    }

    #[test]
    fn maintenance_optional_bounds_candidates_and_process_records() {
        let fixture = TestDirectory::new("bounds");
        let mut input = complete_fixture(&fixture);
        input.git_worktree_candidates = vec![fixture.join("missing"); MAX_MAINTENANCE_PATHS + 1];
        for pid in 1..=(MAX_MAINTENANCE_OUTPUT as u32 + 1) {
            process(&fixture.join("proc"), pid + 100, "toaster", b"toaster\0");
        }
        let inspection = MaintenanceOptionalCapabilityInspector::inspect(input).unwrap();
        assert_eq!(
            inspection.toaster.observed_processes.len(),
            MAX_MAINTENANCE_OUTPUT
        );
        assert!(
            inspection
                .limitations
                .iter()
                .any(|value| value.contains("Git worktree candidates reached"))
        );
    }

    #[test]
    fn maintenance_optional_process_evidence_is_observational_only() {
        let fixture = TestDirectory::new("process");
        let mut input = complete_fixture(&fixture);
        fs::remove_file(fixture.join("tools/toaster")).unwrap();
        fs::remove_file(fixture.join("config/toaster.conf")).unwrap();
        input.toaster_configuration_candidates.clear();
        let inspection = MaintenanceOptionalCapabilityInspector::inspect(input).unwrap();
        assert_eq!(
            inspection.toaster.state,
            OptionalIntegrationState::Unavailable
        );
        assert!(!inspection.toaster.observed_processes.is_empty());
        assert!(!inspection.capability.supports(MaintenanceTool::Toaster));
    }
}
