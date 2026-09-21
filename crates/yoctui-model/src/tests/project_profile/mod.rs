use super::*;
use crate::{Layer, Recipe};
use std::path::PathBuf;

fn profile() -> ProjectProfile {
    ProjectProfile {
        schema_version: PROJECT_PROFILE_SCHEMA_VERSION,
        favorites: ProjectFavorites {
            recipes: vec!["busybox".into()],
            images: vec!["core-image-minimal".into()],
            layers: vec!["meta-poky".into()],
        },
        build_presets: vec![ProjectBuildPreset {
            name: "qemu smoke".into(),
            targets: vec!["core-image-minimal".into()],
            machine: Some("qemux86-64".into()),
            distro: None,
            options: ProjectBuildOptions {
                jobs: Some(4),
                continue_on_error: false,
            },
        }],
        workflows: vec![ProjectWorkflow {
            name: "smoke".into(),
            steps: vec![
                ProjectWorkflowStep::RefreshMetadata,
                ProjectWorkflowStep::UseBuildPreset {
                    preset: "qemu smoke".into(),
                },
                ProjectWorkflowStep::OpenProjectFile {
                    path: PortableProjectPath::new("conf/team.inc").unwrap(),
                },
            ],
        }],
    }
}

mod project_profile_accepts_typed_team_intent;

mod project_profile_rejects_schema_duplicates_and_unknown_presets;

mod project_profile_paths_reject_absolute_escape_and_platform_syntax;

mod project_profile_workflows_are_closed_typed_actions;

mod project_profile_resolution_keeps_stale_and_ambiguous_explicit;

mod project_profile_items_resolve_only_against_authoritative_workspace;
