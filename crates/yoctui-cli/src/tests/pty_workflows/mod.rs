use std::{collections::BTreeMap, fs};

use yoctui_app::{
    PtyBitBakeInteractiveTask, PtyContextAuthority, PtyContextEntry, PtyDevtoolAction,
    PtyDevtoolRouter, PtyInteractiveRecipe, PtyMenuconfigAction, PtyMenuconfigRouter,
    PtySdkShellAction, PtySdkShellRouter, VerifiedPtyEnvironment,
};
use yoctui_bitbake::SdkShellAdapter;
use yoctui_model::{
    AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
    CapabilityId, CapabilityImplementation, CapabilityImplementationKind, CapabilityRecord,
    CapabilitySnapshot, CapabilityState, DaemonCompatibilitySnapshot, DevtoolCapability,
    DevtoolGitState, DevtoolStatus, DevtoolWorkspace, IdentityAuthority, PtySessionKind,
    RecipeIdentity, ToolIdentity, YoctoEnvironmentIdentity,
};

fn devtool_edit_compatibility(
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
                    argv: vec![executable.display().to_string(), "--help".into()],
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

mod pty_devtool_cli_composition_preserves_exact_interactive_routes;
mod pty_menuconfig_cli_composition_uses_exact_authoritative_bitbake_argv;
mod pty_sdk_shell_cli_composition_captures_and_routes_persistent_environments;
