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
        AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
        CapabilityId, CapabilityImplementation, CapabilityImplementationKind, CapabilityRecord,
        CapabilitySnapshot, CapabilityState, IdentityAuthority, ToolIdentity,
        YoctoEnvironmentIdentity,
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

mod signature_adapter_parser_keeps_typed_bounded_dump_data;

mod signature_adapter_parses_typed_diffsigs_summary_honestly;

mod signature_adapter_discovers_dumps_and_constructs_exact_arguments;

mod signature_adapter_compares_exact_validated_paths;

mod signature_adapter_rejects_escape_missing_tools_and_nonzero_results;

mod signature_adapter_bounds_output_and_supports_cancellation;

mod signature_adapter_reports_empty_malformed_and_duplicate_results;
