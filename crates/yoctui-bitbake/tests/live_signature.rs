use std::{collections::BTreeMap, env, path::PathBuf};

use yoctui_bitbake::SignatureAdapter;
use yoctui_model::{
    AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
    CapabilityId, CapabilityImplementation, CapabilityImplementationKind, CapabilityRecord,
    CapabilitySnapshot, CapabilityState, DaemonCompatibilitySnapshot, IdentityAuthority,
    SignatureComparisonRequest, SignatureTarget, YoctoEnvironmentIdentity,
};

fn required_path(name: &str) -> PathBuf {
    env::var_os(name)
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("{name} must name an absolute live-test path"))
}

fn signature_authority(
    build_dir: PathBuf,
    dumpsig: &std::path::Path,
    diffsigs: &std::path::Path,
) -> DaemonCompatibilitySnapshot {
    let capabilities = [
        (
            CapabilityId::BitBakeDumpSig,
            "bitbake_dumpsig.argv",
            dumpsig,
        ),
        (
            CapabilityId::BitBakeDiffSigs,
            "bitbake_diffsigs.argv",
            diffsigs,
        ),
    ];
    for (_, _, executable) in &capabilities {
        let output = std::process::Command::new(executable)
            .arg("--help")
            .output()
            .unwrap_or_else(|error| {
                panic!(
                    "failed to probe live signature tool {}: {error}",
                    executable.display()
                )
            });
        assert!(
            output.status.success(),
            "live signature tool {} rejected --help",
            executable.display()
        );
    }
    DaemonCompatibilitySnapshot {
        snapshot: CapabilitySnapshot {
            generation: 1,
            environment: YoctoEnvironmentIdentity {
                build_directory: AuthoritativeValue::detected(
                    build_dir,
                    IdentityAuthority::InitializedEnvironment,
                ),
                ..YoctoEnvironmentIdentity::default()
            },
            capabilities: capabilities
                .iter()
                .map(|(id, _, executable)| CapabilityRecord {
                    id: *id,
                    state: CapabilityState::Available,
                    evidence: vec![CapabilityEvidence {
                        kind: CapabilityEvidenceKind::DirectProbe,
                        outcome: CapabilityEvidenceOutcome::Positive,
                        subject: executable.display().to_string(),
                        detail: "The live test positively probed the exact initialized-environment executable before authorizing its command form.".into(),
                        argv: vec![executable.display().to_string(), "--help".into()],
                    }],
                })
                .collect(),
        },
        implementations: capabilities
            .iter()
            .map(|(id, implementation, _)| {
                (
                    *id,
                    CapabilityImplementation {
                        id: (*implementation).into(),
                        kind: CapabilityImplementationKind::Command,
                    },
                )
            })
            .collect::<BTreeMap<_, _>>(),
    }
    .normalize()
    .unwrap()
}

#[tokio::test]
#[ignore = "requires an initialized writable Yocto build and real BitBake signature tools"]
async fn signature_adapter_live_smoke() {
    let build_dir = std::fs::canonicalize(required_path("YOCTUI_LIVE_BUILD_DIR"))
        .expect("YOCTUI_LIVE_BUILD_DIR must identify an existing build directory");
    let dumpsig = std::fs::canonicalize(required_path("YOCTUI_LIVE_DUMPSIG"))
        .expect("YOCTUI_LIVE_DUMPSIG must identify an existing executable");
    let diffsigs = std::fs::canonicalize(required_path("YOCTUI_LIVE_DIFFSIGS"))
        .expect("YOCTUI_LIVE_DIFFSIGS must identify an existing executable");
    let left_target = SignatureTarget {
        recipe: env::var("YOCTUI_LIVE_SIGNATURE_RECIPE")
            .unwrap_or_else(|_| "autoconf-native".into()),
        task: env::var("YOCTUI_LIVE_SIGNATURE_TASK").unwrap_or_else(|_| "do_fetch".into()),
    };
    let right_target = SignatureTarget {
        recipe: env::var("YOCTUI_LIVE_SIGNATURE_COMPARE_RECIPE")
            .unwrap_or_else(|_| left_target.recipe.clone()),
        task: env::var("YOCTUI_LIVE_SIGNATURE_COMPARE_TASK")
            .unwrap_or_else(|_| "do_recipe_qa".into()),
    };
    let authority = signature_authority(build_dir.clone(), &dumpsig, &diffsigs);
    let adapter = SignatureAdapter::with_programs(build_dir, dumpsig, diffsigs)
        .with_compatibility(authority)
        .unwrap();
    let left = adapter.dump(left_target).await.unwrap();
    let right = adapter.dump(right_target).await.unwrap();
    assert!(
        !left.records.is_empty(),
        "the requested live signature dump returned no records"
    );
    assert!(
        !right.records.is_empty(),
        "the requested live comparison signature dump returned no records"
    );
    let request = SignatureComparisonRequest {
        left: left.records.last().unwrap().identity.clone(),
        right: right.records.last().unwrap().identity.clone(),
    };
    let comparison = adapter.compare(request).await.unwrap();
    eprintln!(
        "live signatures: left_records={} right_records={} differences={} dump_limitations={} comparison_limitations={}",
        left.records.len(),
        right.records.len(),
        comparison.differences.len(),
        left.limitations.len() + right.limitations.len(),
        comparison.limitations.len()
    );
}
