use super::*;
use crate::{CompatibilityFixtureRole, release_capability_fixtures};
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use std::os::unix::fs::{PermissionsExt, symlink};
use yoctui_model::{
    AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
    CapabilityImplementation, CapabilityImplementationKind, CapabilityRecord, CapabilitySnapshot,
    CapabilityState, IdentityAuthority, ToolIdentity, YoctoEnvironmentIdentity,
};

fn compatibility(build: &Path, tool: &Path) -> DaemonCompatibilitySnapshot {
    let capabilities = [
        (
            CapabilityId::PkgDataGenerated,
            "pkgdata.generated",
            CapabilityImplementationKind::ProcessAdapter,
        ),
        (
            CapabilityId::PkgDataListPackages,
            PKGDATA_LIST_PACKAGES_IMPLEMENTATION,
            CapabilityImplementationKind::Command,
        ),
        (
            CapabilityId::PkgDataPackageInfo,
            PKGDATA_PACKAGE_INFO_IMPLEMENTATION,
            CapabilityImplementationKind::Command,
        ),
        (
            CapabilityId::PkgDataListPackageFiles,
            PKGDATA_LIST_PACKAGE_FILES_IMPLEMENTATION,
            CapabilityImplementationKind::Command,
        ),
        (
            CapabilityId::PkgDataReadValue,
            PKGDATA_READ_VALUE_IMPLEMENTATION,
            CapabilityImplementationKind::Command,
        ),
    ];
    DaemonCompatibilitySnapshot {
        snapshot: CapabilitySnapshot {
            generation: 1,
            environment: YoctoEnvironmentIdentity {
                build_directory: AuthoritativeValue::detected(
                    build.to_owned(),
                    IdentityAuthority::InitializedEnvironment,
                ),
                available_tools: AuthoritativeValue::detected(
                    vec![ToolIdentity {
                        id: "oe-pkgdata-util".into(),
                        executable: tool.to_owned(),
                        version: None,
                    }],
                    IdentityAuthority::ExecutableProbe,
                ),
                ..YoctoEnvironmentIdentity::default()
            },
            capabilities: capabilities
                .iter()
                .map(|(id, _, _)| CapabilityRecord {
                    id: *id,
                    state: CapabilityState::Available,
                    evidence: vec![CapabilityEvidence {
                        kind: CapabilityEvidenceKind::DirectProbe,
                        outcome: CapabilityEvidenceOutcome::Positive,
                        subject: format!("{} fixture probe", id.as_str()),
                        detail: "The fixture exposes this exact package-data behavior.".into(),
                        argv: vec![tool.display().to_string(), "--help".into()],
                    }],
                })
                .collect(),
        },
        implementations: capabilities
            .into_iter()
            .map(|(id, implementation, kind)| {
                (
                    id,
                    CapabilityImplementation {
                        id: implementation.into(),
                        kind,
                    },
                )
            })
            .collect(),
    }
    .normalize()
    .unwrap()
}

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(name: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "yoctui-pkgdata-{name}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn fixture(name: &str, script: &str) -> (TestDirectory, PackageDataAdapter, PathBuf) {
    let directory = TestDirectory::new(name);
    let build_dir = directory.path().join("build");
    let pkgdata_dir = build_dir.join("tmp/pkgdata");
    fs::create_dir_all(&pkgdata_dir).unwrap();
    let tool = directory.path().join("oe-pkgdata-util");
    fs::write(&tool, script).unwrap();
    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(&tool).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&tool, permissions).unwrap();
    }
    let authority = compatibility(&build_dir, &tool);
    let adapter = PackageDataAdapter::with_paths(build_dir, tool, pkgdata_dir)
        .with_compatibility(authority, 1)
        .unwrap();
    let log = directory.path().join("arguments.log");
    (directory, adapter, log)
}

fn inventory_request() -> PackageInventoryRequest {
    PackageInventoryRequest { generation: 1 }
}

fn detail_request() -> PackageDetailRequest {
    PackageDetailRequest {
        identity: PackageIdentity::new("busybox"),
        generation: 2,
    }
}

mod compatibility_command_shared_fixture_builds_pkgdata_argv_without_spawning;

mod compatibility_pkgdata_builds_exact_authorized_commands_and_parses_results;

mod compatibility_pkgdata_uses_detected_tool_and_preserves_valid_empty_inventory;

mod pkgdata_adapter_parsers_bound_and_reject_untrusted_records;

mod compatibility_pkgdata_distinguishes_missing_generated_data_and_command_failure;

mod compatibility_pkgdata_rejects_missing_command_stale_snapshot_and_zero_spawn;

#[cfg(unix)]
mod pkgdata_adapter_rejects_symlinked_tool_and_pkgdata_paths;

mod pkgdata_adapter_times_out_and_cancels_process_groups;
