use super::*;
use crate::{CommandId, Dialog, Effect, Recipe, command_action};

fn target_guide() -> App {
    let mut app = App::new(32, 4096);
    app.workspace.recipes = vec![
        Recipe {
            name: "core-image-minimal".into(),
            ..Recipe::default()
        },
        Recipe {
            name: "vendor-image-debug".into(),
            ..Recipe::default()
        },
        Recipe {
            name: "busybox".into(),
            ..Recipe::default()
        },
    ];
    app.onboarding
        .progress
        .completed
        .insert(OnboardingStep::Environment);
    app.onboarding.progress.current = OnboardingStep::Target;
    update(&mut app, Action::OpenOnboarding);
    app
}

#[test]
fn about_and_help_are_distinct_menu_routes_and_f1_resets_about() {
    let mut app = App::new(32, 4096);
    assert_eq!(
        command_action(&app, CommandId::OpenAbout),
        Action::OpenAbout
    );
    update(&mut app, Action::OpenAbout);
    assert!(app.help_about);
    assert_eq!(app.screen, Screen::Help);
    let help = command_action(&app, CommandId::OpenHelp);
    update(&mut app, help);
    assert!(!app.help_about);
    assert_eq!(app.help_scroll, 0);
    app.dialogs.push_back(Dialog::QuitConfirmation);
    update(&mut app, Action::OpenAbout);
    assert!(!app.help_about, "About must not bypass an active dialog");
}

#[test]
fn guide_target_uses_current_recipes_even_before_cached_inventory_is_populated() {
    let mut app = target_guide();
    assert!(app.available_images.is_empty());
    assert_eq!(
        app.onboarding_projection().rows[1].status,
        OnboardingStepStatus::Current
    );
    assert_eq!(update(&mut app, Action::ActivateOnboardingStep), None);
    let Some(Dialog::ImagePicker(picker)) = app.active_dialog() else {
        panic!("image picker")
    };
    assert_eq!(picker.images, ["core-image-minimal", "vendor-image-debug"]);
    assert!(!app.onboarding.open);
    update(&mut app, Action::ConfirmImagePicker);
    assert_eq!(app.build.target.as_deref(), Some("core-image-minimal"));
    assert!(app.onboarding.open);
    assert_eq!(app.build.status, BuildStatus::Idle);
    assert_eq!(
        update(&mut app, Action::AdvanceOnboarding),
        Some(Effect::PersistOnboarding)
    );
    assert_eq!(app.onboarding.progress.current, OnboardingStep::FirstBuild);
}

#[test]
fn guide_cancel_target_returns_without_starting_or_completing_a_build() {
    let mut app = target_guide();
    update(&mut app, Action::ActivateOnboardingStep);
    update(&mut app, Action::CancelImagePicker);
    assert!(app.onboarding.open);
    assert!(app.build.target.is_none());
    update(&mut app, Action::AdvanceOnboarding);
    assert_eq!(app.onboarding.progress.current, OnboardingStep::Target);
    assert!(
        !app.onboarding
            .progress
            .completed
            .contains(&OnboardingStep::Target)
    );
}

#[test]
fn guide_blocks_future_steps_and_unavailable_inventory_without_closing() {
    let mut app = target_guide();
    app.onboarding.selected = OnboardingStep::FirstBuild;
    update(&mut app, Action::ActivateOnboardingStep);
    assert!(app.onboarding.open);
    assert!(app.active_dialog().is_none());
    app.onboarding.selected = OnboardingStep::Target;
    app.workspace.recipes.clear();
    update(&mut app, Action::ActivateOnboardingStep);
    assert!(app.onboarding.open);
    assert_eq!(
        app.onboarding_projection().rows[1].status,
        OnboardingStepStatus::Unavailable
    );
}

#[test]
fn guide_build_review_cancel_and_confirmation_preserve_explicit_safety() {
    let mut app = target_guide();
    app.build.target = Some("core-image-minimal".into());
    update(&mut app, Action::AdvanceOnboarding);
    update(&mut app, Action::ActivateOnboardingStep);
    assert!(matches!(app.active_dialog(), Some(Dialog::BuildOptions)));
    update(&mut app, Action::CloseBuildOptions);
    assert!(app.onboarding.open);
    assert_eq!(app.build.status, BuildStatus::Idle);
    update(&mut app, Action::ActivateOnboardingStep);
    update(&mut app, Action::BeginBuildTargetTask(None));
    update(&mut app, Action::ConfirmBuildTarget);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::RecipeTaskConfirmation(_))
    ));
    update(&mut app, Action::CancelRecipeTask);
    assert!(app.onboarding.open);
    assert_eq!(app.build.status, BuildStatus::Idle);
    update(&mut app, Action::ActivateOnboardingStep);
    update(&mut app, Action::BeginBuildTargetTask(None));
    update(&mut app, Action::ConfirmBuildTarget);
    assert!(
        matches!(update(&mut app, Action::ConfirmRecipeTask), Some(Effect::Start(request))
        if request.targets == ["core-image-minimal"] && request.task.is_none())
    );
    assert!(!app.onboarding.open);
    assert!(app.onboarding.resume_after_dialog.is_none());
}

#[test]
fn guide_unrelated_build_or_recipe_does_not_count_as_an_image_build() {
    let mut app = target_guide();
    app.build.target = Some("busybox".into());
    update(
        &mut app,
        Action::BuildCompleted {
            success: true,
            exit_code: Some(0),
        },
    );
    assert!(!onboarding_completion_evidence(
        &app,
        OnboardingStep::Target
    ));
    app.build.target = Some("core-image-minimal".into());
    assert!(!onboarding_completion_evidence(
        &app,
        OnboardingStep::FirstBuild
    ));
    assert!(!onboarding_completion_evidence(
        &app,
        OnboardingStep::Diagnostics
    ));
    update(
        &mut app,
        Action::BuildCompleted {
            success: true,
            exit_code: Some(0),
        },
    );
    assert!(onboarding_completion_evidence(
        &app,
        OnboardingStep::FirstBuild
    ));
    assert!(onboarding_completion_evidence(
        &app,
        OnboardingStep::Diagnostics
    ));
}

#[test]
fn guide_artifacts_step_can_initiate_discovery_before_inventory_is_loaded() {
    let mut app = target_guide();
    app.build.target = Some("core-image-minimal".into());
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    app.onboarding.progress.completed.extend([
        OnboardingStep::Target,
        OnboardingStep::FirstBuild,
        OnboardingStep::Diagnostics,
    ]);
    update(
        &mut app,
        Action::BuildCompleted {
            success: true,
            exit_code: Some(0),
        },
    );
    app.dialogs.clear();
    app.onboarding.progress.current = OnboardingStep::ArtifactsRootfs;
    app.onboarding.selected = OnboardingStep::ArtifactsRootfs;
    assert_eq!(
        app.onboarding_projection().rows[4].status,
        OnboardingStepStatus::Current
    );
    assert!(matches!(
        update(&mut app, Action::ActivateOnboardingStep),
        Some(Effect::GetImageArtifacts(_))
    ));
    assert_eq!(app.screen, Screen::Images);
    assert!(!app.onboarding.open);
}

#[test]
fn guide_finishes_all_six_steps_with_matching_artifacts_and_current_daemon() {
    use crate::{
        ImageArtifact, ImageArtifactField as Field, ImageArtifactIdentity, ImageArtifactInventory,
        ImageArtifactKind,
    };
    let mut app = target_guide();
    app.build.target = Some("core-image-minimal".into());
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    update(&mut app, Action::AdvanceOnboarding);
    update(
        &mut app,
        Action::BuildCompleted {
            success: true,
            exit_code: Some(0),
        },
    );
    app.dialogs.clear();
    update(&mut app, Action::AdvanceOnboarding);
    assert_eq!(app.onboarding.progress.current, OnboardingStep::Diagnostics);
    update(&mut app, Action::ActivateOnboardingStep);
    assert_eq!(app.screen, Screen::Logs);
    update(&mut app, Action::OpenOnboarding);
    update(&mut app, Action::AdvanceOnboarding);
    let Some(Effect::GetImageArtifacts(request)) = update(&mut app, Action::ActivateOnboardingStep)
    else {
        panic!("discover artifacts")
    };
    let mut inventory = ImageArtifactInventory {
        machine: "qemux86-64".into(),
        deploy_directory: Field::Available("/build/deploy".into()),
        artifacts: vec![ImageArtifact {
            identity: ImageArtifactIdentity {
                machine: "qemux86-64".into(),
                image: "another-image".into(),
                path: "/build/deploy/another-image.ext4".into(),
            },
            kind: ImageArtifactKind::RootFilesystem,
            size_bytes: Field::Available(1024),
            modified_unix_seconds: Field::Unavailable,
            checksums: Field::Unavailable,
            manifests: Field::Unavailable,
            licenses: Field::Unavailable,
            spdx: Field::Unavailable,
            wic_files: Field::Unavailable,
        }],
    };
    update(
        &mut app,
        Action::ImageArtifactInventoryLoaded {
            request: request.clone(),
            inventory: inventory.clone(),
        },
    );
    update(&mut app, Action::OpenOnboarding);
    update(&mut app, Action::AdvanceOnboarding);
    assert_eq!(
        app.onboarding.progress.current,
        OnboardingStep::ArtifactsRootfs,
        "another image's artifacts must not complete this step"
    );
    inventory.artifacts[0].identity.image = "core-image-minimal".into();
    app.image_artifacts = ImageArtifactInventoryState::Available { request, inventory };
    update(&mut app, Action::AdvanceOnboarding);
    assert_eq!(app.onboarding.progress.current, OnboardingStep::Terminal);
    app.daemon.status = ClientReplicaStatus::Current;
    update(&mut app, Action::ActivateOnboardingStep);
    assert_eq!(app.screen, Screen::TerminalSessions);
    update(&mut app, Action::OpenOnboarding);
    update(&mut app, Action::AdvanceOnboarding);
    assert!(!app.onboarding.open);
    assert!(app.onboarding_projection().finished);
    assert!(app.onboarding.progress.skipped.is_empty());
}

#[test]
fn guide_image_inventory_refreshes_after_verification_and_recipe_changes() {
    let mut app = App::new_unconfigured(32, 4096);
    update(
        &mut app,
        Action::RecipesLoaded(vec![Recipe {
            name: "core-image-minimal".into(),
            ..Recipe::default()
        }]),
    );
    assert!(app.available_images.is_empty());
    app.build_environment = BuildEnvironmentState::Verifying {
        profile: crate::BuildEnvironmentProfile {
            source_dir: "/src".into(),
            build_dir: "/build".into(),
            init_script: "/src/oe-init-build-env".into(),
        },
        generation: 1,
    };
    update(&mut app, Action::BuildEnvironmentVerified { generation: 1 });
    assert_eq!(app.available_images, ["core-image-minimal"]);
    update(
        &mut app,
        Action::RecipesLoaded(vec![Recipe {
            name: "vendor-image-debug".into(),
            ..Recipe::default()
        }]),
    );
    assert_eq!(app.available_images, ["vendor-image-debug"]);
    update(&mut app, Action::RecipesLoaded(vec![]));
    assert!(!app.has_image_targets());
    assert!(app.available_images.is_empty());
}

#[test]
fn guide_image_targets_exclude_image_libraries_and_include_custom_image_providers() {
    let mut app = target_guide();
    app.workspace.recipes.extend([
        Recipe {
            name: "python3-imagesize".into(),
            ..Recipe::default()
        },
        Recipe {
            name: "imagemagick".into(),
            ..Recipe::default()
        },
        Recipe {
            name: "nativesdk-core-image-minimal".into(),
            ..Recipe::default()
        },
        Recipe {
            name: "core-image-minimal-native".into(),
            ..Recipe::default()
        },
        Recipe {
            name: "my-appliance".into(),
            file: Some("/layers/meta/recipes-core/images/my-appliance.bb".into()),
            ..Recipe::default()
        },
    ]);
    assert_eq!(
        app.image_targets(),
        ["core-image-minimal", "my-appliance", "vendor-image-debug"]
    );
    assert!(!app.is_image_target("python3-imagesize"));
    assert!(app.is_image_target("my-appliance"));
}
