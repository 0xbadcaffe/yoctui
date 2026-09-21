#[test]
fn ux_responsive_all_screens_and_dialogs_render_at_boundary_sizes() {
    let screens = [
        Screen::Dashboard,
        Screen::Tasks,
        Screen::BuildHistory,
        Screen::Dependencies,
        Screen::LayerRelationships,
        Screen::Recipes,
        Screen::Images,
        Screen::Sdk,
        Screen::Testing,
        Screen::Security,
        Screen::Layers,
        Screen::Configuration,
        Screen::Bbmask,
        Screen::Maintenance,
        Screen::Logs,
        Screen::Errors,
        Screen::Help,
        Screen::BuildEnvironment,
        Screen::Compatibility,
        Screen::Settings,
    ];
    for screen in screens {
        for (width, height) in [
            (200, 60),
            (160, 50),
            (130, 40),
            (100, 30),
            (80, 24),
            (79, 23),
        ] {
            let mut app = App::new(10, 1_000);
            app.screen = screen;
            let output = rendered_text(&app, width, height);
            assert!(
                !output.contains('\u{fffd}'),
                "{screen:?} at {width}x{height}"
            );
        }
    }

    let mut build_options = App::new(10, 1_000);
    build_options.dialogs.push_back(Dialog::BuildOptions);
    build_options.focus = FocusTarget::Dialog;
    let _ = rendered_text(&build_options, 80, 24);

    let mut palette = App::new(10, 1_000);
    palette.command_palette_open = true;
    palette.focus = FocusTarget::CommandPalette;
    let _ = rendered_text(&palette, 80, 24);

    let mut confirmation = App::new(10, 1_000);
    confirmation
        .dialogs
        .push_back(Dialog::RecipeTaskConfirmation(BuildRequest {
            targets: vec!["base-files".into()],
            task: Some("listtasks".into()),
            force: false,
        }));
    confirmation.focus = FocusTarget::Dialog;
    let _ = rendered_text(&confirmation, 80, 24);
}
#[test]
fn dialog_families_render_on_narrow_supported_terminals() {
    let dialogs = vec![
        (Dialog::BuildOptions, "Image build options"),
        (Dialog::BuildCompletion, "Build finished"),
        (
            Dialog::BuildTarget {
                editor: yoctui_model::PopupEditor::new("target = \"busybox\"\n".into()),
                task: None,
            },
            "Build target",
        ),
        (
            Dialog::ImagePicker(yoctui_model::ImagePicker {
                images: vec!["core-image-minimal".into()],
                selection: 0,
            }),
            "Available image targets",
        ),
        (
            Dialog::RecipeTaskConfirmation(BuildRequest {
                targets: vec!["busybox".into()],
                task: None,
                force: false,
            }),
            "Confirm recipe task",
        ),
        (
            Dialog::DevtoolModifyConfirmation(yoctui_model::RecipeIdentity {
                name: "busybox".into(),
                file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
            }),
            "Confirm Devtool modify",
        ),
        (
            Dialog::DevtoolResetConfirmation(yoctui_model::DevtoolResetPlan {
                identity: yoctui_model::RecipeIdentity {
                    name: "busybox".into(),
                    file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
                },
                source_path: "/build/workspace/sources/busybox".into(),
            }),
            "Confirm Devtool reset",
        ),
        (
            Dialog::DevtoolUpdateConfirmation(yoctui_model::RecipeIdentity {
                name: "busybox".into(),
                file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
            }),
            "Confirm Devtool update-recipe",
        ),
        (
            Dialog::DevtoolFinishPicker(yoctui_model::DevtoolFinishPicker {
                identity: yoctui_model::RecipeIdentity {
                    name: "busybox".into(),
                    file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
                },
                layers: vec![yoctui_model::Layer {
                    name: "meta".into(),
                    path: "/layers/meta".into(),
                    priority: Some(5),
                }],
                selection: 0,
            }),
            "Devtool finish busybox",
        ),
        (
            Dialog::DevtoolFinishConfirmation(yoctui_model::DevtoolFinishPlan {
                identity: yoctui_model::RecipeIdentity {
                    name: "busybox".into(),
                    file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
                },
                layer: yoctui_model::Layer {
                    name: "meta".into(),
                    path: "/layers/meta".into(),
                    priority: Some(5),
                },
            }),
            "Confirm Devtool finish",
        ),
        (
            Dialog::DevtoolDeploy(yoctui_model::DevtoolDeployDraft {
                identity: yoctui_model::RecipeIdentity {
                    name: "busybox".into(),
                    file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
                },
                target: "qemu".into(),
            }),
            "Devtool deploy target",
        ),
        (
            Dialog::DevtoolDeployConfirmation(yoctui_model::DevtoolDeployPlan {
                identity: yoctui_model::RecipeIdentity {
                    name: "busybox".into(),
                    file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
                },
                target: "qemu".into(),
            }),
            "Confirm Devtool deploy-target",
        ),
        (
            Dialog::BbmaskEdit(yoctui_model::PopupEditor::new(
                "bbmask = \"meta-old/.*\"\n".into(),
            )),
            "BBMASK.toml",
        ),
        (
            Dialog::BbmaskConfirmation("meta-old/.*".into()),
            "Confirm BBMASK change",
        ),
        (
            Dialog::RecipeEditor(RecipeEditor {
                recipe: "busybox".into(),
                root: "/workspace/busybox".into(),
                files: vec!["main.c".into()],
                selection: 0,
                focus: yoctui_model::RecipeEditorFocus::Files,
                language: yoctui_model::SourceLanguage::C,
                document: yoctui_model::TextAreaState::new("int main() {}".into()),
                searching: false,
            }),
            "Workspace file tree",
        ),
        (
            Dialog::BuildCancellationConfirmation,
            "Confirm build cancellation",
        ),
        (Dialog::QuitConfirmation, "Confirm exit"),
    ];

    for (dialog, title) in dialogs {
        let mut app = App::new(10, 1_000);
        app.build.status = yoctui_model::BuildStatus::Completed;
        app.dialogs.push_back(dialog);
        let output = rendered_text(&app, 80, 24);
        assert!(output.contains(title), "missing {title} in narrow dialog");
        assert!(output.contains("modal ·"), "missing modal shell: {output}");
    }
}

#[test]
fn next_generation_dialogs_name_standard_confirmation_destructive_and_result_shells() {
    let reset = yoctui_model::DevtoolResetPlan {
        identity: yoctui_model::RecipeIdentity {
            name: "busybox".into(),
            file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
        },
        source_path: "/build/workspace/sources/busybox".into(),
    };
    for (dialog, expected, control) in [
        (Dialog::BuildOptions, "modal · Image build options", "Esc"),
        (
            Dialog::RecipeTaskConfirmation(BuildRequest {
                targets: vec!["busybox".into()],
                task: Some("compile".into()),
                force: false,
            }),
            "confirm modal · Confirm recipe task",
            "Esc",
        ),
        (
            Dialog::DevtoolResetConfirmation(reset),
            "destructive modal · Confirm Devtool reset",
            "Esc",
        ),
        (
            Dialog::BuildCompletion,
            "result modal · Build finished",
            "any key",
        ),
    ] {
        let mut app = App::new(32, 8192);
        app.focus = FocusTarget::Dialog;
        app.build.status = BuildStatus::Completed;
        app.dialogs.push_front(dialog);
        for (width, height) in [(160, 40), (100, 30), (80, 24)] {
            let output = rendered_text(&app, width, height);
            assert!(output.contains(expected), "{width}x{height}: {output}");
            assert!(output.contains(control), "{width}x{height}: {output}");
        }
    }
}

#[test]
fn next_generation_dialogs_reserve_validation_fields_and_controls() {
    let mut app = qemu_workspace_app();
    app.focus = FocusTarget::Dialog;
    let artifact = app.selected_image_artifact().unwrap().identity.clone();
    let mut launch = QemuLaunchDialog::new(yoctui_model::QemuLaunchDraft::for_artifact(
        artifact,
        yoctui_model::ImageArtifactKind::Wic,
    ));
    launch.selected_field = QemuLaunchField::Kernel;
    launch.editing = true;
    launch.draft.kernel = "relative/kernel".into();
    launch.validation_error = Some("kernel path must be absolute".repeat(20));
    app.dialogs.push_front(Dialog::QemuLaunch(launch));
    for (width, height) in [(160, 40), (100, 30), (80, 24)] {
        let output = rendered_text(&app, width, height);
        for expected in [
            "modal · Launch runqemu",
            "▶ Kernel [editing]",
            "✕ Validation: kernel path must be absolute",
            "[p] Preview",
            "[Esc] Close",
        ] {
            assert!(output.contains(expected), "{width}x{height}: {output}");
        }
        assert_eq!(output.matches("▶ Kernel").count(), 1, "{output}");
    }

    app.dialogs.clear();
    let mut editor =
        yoctui_model::PopupEditor::new("destination = \"⟦/exports/result.xml⟧\"\n".into());
    editor.cursor = editor.text.len();
    app.dialogs.push_front(Dialog::SdkPublishTomlEditor(editor));
    let output = rendered_text(&app, 80, 24);
    assert!(
        output.contains("modal · SDK publish.toml — NORMAL"),
        "{output}"
    );
    assert!(output.contains("[Enter] Save/preview"), "{output}");
    assert!(output.contains("Home/End line"), "{output}");
    assert!(output.contains("Ctrl+V paste"), "{output}");
}

#[test]
fn next_generation_dialogs_keep_accessible_focus_and_bounded_geometry() {
    for (theme, color_enabled) in [
        (Theme::HighContrast, true),
        (Theme::Monochrome, true),
        (Theme::DarkPro, false),
    ] {
        let mut app = App::new(32, 8192);
        app.theme = theme;
        app.color_enabled = color_enabled;
        app.reduced_motion = true;
        app.focus = FocusTarget::Dialog;
        app.dialogs.push_front(Dialog::QuitConfirmation);
        let output = rendered_text(&app, 80, 24);
        assert!(
            output.contains("destructive modal · Confirm exit"),
            "{output}"
        );
        assert!(
            output.contains("Are you sure you want to exit yoctui?"),
            "{output}"
        );
        assert!(output.contains("[y/Enter] Exit yoctui"), "{output}");
        assert!(output.contains("[n/Esc] Stay"), "{output}");
    }

    let mut cancellation = App::new(32, 8192);
    cancellation.focus = FocusTarget::Dialog;
    cancellation.build.status = BuildStatus::Running;
    cancellation
        .dialogs
        .push_front(Dialog::BuildCancellationConfirmation);
    let output = rendered_text(&cancellation, 80, 24);
    assert!(
        output.contains("Are you sure you want to cancel the build?"),
        "{output}"
    );
    assert!(output.contains("[y/Enter] Cancel build"), "{output}");
    assert!(output.contains("[n/Esc] Keep building"), "{output}");

    for area in [
        Rect::new(0, 0, 200, 60),
        Rect::new(0, 0, 100, 30),
        Rect::new(0, 0, 80, 24),
        Rect::new(7, 11, 80, 24),
    ] {
        let popup = dialog_popup_rect(area, 110, 30);
        assert!(popup.x >= area.x && popup.y >= area.y);
        assert!(popup.right() <= area.right() && popup.bottom() <= area.bottom());
        assert!(popup.width <= area.width && popup.height <= area.height);
        if area.width >= 2 && area.height >= 2 {
            assert!(popup.x > area.x && popup.y > area.y);
        }
    }
}

#[test]
fn command_palette_renders_search_results_and_disabled_explanations() {
    let mut app = App::new(10, 1_000);
    app.command_palette_open = true;
    app.command_palette_query = "build".into();
    let output = rendered_text(&app, 100, 25);
    assert!(output.contains("[EDITING] Query: build▏"));
    assert!(output.contains("Ctrl+U clear"));
    assert!(output.contains("Build image"));
    assert!(output.contains("Cannot run:"));
    assert!(output.contains("Load a Yocto workspace first"));
    assert!(output.contains("Type search"));

    app.command_palette_query = "nothing matches this".into();
    let output = rendered_text(&app, 80, 24);
    assert!(output.contains("No commands match"));
}

#[test]
fn global_search_starts_empty_excludes_commands_and_renders_inline_errors() {
    let mut app = App::new(10, 1_000);
    app.command_palette_open = true;
    app.command_palette_mode = CommandPaletteMode::GlobalRegexSearch;
    app.command_palette_query = "^open (packages|sdk)$".into();
    let output = rendered_text(&app, 100, 25);
    assert!(output.contains("Global Regex Search"), "{output}");
    assert!(!output.contains("Open Packages"), "{output}");
    assert!(!output.contains("Open SDK"), "{output}");
    assert!(output.contains("Content regex"), "{output}");
    app.command_palette_query.clear();
    for (width, height) in [(100, 25), (80, 24)] {
        let output = rendered_text(&app, width, height);
        assert!(output.contains("0 matches"), "{output}");
        assert!(output.contains("Type a regular expression"), "{output}");
    }

    app.command_palette_query = "[".into();
    let output = rendered_text(&app, 100, 25);
    assert!(output.contains("Invalid regular expression"), "{output}");
    assert!(output.contains("unclosed character class"), "{output}");
}

#[test]
fn global_search_renders_generated_image_service_with_provenance() {
    let mut app = App::new(10, 1_000);
    app.command_palette_open = true;
    app.command_palette_mode = CommandPaletteMode::GlobalRegexSearch;
    app.command_palette_query = "watchdog".into();
    app.global_search_content = yoctui_model::GlobalSearchContentState::Ready {
            generation: 1,
            query: "watchdog".into(),
            hits: vec![yoctui_model::GlobalSearchHit {
                kind: yoctui_model::GlobalSearchContentKind::ImageRootfs,
                path: "/build/tmp/work/qemux/core-image-demo/1.0/rootfs/usr/lib/systemd/system/watchdog.service".into(),
                line: 4,
                column: 13,
                preview: "Description=watchdog service".into(),
                image: Some("core-image-demo".into()),
            }],
            truncated: false,
            searched_scopes: vec!["generated-work=/build/tmp/work".into()],
        };
    let output = rendered_text(&app, 120, 28);
    for expected in [
        "Generated image rootfs",
        "watchdog.service",
        "line 4 · column 13",
        "Description=watchdog service",
        "Generated image: core-image-demo",
    ] {
        assert!(output.contains(expected), "missing {expected}: {output}");
    }
}
