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
    CapabilityId, DaemonCompatibilitySnapshot, MAX_PACKAGE_LIMITATIONS, MAX_PACKAGE_RECORDS,
    PackageDetail, PackageDetailRequest, PackageField, PackageIdentity, PackageInventoryRequest,
    PackageSummary, normalize_package_detail, normalize_package_summaries,
};

const MAX_PACKAGE_OUTPUT_BYTES: usize = 8 * 1024 * 1024;
const MAX_PACKAGE_OUTPUT_LINES: usize = 32_768;
const PACKAGE_COMMAND_TIMEOUT: Duration = Duration::from_secs(120);
const PACKAGE_ARGUMENT_BATCH: usize = 128;
pub const PKGDATA_LIST_PACKAGES_IMPLEMENTATION: &str = "pkgdata.list_packages.argv";
pub const PKGDATA_PACKAGE_INFO_IMPLEMENTATION: &str = "pkgdata.package_info.argv";
pub const PKGDATA_LIST_PACKAGE_FILES_IMPLEMENTATION: &str = "pkgdata.list_package_files.argv";
pub const PKGDATA_READ_VALUE_IMPLEMENTATION: &str = "pkgdata.read_value.argv";

include!("package/types_and_cancellation.rs");
include!("package/adapter.rs");
include!("package/response_parsing.rs");
include!("package/process_io.rs");

#[cfg(test)]
mod tests {
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
        CapabilityImplementation, CapabilityImplementationKind, CapabilityRecord,
        CapabilitySnapshot, CapabilityState, IdentityAuthority, ToolIdentity,
        YoctoEnvironmentIdentity,
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

    #[test]
    fn compatibility_command_shared_fixture_builds_pkgdata_argv_without_spawning() {
        let mut authority = release_capability_fixtures()
            .into_iter()
            .find(|fixture| fixture.role == CompatibilityFixtureRole::LatestSupportCandidate)
            .unwrap()
            .command_authority(39);
        let build_dir = authority
            .snapshot
            .environment
            .build_directory
            .value()
            .unwrap()
            .clone();
        let tool = authority
            .snapshot
            .environment
            .available_tools
            .value()
            .unwrap()
            .iter()
            .find(|tool| tool.id == "oe-pkgdata-util")
            .unwrap()
            .executable
            .clone();
        let pkgdata_dir = build_dir.join("tmp/pkgdata");
        let context = PackageDataContext {
            build_dir,
            pkgdata_dir: pkgdata_dir.clone(),
            tool: tool.clone(),
            compatibility: authority.clone(),
        };

        let list = context
            .command(
                CapabilityId::PkgDataListPackages,
                PKGDATA_LIST_PACKAGES_IMPLEMENTATION,
                "list-pkgs",
                [OsString::from("-r")],
            )
            .unwrap();
        assert_eq!(list.executable(), tool);
        assert_eq!(
            list.arguments(),
            [
                OsString::from("-p"),
                pkgdata_dir.as_os_str().to_owned(),
                OsString::from("list-pkgs"),
                OsString::from("-r"),
            ]
        );

        let read = context
            .command(
                CapabilityId::PkgDataReadValue,
                PKGDATA_READ_VALUE_IMPLEMENTATION,
                "read-value",
                [OsString::from("RDEPENDS"), OsString::from("-n")],
            )
            .unwrap();
        assert_eq!(
            read.arguments(),
            [
                OsString::from("-p"),
                pkgdata_dir.as_os_str().to_owned(),
                OsString::from("read-value"),
                OsString::from("RDEPENDS"),
                OsString::from("-n"),
            ]
        );

        let record = authority
            .snapshot
            .capabilities
            .iter_mut()
            .find(|record| record.id == CapabilityId::PkgDataReadValue)
            .unwrap();
        record.state = CapabilityState::Unavailable {
            reason: yoctui_model::CapabilityReason::new(
                "fixture.command_absent",
                "The fixture does not expose read-value.",
                Some("Required command: read-value".into()),
            )
            .unwrap(),
        };
        record.evidence[0].outcome = CapabilityEvidenceOutcome::Negative;
        authority
            .implementations
            .remove(&CapabilityId::PkgDataReadValue);
        let unavailable = PackageDataContext {
            build_dir: context.build_dir,
            pkgdata_dir: context.pkgdata_dir,
            tool: context.tool,
            compatibility: authority.normalize().unwrap(),
        };
        assert!(matches!(
            unavailable.command(
                CapabilityId::PkgDataReadValue,
                PKGDATA_READ_VALUE_IMPLEMENTATION,
                "read-value",
                [OsString::from("RDEPENDS")],
            ),
            Err(PackageDataAdapterError::CapabilityUnavailable {
                capability: CapabilityId::PkgDataReadValue,
                ..
            })
        ));
    }

    #[tokio::test]
    async fn compatibility_pkgdata_builds_exact_authorized_commands_and_parses_results() {
        let script = r#"#!/bin/sh
log="$(dirname "$0")/arguments.log"
printf '%s\n' "--" "$@" >> "$log"
case "$3" in
  list-pkgs)
    printf 'libc6\nbusybox\ninit\n'
    ;;
  package-info)
    printf 'busybox 1.37.0-r0 busybox 1.37.0-r0 1024 "GPL-2.0-only"\n'
    printf 'init 1.0-r0 init 1.0-r0 64 "MIT"\n'
    printf 'libc6 2.40-r0 glibc 2.40-r0 4096 "GPL-2.0-or-later"\n'
    ;;
  list-pkg-files)
    printf 'busybox:\n\t/bin/busybox\n\t/etc/busybox.conf\n'
    ;;
  read-value)
    printf 'busybox libc6 (>= 2.40)\n'
    printf 'init busybox\n'
    printf 'libc6\n'
    ;;
  *)
    printf 'unexpected subcommand\n' >&2
    exit 9
    ;;
esac
"#;
        let (_directory, adapter, log) = fixture("typed", script);
        let inventory = adapter
            .clone()
            .with_argument_batch(8)
            .inventory(inventory_request())
            .await
            .unwrap();
        assert_eq!(
            inventory
                .packages
                .iter()
                .map(|package| package.identity.name.as_str())
                .collect::<Vec<_>>(),
            vec!["busybox", "init", "libc6"]
        );
        let busybox = &inventory.packages[0];
        assert_eq!(busybox.recipe, PackageField::Available("busybox".into()));
        assert_eq!(busybox.version, PackageField::Available("1.37.0-r0".into()));
        assert_eq!(busybox.installed_size_bytes, PackageField::Available(1_024));
        assert_eq!(
            busybox.license,
            PackageField::Available("GPL-2.0-only".into())
        );
        assert_eq!(busybox.provider, PackageField::Unavailable);
        assert!(
            inventory
                .limitations
                .iter()
                .any(|limitation| limitation.contains("provider recipe paths"))
        );

        let detail = adapter
            .with_argument_batch(2)
            .detail(detail_request())
            .await
            .unwrap();
        assert_eq!(
            detail.detail.files,
            PackageField::Available(vec![
                PathBuf::from("/bin/busybox"),
                PathBuf::from("/etc/busybox.conf"),
            ])
        );
        assert_eq!(
            detail.detail.runtime_dependencies,
            PackageField::Available(vec![PackageIdentity::new("libc6")])
        );
        assert_eq!(
            detail.detail.reverse_dependencies,
            PackageField::Available(vec![PackageIdentity::new("init")])
        );

        let arguments = fs::read_to_string(log).unwrap();
        assert!(arguments.contains("\n-p\n"));
        assert!(arguments.contains("\nlist-pkgs\n-r\n"));
        assert!(arguments.contains("\npackage-info\n-e\nLICENSE\nbusybox\n"));
        assert!(arguments.contains("\nlist-pkg-files\n-r\nbusybox\n"));
        assert!(arguments.contains("\nread-value\nRDEPENDS\n-n\n"));
    }

    #[tokio::test]
    async fn compatibility_pkgdata_uses_detected_tool_and_preserves_valid_empty_inventory() {
        let directory = TestDirectory::new("discover");
        let build_dir = directory.path().join("build");
        fs::create_dir_all(build_dir.join("tmp/pkgdata")).unwrap();
        let scripts = directory.path().join("layers/openembedded-core/scripts");
        fs::create_dir_all(&scripts).unwrap();
        let tool = scripts.join("oe-pkgdata-util");
        fs::write(
            &tool,
            "#!/bin/sh\nprintf 'No packages found\\n' >&2\nexit 1\n",
        )
        .unwrap();
        #[cfg(unix)]
        {
            let mut permissions = fs::metadata(&tool).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&tool, permissions).unwrap();
        }
        let pkgdata_dir = build_dir.join("tmp/pkgdata");
        let authority = compatibility(&build_dir, &tool);
        let response = PackageDataAdapter::with_paths(build_dir, tool, pkgdata_dir)
            .with_compatibility(authority, 1)
            .unwrap()
            .inventory(inventory_request())
            .await
            .unwrap();
        assert!(response.packages.is_empty());
        assert!(response.limitations.is_empty());
    }

    #[test]
    fn pkgdata_adapter_parsers_bound_and_reject_untrusted_records() {
        let mut limitations = Vec::new();
        let mut input = String::new();
        for index in 0..=MAX_PACKAGE_RECORDS {
            input.push_str(&format!("package-{index}\n"));
        }
        input.push_str("bad package\n");
        let identities = parse_package_list(input.as_bytes(), &mut limitations).unwrap();
        assert_eq!(identities.len(), MAX_PACKAGE_RECORDS);
        assert!(
            limitations
                .iter()
                .any(|limitation| limitation.contains("limited"))
        );

        let mut summaries = BTreeMap::from([unavailable_summary(PackageIdentity::new("busybox"))]);
        parse_package_info(
            b"other 1.0 other 1.0 5 \"MIT\"\nmalformed\nbusybox 1.0 busybox 1.0 nope \"MIT\"\n",
            &mut summaries,
            &mut limitations,
        )
        .unwrap();
        assert_eq!(
            summaries[&PackageIdentity::new("busybox")].installed_size_bytes,
            PackageField::Unavailable
        );
        assert!(
            limitations
                .iter()
                .any(|limitation| limitation.contains("unexpected package-info identity"))
        );

        assert!(matches!(
            parse_package_files(
                &PackageIdentity::new("busybox"),
                b"wrong:\n\t/bin/value\n",
                &mut limitations
            ),
            Err(PackageDataAdapterError::Malformed(_))
        ));
    }

    #[tokio::test]
    async fn compatibility_pkgdata_distinguishes_missing_generated_data_and_command_failure() {
        let directory = TestDirectory::new("failures");
        let build_dir = directory.path().join("build");
        fs::create_dir_all(&build_dir).unwrap();
        let tool = directory.path().join("oe-pkgdata-util");
        fs::write(&tool, "#!/bin/sh\nexit 0\n").unwrap();
        #[cfg(unix)]
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o755)).unwrap();
        let authority = compatibility(&build_dir, &tool);
        let missing =
            PackageDataAdapter::with_paths(build_dir.clone(), tool, build_dir.join("tmp/pkgdata"))
                .with_compatibility(authority, 1)
                .unwrap()
                .inventory(inventory_request())
                .await
                .unwrap_err();
        assert!(matches!(
            missing,
            PackageDataAdapterError::MissingPkgdata(_)
        ));

        let script = "#!/bin/sh\nprintf 'broken metadata\\n' >&2\nexit 17\n";
        let (_fixture, adapter, _log) = fixture("nonzero", script);
        assert_eq!(
            adapter.inventory(inventory_request()).await.unwrap_err(),
            PackageDataAdapterError::NonZero {
                exit_code: Some(17),
                message: "broken metadata".into(),
            }
        );
        assert!(matches!(
            adapter
                .inventory(PackageInventoryRequest { generation: 0 })
                .await,
            Err(PackageDataAdapterError::InvalidRequest(_))
        ));
        assert!(matches!(
            adapter
                .detail(PackageDetailRequest {
                    identity: PackageIdentity::new("bad package"),
                    generation: 1,
                })
                .await,
            Err(PackageDataAdapterError::InvalidRequest(_))
        ));
    }

    #[tokio::test]
    async fn compatibility_pkgdata_rejects_missing_command_stale_snapshot_and_zero_spawn() {
        let directory = TestDirectory::new("capability-reject");
        let build_dir = directory.path().join("build");
        let pkgdata_dir = build_dir.join("tmp/pkgdata");
        fs::create_dir_all(&pkgdata_dir).unwrap();
        let tool = directory.path().join("oe-pkgdata-util");
        let marker = directory.path().join("spawned");
        fs::write(&tool, format!("#!/bin/sh\ntouch '{}'\n", marker.display())).unwrap();
        #[cfg(unix)]
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o755)).unwrap();
        let mut authority = compatibility(&build_dir, &tool);
        let record = authority
            .snapshot
            .capabilities
            .iter_mut()
            .find(|record| record.id == CapabilityId::PkgDataListPackages)
            .unwrap();
        record.state = CapabilityState::Unavailable {
            reason: yoctui_model::CapabilityReason::new(
                "pkgdata.command_missing",
                "Current oe-pkgdata-util does not expose list-pkgs.",
                Some("Required command: list-pkgs".into()),
            )
            .unwrap(),
        };
        record.evidence[0].outcome = CapabilityEvidenceOutcome::Negative;
        authority
            .implementations
            .remove(&CapabilityId::PkgDataListPackages);
        let authority = authority.normalize().unwrap();
        assert!(matches!(
            PackageDataAdapter::with_paths(build_dir.clone(), tool.clone(), pkgdata_dir.clone())
                .with_compatibility(authority.clone(), 2),
            Err(PackageDataAdapterError::StaleCapability { .. })
        ));
        let error = PackageDataAdapter::with_paths(build_dir, tool, pkgdata_dir)
            .with_compatibility(authority, 1)
            .unwrap()
            .inventory(inventory_request())
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            PackageDataAdapterError::CapabilityUnavailable {
                capability: CapabilityId::PkgDataListPackages,
                reason,
            } if reason.contains("does not expose list-pkgs")
        ));
        assert!(!marker.exists());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn pkgdata_adapter_rejects_symlinked_tool_and_pkgdata_paths() {
        let directory = TestDirectory::new("symlinks");
        let build_dir = directory.path().join("build");
        let real_pkgdata = directory.path().join("real-pkgdata");
        fs::create_dir_all(&build_dir).unwrap();
        fs::create_dir_all(&real_pkgdata).unwrap();
        let linked_pkgdata = build_dir.join("pkgdata");
        symlink(&real_pkgdata, &linked_pkgdata).unwrap();
        let tool = directory.path().join("tool");
        fs::write(&tool, "#!/bin/sh\nexit 0\n").unwrap();
        let mut permissions = fs::metadata(&tool).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&tool, permissions).unwrap();
        let error = PackageDataAdapter::with_paths(build_dir.clone(), tool.clone(), linked_pkgdata)
            .with_compatibility(compatibility(&build_dir, &tool), 1)
            .unwrap()
            .inventory(inventory_request())
            .await
            .unwrap_err();
        assert!(matches!(error, PackageDataAdapterError::MissingPkgdata(_)));

        let linked_tool = directory.path().join("linked-tool");
        symlink(&tool, &linked_tool).unwrap();
        let authority = compatibility(&build_dir, &linked_tool);
        let error = PackageDataAdapter::with_paths(build_dir, linked_tool, real_pkgdata)
            .with_compatibility(authority, 1)
            .unwrap()
            .inventory(inventory_request())
            .await
            .unwrap_err();
        assert!(matches!(error, PackageDataAdapterError::InvalidPath(_)));
    }

    #[tokio::test]
    async fn pkgdata_adapter_times_out_and_cancels_process_groups() {
        let script = "#!/bin/sh\nsleep 5\nprintf 'busybox\\n'\n";
        let (_timeout_directory, timeout_adapter, _log) = fixture("timeout", script);
        let error = timeout_adapter
            .with_timeout(Duration::from_millis(20))
            .inventory(inventory_request())
            .await
            .unwrap_err();
        assert!(matches!(error, PackageDataAdapterError::Timeout(_)));

        let (_cancel_directory, cancel_adapter, _log) = fixture("cancel", script);
        let cancellation = PackageDataCancellation::default();
        let task_cancellation = cancellation.clone();
        let task = tokio::spawn(async move {
            cancel_adapter
                .inventory_with_cancellation(inventory_request(), task_cancellation)
                .await
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(cancellation.cancel());
        assert_eq!(
            task.await.unwrap().unwrap_err(),
            PackageDataAdapterError::Cancelled
        );
    }
}
