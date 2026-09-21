use super::*;

#[tokio::test]
async fn compatibility_command_unavailable_action_is_rejected_before_process_spawn() {
    use crate::{BitBakeBackend, ProcessBackend};
    use std::{fs, os::unix::fs::PermissionsExt};

    let root = std::env::temp_dir().join(format!(
        "yoctui-compatibility-command-no-spawn-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let executable = root.join("bitbake");
    let marker = root.join("spawned");
    fs::write(
        &executable,
        format!("#!/bin/sh\ntouch '{}'\n", marker.display()),
    )
    .unwrap();
    let mut permissions = fs::metadata(&executable).unwrap().permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&executable, permissions).unwrap();

    let mut unavailable = authority(
        1,
        &[(
            CapabilityId::BitBakeBuild,
            BITBAKE_BUILD_ARGV_IMPLEMENTATION,
        )],
    );
    unavailable.snapshot.environment.build_directory =
        AuthoritativeValue::detected(root.clone(), IdentityAuthority::InitializedEnvironment);
    unavailable.snapshot.capabilities[0] = CapabilityRecord {
        id: CapabilityId::BitBakeBuild,
        state: CapabilityState::Unavailable {
            reason: CapabilityReason::new(
                "command.unavailable",
                "The connected BitBake command was not positively verified.",
                None,
            )
            .unwrap(),
        },
        evidence: vec![CapabilityEvidence {
            kind: CapabilityEvidenceKind::DirectProbe,
            outcome: CapabilityEvidenceOutcome::Negative,
            subject: "bitbake executable".into(),
            detail: "The required command behavior is absent.".into(),
            argv: Vec::new(),
        }],
    };
    unavailable.implementations.clear();
    let mut backend = ProcessBackend::with_executable(root.clone(), executable)
        .with_compatibility(unavailable.normalize().unwrap())
        .unwrap();
    let error = backend
        .start_build(BuildRequest {
            targets: vec!["busybox".into()],
            task: None,
            force: false,
        })
        .await
        .unwrap_err();
    assert!(error.to_string().contains("not positively verified"));
    assert!(!marker.exists());
    fs::remove_dir_all(root).unwrap();
}
