use super::*;

fn environment_app() -> App {
    let mut app = App::new(32, 8192);
    app.screen = Screen::BuildEnvironment;
    app.focus = FocusTarget::Workspace;
    app
}

fn assert_controls(app: &App) {
    for (width, height) in [(80, 24), (100, 30), (160, 40), (200, 50)] {
        let output = rendered_text(app, width, height);
        for label in [
            "Enter/e Configure paths",
            "b Browse directories",
            "Alt+a Advanced TOML",
            "c Clone Poky",
            "Alt+v Verify BitBake",
            "n/Alt+n profile item",
            "p preview/open",
        ] {
            assert!(
                output.contains(label),
                "{width}x{height} lost {label}: {output}"
            );
        }
        assert!(!output.contains("A Advanced TOML"), "{output}");
        assert!(!output.contains("V Initialize"), "{output}");
        assert!(!output.contains("N/n profile"), "{output}");
    }
    assert_eq!(app.focus, FocusTarget::Workspace);
    assert!(app.dialogs.is_empty());
}

#[test]
fn demo_environment_controls_remain_visible_with_long_typed_environment_states() {
    let mut app = environment_app();
    let profile = yoctui_model::BuildEnvironmentProfile {
        source_dir: format!("/source/{}", "nested/".repeat(40)).into(),
        build_dir: format!("/build/{}", "nested/".repeat(40)).into(),
        init_script: "/source/oe-init-build-env".into(),
    };
    for state in [
        BuildEnvironmentState::Unconfigured,
        BuildEnvironmentState::Configured(profile.clone()),
        BuildEnvironmentState::Verifying {
            profile: profile.clone(),
            generation: 7,
        },
        BuildEnvironmentState::Connected(profile.clone()),
        BuildEnvironmentState::Failed {
            profile,
            message: "native authority unavailable ".repeat(50),
        },
    ] {
        app.build_environment = state;
        app.available_images = (0..30).map(|i| format!("image-{i}")).collect();
        let before = app.build_environment.clone();
        assert_controls(&app);
        assert_eq!(app.build_environment, before);
    }
}

#[test]
fn demo_environment_controls_preserve_project_profile_facts_and_error_identity() {
    let mut app = environment_app();
    for state in [
        yoctui_model::ProjectProfileState::NotLoaded,
        yoctui_model::ProjectProfileState::Absent,
        yoctui_model::ProjectProfileState::Invalid("unsupported schema version 9".into()),
        yoctui_model::ProjectProfileState::Loaded(yoctui_model::ProjectProfile {
            schema_version: yoctui_model::PROJECT_PROFILE_SCHEMA_VERSION,
            favorites: yoctui_model::ProjectFavorites {
                recipes: vec!["busybox".into(), "missing-recipe".into()],
                images: Vec::new(),
                layers: Vec::new(),
            },
            build_presets: Vec::new(),
            workflows: Vec::new(),
        }),
    ] {
        app.project_profile = state;
        app.workspace.recipes = vec![yoctui_model::Recipe {
            name: "busybox".into(),
            ..Default::default()
        }];
        assert_controls(&app);
        let output = rendered_text(&app, 80, 24);
        match &app.project_profile {
            yoctui_model::ProjectProfileState::Loaded(_) => {
                assert!(
                    output.contains("Recipe favorite: busybox — resolved"),
                    "{output}"
                );
                let full = rendered_text(&app, 200, 50);
                assert!(
                    full.contains("missing-recipe — STALE: not reported by BitBake"),
                    "{full}"
                );
            }
            yoctui_model::ProjectProfileState::Invalid(_) => {
                assert!(output.contains("unsupported schema version 9"), "{output}");
            }
            _ => assert!(output.contains("Project profile:"), "{output}"),
        }
    }
}
