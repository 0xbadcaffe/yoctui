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
mod tests {
    use super::*;
    use std::{
        fs,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    #[cfg(unix)]
    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(name: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "yoctui-signature-{name}-{}-{nonce}",
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

    fn target() -> SignatureTarget {
        SignatureTarget {
            recipe: "busybox".into(),
            task: "do_compile".into(),
        }
    }

    fn fixture(hash: &str) -> String {
        format!(
            "basehash_ignore_vars: []\n\
             taskhash_ignore_tasks: []\n\
             Task dependencies: ['CC']\n\
             basehash: base-{hash}\n\
             List of dependencies for variable CC is []\n\
             Variable CC value is gcc\n\
             Variable SCRIPT value is line one\n\
             line two\n\
             Tasks this task depends on: ['busybox:do_configure']\n\
             Hash for dependent task busybox:do_configure is dep-{hash}\n\
             Computed base hash is base-{hash} and from file base-{hash}\n\
             Computed task hash is {hash}\n"
        )
    }

    fn write_executable(path: &Path, body: &str) {
        crate::test_support::write_executable(path, body);
    }

    fn signature_path(root: &Path, hash: &str) -> PathBuf {
        let directory = root.join("tmp/stamps/qemux86_64/busybox");
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join(format!("1.0.do_compile.sigdata.{hash}"));
        fs::write(&path, "{}").unwrap();
        path
    }

    fn test_compatibility(root: &Path, dump: &Path, diff: &Path) -> DaemonCompatibilitySnapshot {
        use yoctui_model::{
            AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind,
            CapabilityEvidenceOutcome, CapabilityId, CapabilityImplementation,
            CapabilityImplementationKind, CapabilityRecord, CapabilitySnapshot, CapabilityState,
            IdentityAuthority, ToolIdentity, YoctoEnvironmentIdentity,
        };
        let capabilities = [
            (
                CapabilityId::BitBakeDumpSig,
                crate::compatibility_command::BITBAKE_DUMPSIG_ARGV_IMPLEMENTATION,
            ),
            (
                CapabilityId::BitBakeDiffSigs,
                crate::compatibility_command::BITBAKE_DIFFSIGS_ARGV_IMPLEMENTATION,
            ),
        ];
        DaemonCompatibilitySnapshot {
            snapshot: CapabilitySnapshot {
                generation: 1,
                environment: YoctoEnvironmentIdentity {
                    build_directory: AuthoritativeValue::detected(
                        root.to_owned(),
                        IdentityAuthority::InitializedEnvironment,
                    ),
                    available_tools: AuthoritativeValue::detected(
                        vec![
                            ToolIdentity {
                                id: "bitbake-dumpsig".into(),
                                executable: dump.to_owned(),
                                version: None,
                            },
                            ToolIdentity {
                                id: "bitbake-diffsigs".into(),
                                executable: diff.to_owned(),
                                version: None,
                            },
                        ],
                        IdentityAuthority::ExecutableProbe,
                    ),
                    ..YoctoEnvironmentIdentity::default()
                },
                capabilities: capabilities
                    .iter()
                    .map(|(id, _)| CapabilityRecord {
                        id: *id,
                        state: CapabilityState::Available,
                        evidence: vec![CapabilityEvidence {
                            kind: CapabilityEvidenceKind::DirectProbe,
                            outcome: CapabilityEvidenceOutcome::Positive,
                            subject: format!("{} fixture probe", id.as_str()),
                            detail: "The exact signature helper argv is supported.".into(),
                            argv: Vec::new(),
                        }],
                    })
                    .collect(),
            },
            implementations: capabilities
                .into_iter()
                .map(|(id, implementation)| {
                    (
                        id,
                        CapabilityImplementation {
                            id: implementation.into(),
                            kind: CapabilityImplementationKind::Command,
                        },
                    )
                })
                .collect(),
        }
        .normalize()
        .unwrap()
    }

    fn test_adapter(root: &Path, dump: PathBuf, diff: PathBuf) -> SignatureAdapter {
        let compatibility = test_compatibility(root, &dump, &diff);
        SignatureAdapter::with_programs(root.to_owned(), dump, diff)
            .with_compatibility(compatibility)
            .unwrap()
    }

    #[test]
    fn signature_adapter_parser_keeps_typed_bounded_dump_data() {
        let identity = SignatureIdentity {
            target: target(),
            hash: Some("aaa".into()),
            path: Some("/build/tmp/stamps/qemux86_64/busybox/1.0.do_compile.sigdata.aaa".into()),
        };
        let (record, limitations) = parse_signature_dump(&identity, &fixture("aaa")).unwrap();
        assert!(limitations.is_empty());
        assert_eq!(record.base_hash.as_deref(), Some("base-aaa"));
        assert_eq!(record.task_hash.as_deref(), Some("aaa"));
        assert_eq!(record.variables.len(), 2);
        assert_eq!(
            record.variables[1].value.as_deref(),
            Some("line one\nline two")
        );
        assert_eq!(record.dependencies, vec!["busybox:do_configure=dep-aaa"]);

        assert!(matches!(
            parse_signature_dump(&identity, "nonsense"),
            Err(SignatureAdapterError::Malformed(_))
        ));
    }

    #[test]
    fn signature_adapter_parses_typed_diffsigs_summary_honestly() {
        let (differences, limitations) = parse_diffsigs_output(
            "basehash changed from old to new\n\
             Variable CC value changed from 'gcc' to 'clang'\n\
             Dependency on variable CFLAGS was added\n\
             recursive detail",
        );
        assert_eq!(differences.len(), 3);
        assert_eq!(limitations.len(), 1);
    }

    #[tokio::test]
    async fn signature_adapter_discovers_dumps_and_constructs_exact_arguments() {
        let directory = TestDirectory::new("dump");
        let path = signature_path(directory.path(), "aaa");
        let dump = directory.path().join("dump");
        let diff = directory.path().join("diff");
        write_executable(
            &dump,
            &format!(
                "#!/bin/sh\n[ \"$#\" -eq 1 ] || exit 8\n[ \"$1\" = \"{}\" ] || exit 9\nprintf '%s' '{}'\n",
                path.display(),
                fixture("aaa").replace('\'', "'\\''")
            ),
        );
        write_executable(&diff, "#!/bin/sh\nexit 0\n");
        let response = test_adapter(directory.path(), dump, diff)
            .dump(target())
            .await
            .unwrap();
        assert_eq!(response.records.len(), 1);
        assert_eq!(response.records[0].identity.path.as_ref(), Some(&path));
        assert!(response.limitations.is_empty());
    }

    #[tokio::test]
    async fn signature_adapter_compares_exact_validated_paths() {
        let directory = TestDirectory::new("compare");
        let left_path = signature_path(directory.path(), "aaa");
        let right_path = signature_path(directory.path(), "bbb");
        let dump = directory.path().join("dump");
        let diff = directory.path().join("diff");
        write_executable(
            &dump,
            &format!(
                "#!/bin/sh\ncase \"$1\" in\n*aaa) printf '%s' '{}';;\n*bbb) printf '%s' '{}';;\n*) exit 9;;\nesac\n",
                fixture("aaa").replace('\'', "'\\''"),
                fixture("bbb")
                    .replace("Variable CC value is gcc", "Variable CC value is clang")
                    .replace('\'', "'\\''")
            ),
        );
        write_executable(
            &diff,
            &format!(
                "#!/bin/sh\n[ \"$1\" = '-c' ] || exit 8\n[ \"$2\" = 'never' ] || exit 9\n[ \"$3\" = '{}' ] || exit 10\n[ \"$4\" = '{}' ] || exit 11\nprintf '%s\\n' \"basehash changed from base-aaa to base-bbb\"\n",
                left_path.display(),
                right_path.display()
            ),
        );
        let request = SignatureComparisonRequest {
            left: identity_from_path(&target(), left_path).unwrap(),
            right: identity_from_path(&target(), right_path).unwrap(),
        };
        let response = test_adapter(directory.path(), dump, diff)
            .compare(request.clone())
            .await
            .unwrap();
        assert_eq!(response.request, request);
        assert!(response.limitations.is_empty());
        assert!(response.differences.iter().any(|difference| {
            difference.category == SignatureDifferenceCategory::ChangedValue
                && difference.key == "CC"
        }));
    }

    #[tokio::test]
    async fn signature_adapter_rejects_escape_missing_tools_and_nonzero_results() {
        let directory = TestDirectory::new("errors");
        let outside_directory = TestDirectory::new("outside");
        let outside = outside_directory.path().join("outside.sigdata.aaa");
        fs::write(&outside, "{}").unwrap();
        let identity = SignatureIdentity {
            target: target(),
            hash: Some("aaa".into()),
            path: Some(outside.clone()),
        };
        let request = SignatureComparisonRequest {
            left: identity,
            right: SignatureIdentity {
                target: target(),
                hash: Some("bbb".into()),
                path: Some(outside),
            },
        };
        let adapter = test_adapter(
            directory.path(),
            directory.path().join("missing"),
            directory.path().join("missing"),
        );
        assert!(matches!(
            adapter.compare(request).await,
            Err(SignatureAdapterError::PathEscape(_))
        ));

        let path = signature_path(directory.path(), "aaa");
        assert!(matches!(
            adapter.dump(target()).await,
            Err(SignatureAdapterError::MissingTool(_))
        ));
        let failure = directory.path().join("failure");
        write_executable(&failure, "#!/bin/sh\nprintf 'bad input\\n' >&2\nexit 7\n");
        let error = test_adapter(directory.path(), failure, directory.path().join("unused"))
            .dump(target())
            .await
            .unwrap_err();
        assert_eq!(
            error,
            SignatureAdapterError::NonZero {
                exit_code: Some(7),
                message: "bad input".into()
            }
        );
        assert!(path.exists());
    }

    #[tokio::test]
    async fn signature_adapter_bounds_output_and_supports_cancellation() {
        let directory = TestDirectory::new("bounds");
        signature_path(directory.path(), "aaa");
        let oversized = directory.path().join("oversized");
        write_executable(
            &oversized,
            "#!/bin/sh\nhead -c 9000000 /dev/zero | tr '\\0' x\n",
        );
        let error = test_adapter(directory.path(), oversized, directory.path().join("unused"))
            .dump(target())
            .await
            .unwrap_err();
        assert_eq!(
            error,
            SignatureAdapterError::OutputLimit(MAX_SIGNATURE_OUTPUT_BYTES)
        );

        let sleeping = directory.path().join("sleeping");
        write_executable(&sleeping, "#!/bin/sh\nsleep 30\n");
        let adapter = test_adapter(directory.path(), sleeping, directory.path().join("unused"))
            .with_timeout(Duration::from_secs(60));
        let cancellation = SignatureCancellation::default();
        let cancel_handle = cancellation.clone();
        let operation =
            tokio::spawn(
                async move { adapter.dump_with_cancellation(target(), cancellation).await },
            );
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(cancel_handle.cancel());
        assert!(!cancel_handle.cancel());
        assert_eq!(
            operation.await.unwrap().unwrap_err(),
            SignatureAdapterError::Cancelled
        );
    }

    #[tokio::test]
    async fn signature_adapter_reports_empty_malformed_and_duplicate_results() {
        let directory = TestDirectory::new("malformed");
        let dump = directory.path().join("dump");
        let diff = directory.path().join("diff");
        write_executable(&dump, "#!/bin/sh\nprintf 'nonsense\\n'\n");
        write_executable(&diff, "#!/bin/sh\nexit 0\n");
        let adapter = test_adapter(directory.path(), dump.clone(), diff);
        assert!(adapter.dump(target()).await.unwrap().records.is_empty());

        signature_path(directory.path(), "aaa");
        assert!(matches!(
            adapter.dump(target()).await,
            Err(SignatureAdapterError::Malformed(_))
        ));

        let identity = SignatureIdentity {
            target: target(),
            hash: Some("aaa".into()),
            path: Some(
                directory
                    .path()
                    .join("tmp/stamps/qemux86_64/busybox/1.0.do_compile.sigdata.aaa"),
            ),
        };
        let (left, _) = parse_signature_dump(&identity, &fixture("aaa")).unwrap();
        let (records, report) = normalize_signature_records(&target(), vec![left.clone(), left], 8);
        assert_eq!(records.len(), 1);
        assert_eq!(report.duplicate_records, 1);
    }
}
