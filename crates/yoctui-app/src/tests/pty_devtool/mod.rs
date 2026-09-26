use super::*;
use crate::{PtyContextEntry, VerifiedPtyEnvironment};
use std::{collections::BTreeMap, io::Write as _};
use yoctui_model::{
    AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
    CapabilityId, CapabilityImplementation, CapabilityImplementationKind, CapabilityRecord,
    CapabilitySnapshot, CapabilityState, DevtoolGitState, DevtoolStatusError, IdentityAuthority,
    ToolIdentity, YoctoEnvironmentIdentity,
};

fn compatibility(
    build: &std::path::Path,
    executable: &std::path::Path,
) -> DaemonCompatibilitySnapshot {
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
                        id: "devtool".into(),
                        executable: executable.to_owned(),
                        version: None,
                    }],
                    IdentityAuthority::ExecutableProbe,
                ),
                ..YoctoEnvironmentIdentity::default()
            },
            capabilities: vec![CapabilityRecord {
                id: CapabilityId::DevtoolEditRecipe,
                state: CapabilityState::Available,
                evidence: vec![CapabilityEvidence {
                    kind: CapabilityEvidenceKind::DirectProbe,
                    outcome: CapabilityEvidenceOutcome::Positive,
                    subject: "devtool edit-recipe --help".into(),
                    detail: "Fixture exposes edit-recipe.".into(),
                    argv: vec![
                        executable.display().to_string(),
                        "edit-recipe".into(),
                        "--help".into(),
                    ],
                }],
            }],
        },
        implementations: BTreeMap::from([(
            CapabilityId::DevtoolEditRecipe,
            CapabilityImplementation {
                id: yoctui_bitbake::DEVTOOL_EDIT_RECIPE_IMPLEMENTATION.into(),
                kind: CapabilityImplementationKind::Command,
            },
        )]),
    }
    .normalize()
    .unwrap()
}

fn fixture() -> (PathBuf, PtyDevtoolRouter, DevtoolStatus) {
    let nonce = std::time::SystemTime::UNIX_EPOCH
        .elapsed()
        .unwrap()
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("yoctui-pty-devtool-{}-{nonce}", std::process::id()));
    for path in ["source", "build", "workspace/busybox"] {
        fs::create_dir_all(root.join(path)).unwrap();
    }
    let executable = root.join("devtool");
    let mut file = fs::File::create(&executable).unwrap();
    writeln!(file, "#!/bin/sh").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let environment = VerifiedPtyEnvironment {
        identity: "build-env".into(),
        shell: fs::canonicalize("/bin/sh").unwrap(),
        environment: BTreeMap::from([(
            "BUILDDIR".into(),
            root.join("build").display().to_string(),
        )]),
    };
    let contexts = PtyContextAuthority::new(
        "workspace".into(),
        root.join("source"),
        root.join("build"),
        environment,
        Vec::new(),
        Vec::new(),
        vec![PtyContextEntry {
            identity: "busybox-workspace".into(),
            directory: root.join("workspace/busybox"),
        }],
        Vec::new(),
        Vec::new(),
    )
    .unwrap();
    let executable = fs::canonicalize(executable).unwrap();
    let build = fs::canonicalize(root.join("build")).unwrap();
    let router = PtyDevtoolRouter::new(
        contexts,
        executable.clone(),
        compatibility(&build, &executable),
    )
    .unwrap();
    let status = DevtoolStatus {
        identity: RecipeIdentity {
            name: "busybox".into(),
            file: root.join("source/meta/recipes-core/busybox.bb"),
        },
        capability: DevtoolCapability::Available,
        workspace: DevtoolWorkspace::Present {
            source_path: root.join("workspace/busybox"),
            recipe_file: None,
        },
        git: DevtoolGitState::Available {
            repository_root: Some(root.join("workspace/busybox")),
            branch: Some("devtool".into()),
            upstream: None,
            ahead: 0,
            behind: 0,
            head: Some("abc".into()),
            modified: 0,
            untracked: 0,
            conflicted: 0,
        },
        error: None,
    };
    (root, router, status)
}

mod compatibility_devtool_previews_workspace_shell_and_authorized_exact_edit_recipe;

mod compatibility_devtool_rejects_unavailable_edit_recipe_with_probe_reason;

mod pty_devtool_keeps_noninteractive_actions_on_existing_job_path;

mod pty_devtool_rejects_stale_status_workspace_and_recipe;
