//! Regression tests grouped around devwork_editor_detects_languages_and_supports_search_undo_and_redo.
use super::*;

#[test]
fn devwork_editor_detects_languages_and_supports_search_undo_and_redo() {
    assert_eq!(
        SourceLanguage::from_path(Path::new("recipe.bbappend")),
        SourceLanguage::BitBake
    );
    assert_eq!(
        SourceLanguage::from_path(Path::new("src/main.cpp")),
        SourceLanguage::Cpp
    );
    assert_eq!(
        SourceLanguage::from_path(Path::new("src/lib.rs")),
        SourceLanguage::Rust
    );
    assert_eq!(
        SourceLanguage::from_path(Path::new("Makefile")),
        SourceLanguage::Make
    );
    assert_eq!(
        SourceLanguage::from_path(Path::new("arch/arm/boot/dts/board.dts")),
        SourceLanguage::DeviceTree
    );
    assert_eq!(
        SourceLanguage::from_path(Path::new("soc/common.dtsi")),
        SourceLanguage::DeviceTree
    );

    let mut app = App::new(10, 1_000);
    let _ = update(
        &mut app,
        Action::OpenRecipeEditor {
            recipe: "demo".into(),
            root: "/workspace/demo".into(),
            files: vec!["src/main.cpp".into()],
        },
    );
    let _ = update(
        &mut app,
        Action::LoadRecipeEditorContent("int main() { return 0; }".into()),
    );
    let _ = update(
        &mut app,
        Action::FocusRecipeEditor(RecipeEditorFocus::Document),
    );
    let _ = update(
        &mut app,
        Action::EditRecipeEditor(PopupEditorCommand::ToggleInsert),
    );
    let _ = update(
        &mut app,
        Action::EditRecipeEditor(PopupEditorCommand::Newline),
    );
    let _ = update(&mut app, Action::EditRecipeEditor(PopupEditorCommand::Undo));
    let _ = update(&mut app, Action::EditRecipeEditor(PopupEditorCommand::Redo));
    let _ = update(&mut app, Action::BeginRecipeEditorSearch);
    for character in "return".chars() {
        let _ = update(&mut app, Action::AppendRecipeEditorSearch(character));
    }
    let editor = match app.active_dialog() {
        Some(Dialog::RecipeEditor(editor)) => editor,
        other => panic!("expected recipe editor, got {other:?}"),
    };
    assert_eq!(editor.language, SourceLanguage::Cpp);
    assert_eq!(editor.document.search_state().matches.len(), 1);
    assert!(editor.is_dirty());
}

#[test]
fn devwork_editor_reports_structural_diagnostics_without_claiming_compiler_authority() {
    let diagnostics = source_structural_validation(SourceLanguage::C, "int main( {\n");
    assert!(
        diagnostics
            .iter()
            .any(|span| span.message.contains("parentheses"))
    );
    assert!(
        diagnostics
            .iter()
            .any(|span| span.message.contains("braces"))
    );
    let bitbake = source_structural_validation(SourceLanguage::BitBake, "SUMMARY =\n");
    assert!(
        bitbake
            .iter()
            .any(|span| span.message.contains("assignment has no value"))
    );
}
#[test]
fn clean_state_requires_confirmation_before_starting() {
    let mut app = App::new(10, 1_000);
    app.workspace.recipes = vec![Recipe {
        name: "busybox".into(),
        version: None,
        layer: None,
        ..Recipe::default()
    }];
    app.recipe_metadata.insert(
        "busybox".into(),
        RecipeMetadata {
            recipe: "busybox".into(),
            tasks: Some(vec!["do_cleansstate".into()]),
            ..RecipeMetadata::default()
        },
    );
    let _ = update(&mut app, Action::BeginSelectedRecipeCleanState);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::RecipeTaskConfirmation(_))
    ));
    assert_eq!(app.build.status, BuildStatus::Idle);

    assert_eq!(
        update(&mut app, Action::ConfirmRecipeTask),
        Some(Effect::Start(BuildRequest {
            targets: vec!["busybox".into()],
            task: Some("cleansstate".into()),
            force: false,
        }))
    );
    assert_eq!(app.build.status, BuildStatus::LoadingWorkspace);
}
#[test]
fn layer_selection_stays_in_workspace_bounds() {
    let mut app = App::new(10, 1_000);
    app.workspace.layers = vec![
        Layer {
            name: "alpha".into(),
            path: PathBuf::from("/layers/alpha"),
            priority: Some(1),
        },
        Layer {
            name: "beta".into(),
            path: PathBuf::from("/layers/beta"),
            priority: None,
        },
    ];
    let _ = update(&mut app, Action::SelectLayer { delta: 8 });
    assert_eq!(app.layer_selection, 1);
    let _ = update(&mut app, Action::SelectLayer { delta: -8 });
    assert_eq!(app.layer_selection, 0);
}
#[test]
fn selected_layer_opens_its_directory() {
    let mut app = App::new(10, 1_000);
    app.workspace.layers = vec![Layer {
        name: "meta-demo".into(),
        path: PathBuf::from("/layers/meta-demo"),
        priority: None,
    }];
    assert_eq!(
        update(&mut app, Action::OpenSelectedLayer),
        Some(Effect::OpenInEditor(PathBuf::from("/layers/meta-demo")))
    );
}
#[test]
fn selected_layer_opens_the_in_tui_workspace_editor() {
    let mut app = App::new(10, 1_000);
    app.workspace.layers = vec![Layer {
        name: "meta-demo".into(),
        path: PathBuf::from("/layers/meta-demo"),
        priority: None,
    }];
    assert_eq!(
        update(&mut app, Action::BeginSelectedLayerWorkspaceEditor),
        Some(Effect::OpenWorkspaceEditor {
            label: "Layer: meta-demo".into(),
            root: PathBuf::from("/layers/meta-demo"),
        })
    );
}
#[test]
fn layer_tree_loads_children_lazily_and_collapses_without_losing_parent() {
    let mut app = App::new(10, 1_000);
    app.workspace.layers.push(Layer {
        name: "meta-demo".into(),
        path: "/layers/meta-demo".into(),
        priority: Some(5),
    });
    assert_eq!(
        update(&mut app, Action::BeginSelectedLayerBrowser),
        Some(Effect::LoadLayerBrowserDirectory {
            layer: "meta-demo".into(),
            root: "/layers/meta-demo".into(),
            directory: "/layers/meta-demo".into(),
        })
    );
    let _ = update(
        &mut app,
        Action::LoadLayerBrowserDirectory {
            layer: "meta-demo".into(),
            root: "/layers/meta-demo".into(),
            directory: "/layers/meta-demo".into(),
            entries: vec![LayerBrowserEntry {
                path: "/layers/meta-demo/recipes-core".into(),
                is_dir: true,
                ..LayerBrowserEntry::default()
            }],
        },
    );
    assert_eq!(
        update(&mut app, Action::LayerBrowserEnter),
        Some(Effect::LoadLayerBrowserDirectory {
            layer: "meta-demo".into(),
            root: "/layers/meta-demo".into(),
            directory: "/layers/meta-demo/recipes-core".into(),
        })
    );
    let _ = update(
        &mut app,
        Action::LoadLayerBrowserDirectory {
            layer: "meta-demo".into(),
            root: "/layers/meta-demo".into(),
            directory: "/layers/meta-demo/recipes-core".into(),
            entries: vec![LayerBrowserEntry {
                path: "/layers/meta-demo/recipes-core/demo.bb".into(),
                ..LayerBrowserEntry::default()
            }],
        },
    );
    let browser = app.layer_browser.as_ref().unwrap();
    assert_eq!(browser.entries.len(), 2);
    assert_eq!(browser.entries[0].depth, 0);
    assert_eq!(browser.entries[1].depth, 1);
    assert!(
        browser
            .nodes
            .contains_key(&PathBuf::from("/layers/meta-demo/recipes-core"))
    );
    let _ = update(&mut app, Action::LayerBrowserUp);
    assert_eq!(app.layer_browser.as_ref().unwrap().entries.len(), 1);
}

#[test]
fn ux_list_tree_layer_flatten_rejects_cycles_and_hard_bounds_depth() {
    let root = PathBuf::from("/layers/meta-cycle");
    let child = root.join("child");
    let mut browser = LayerBrowser::new("meta-cycle".into(), root.clone());
    browser.nodes.insert(
        root.clone(),
        vec![LayerBrowserEntry {
            path: child.clone(),
            is_dir: true,
            ..LayerBrowserEntry::default()
        }],
    );
    browser.nodes.insert(
        child.clone(),
        vec![LayerBrowserEntry {
            path: root.clone(),
            is_dir: true,
            ..LayerBrowserEntry::default()
        }],
    );
    browser.expanded.insert(child);
    browser.rebuild(None);
    assert_eq!(browser.cycle_entries, 1);
    assert!(browser.entries.len() <= LIST_TREE_MAX_ROWS);
    assert!(
        browser
            .entries
            .iter()
            .all(|entry| entry.depth <= LIST_TREE_MAX_DEPTH)
    );
}
#[test]
fn layer_tree_hidden_filter_and_search_keep_selection_bounded() {
    let mut app = App::new(10, 1_000);
    let _ = update(
        &mut app,
        Action::LoadLayerBrowserDirectory {
            layer: "meta-demo".into(),
            root: "/layers/meta-demo".into(),
            directory: "/layers/meta-demo".into(),
            entries: vec![
                LayerBrowserEntry {
                    path: "/layers/meta-demo/.hidden".into(),
                    is_hidden: true,
                    ..LayerBrowserEntry::default()
                },
                LayerBrowserEntry {
                    path: "/layers/meta-demo/visible.bb".into(),
                    ..LayerBrowserEntry::default()
                },
            ],
        },
    );
    assert_eq!(app.layer_browser.as_ref().unwrap().entries.len(), 1);
    let _ = update(&mut app, Action::ToggleLayerBrowserHidden);
    assert_eq!(app.layer_browser.as_ref().unwrap().entries.len(), 2);
    let _ = update(&mut app, Action::BeginMetadataSearch);
    let _ = update(&mut app, Action::AppendMetadataQuery('v'));
    let _ = update(
        &mut app,
        Action::SelectLayerBrowserEntry { delta: isize::MAX },
    );
    assert_eq!(
        app.layer_browser
            .as_ref()
            .unwrap()
            .selected_entry()
            .unwrap()
            .path,
        PathBuf::from("/layers/meta-demo/visible.bb")
    );
}

#[test]
fn layer_file_right_focuses_preview_and_arrows_scroll_only_the_preview() {
    let mut app = App::new(10, 1_000);
    let _ = update(
        &mut app,
        Action::LoadLayerBrowserDirectory {
            layer: "rootfs".into(),
            root: "/build/rootfs".into(),
            directory: "/build/rootfs".into(),
            entries: vec![LayerBrowserEntry {
                path: "/build/rootfs/etc/os-release".into(),
                ..LayerBrowserEntry::default()
            }],
        },
    );
    let _ = update(
        &mut app,
        Action::LoadLayerBrowserPreview {
            path: "/build/rootfs/etc/os-release".into(),
            content: "one\ntwo\nthree".into(),
            kind: PreviewKind::Text,
            truncated: false,
        },
    );
    assert_eq!(update(&mut app, Action::LayerBrowserExpand), None);
    assert!(app.layer_browser.as_ref().unwrap().preview_focused);
    let _ = update(&mut app, Action::ScrollLayerBrowserPreview { delta: 1 });
    assert_eq!(app.layer_browser.as_ref().unwrap().preview_scroll, 1);
    let _ = update(&mut app, Action::FocusLayerBrowserTree);
    assert!(!app.layer_browser.as_ref().unwrap().preview_focused);
}
#[test]
fn layer_tree_ignores_stale_preview_and_tracks_binary_metadata() {
    let mut app = App::new(10, 1_000);
    let path = PathBuf::from("/layers/meta-demo/image.bin");
    let _ = update(
        &mut app,
        Action::LoadLayerBrowserDirectory {
            layer: "meta-demo".into(),
            root: "/layers/meta-demo".into(),
            directory: "/layers/meta-demo".into(),
            entries: vec![LayerBrowserEntry {
                path: path.clone(),
                size: Some(100_000),
                git: GitFileState::Untracked,
                ..LayerBrowserEntry::default()
            }],
        },
    );
    let _ = update(
        &mut app,
        Action::LoadLayerBrowserPreview {
            path: PathBuf::from("/layers/meta-demo/stale.bb"),
            content: "stale".into(),
            kind: PreviewKind::Text,
            truncated: false,
        },
    );
    assert!(app.layer_browser.as_ref().unwrap().preview.is_empty());
    let _ = update(
        &mut app,
        Action::LoadLayerBrowserPreview {
            path,
            content: String::new(),
            kind: PreviewKind::Binary,
            truncated: true,
        },
    );
    let browser = app.layer_browser.as_ref().unwrap();
    assert_eq!(browser.preview_kind, PreviewKind::Binary);
    assert!(browser.preview_truncated);
}
#[test]
fn layer_tree_external_editor_effect_is_typed_and_missing_selection_is_visible() {
    let mut app = App::new(10, 1_000);
    let mut browser = LayerBrowser::new("meta-demo".into(), "/layers/meta-demo".into());
    browser.entries.push(LayerBrowserEntry {
        path: "/layers/meta-demo/recipes-demo/demo/demo.bb".into(),
        ..LayerBrowserEntry::default()
    });
    app.layer_browser = Some(browser);
    assert_eq!(
        update(&mut app, Action::EditSelectedLayerBrowserFile),
        Some(Effect::OpenLayerBrowserEditor {
            layer: "meta-demo".into(),
            root: "/layers/meta-demo".into(),
            file: "recipes-demo/demo/demo.bb".into(),
        })
    );
    app.layer_browser.as_mut().unwrap().entries.clear();
    assert_eq!(update(&mut app, Action::EditSelectedLayerBrowserFile), None);
    assert_eq!(app.notification.as_deref(), Some("Select a file to edit."));
}
#[test]
fn configuration_selection_stays_in_workspace_bounds() {
    let mut app = App::new(10, 1_000);
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemuarm".into());
    app.workspace
        .variables
        .insert("DISTRO".into(), "poky".into());
    let _ = update(&mut app, Action::SelectConfigVariable { delta: 8 });
    assert_eq!(app.config_selection, 1);
    let _ = update(&mut app, Action::SelectConfigVariable { delta: -8 });
    assert_eq!(app.config_selection, 0);
}
#[test]
fn config_source_opens_single_typed_relative_operation() {
    let mut app = App::new(10, 1_000);
    app.workspace.build_dir = Some(PathBuf::from("/build"));
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemuarm".into());
    let identity = VariableIdentity {
        name: "MACHINE".into(),
        recipe: None,
    };
    app.variable_details.insert(
        identity.clone(),
        VariableDetail {
            identity,
            effective_value: Some("qemuarm".into()),
            unexpanded_value: None,
            provenance: Some("conf/local.conf:12".into()),
            operations: vec![VariableOperation {
                operation: "set".into(),
                file: Some("conf/local.conf".into()),
                line: Some(12),
                value: Some("qemuarm".into()),
            }],
            active_overrides: vec![],
        },
    );
    assert_eq!(
        update(&mut app, Action::OpenSelectedConfigSource),
        Some(Effect::OpenInEditor(PathBuf::from(
            "/build/conf/local.conf"
        )))
    );
}

#[test]
fn config_source_picker_uses_typed_operation_line_and_restores_focus() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Configuration;
    app.focus = FocusTarget::Inspector;
    app.workspace.build_dir = Some("/build".into());
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemuarm".into());
    let identity = VariableIdentity {
        name: "MACHINE".into(),
        recipe: None,
    };
    app.variable_details.insert(
        identity.clone(),
        VariableDetail {
            identity,
            effective_value: Some("qemuarm".into()),
            unexpanded_value: None,
            provenance: None,
            operations: vec![
                VariableOperation {
                    operation: "set".into(),
                    file: Some("meta/conf/bitbake.conf".into()),
                    line: Some(10),
                    value: None,
                },
                VariableOperation {
                    operation: "override".into(),
                    file: Some("conf/local.conf".into()),
                    line: Some(12),
                    value: None,
                },
            ],
            active_overrides: vec![],
        },
    );
    assert_eq!(update(&mut app, Action::OpenSelectedConfigSource), None);
    assert_eq!(app.focus, FocusTarget::Dialog);
    let Some(Dialog::ConfigSourcePicker(picker)) = app.active_dialog() else {
        panic!("source picker was not opened");
    };
    assert_eq!(picker.sources[1].operation, "override");
    assert_eq!(picker.sources[1].line, Some(12));
    let _ = update(&mut app, Action::SelectConfigSource { delta: 1 });
    assert_eq!(
        update(&mut app, Action::OpenSelectedConfigSourceChoice),
        Some(Effect::OpenInEditor("/build/conf/local.conf".into()))
    );
    assert_eq!(app.focus, FocusTarget::Navigator);
}

#[test]
fn config_source_rejects_escape_and_explains_unloaded_detail() {
    let mut app = App::new(10, 1_000);
    app.workspace.build_dir = Some("/build".into());
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemuarm".into());
    assert_eq!(update(&mut app, Action::OpenSelectedConfigSource), None);
    assert!(app.notification.as_deref().unwrap().contains("with Enter"));
    let identity = VariableIdentity {
        name: "MACHINE".into(),
        recipe: None,
    };
    app.variable_details.insert(
        identity.clone(),
        VariableDetail {
            identity,
            effective_value: Some("qemuarm".into()),
            unexpanded_value: None,
            provenance: None,
            operations: vec![VariableOperation {
                operation: "set".into(),
                file: Some("../outside.conf".into()),
                line: Some(1),
                value: None,
            }],
            active_overrides: vec![],
        },
    );
    let _ = update(&mut app, Action::OpenSelectedConfigSource);
    assert!(
        app.notification
            .as_deref()
            .unwrap()
            .contains("escapes the build directory")
    );
}
#[test]
fn metadata_search_tracks_query_and_resets_metadata_selection() {
    let mut app = App::new(10, 1_000);
    app.recipe_selection = 3;
    app.layer_selection = 2;
    app.config_selection = 1;

    let _ = update(&mut app, Action::BeginMetadataSearch);
    let _ = update(&mut app, Action::AppendMetadataQuery('q'));
    let _ = update(&mut app, Action::AppendMetadataQuery('e'));

    assert!(app.metadata_searching);
    assert_eq!(app.metadata_query, "qe");
    assert_eq!(
        (
            app.recipe_selection,
            app.layer_selection,
            app.config_selection
        ),
        (0, 0, 0)
    );

    let _ = update(&mut app, Action::BackspaceMetadataQuery);
    let _ = update(&mut app, Action::FinishMetadataSearch);
    assert_eq!(app.metadata_query, "q");
    assert!(!app.metadata_searching);
}

#[test]
fn search_clear_actions_reset_every_typed_query_without_closing_focus() {
    let mut app = App::new(10, 1_000);
    app.command_palette_open = true;
    app.command_palette_query = "palette".into();
    app.compatibility_ui.query = "capability".into();
    app.image_artifact_query = "image".into();
    app.sdk_artifact_query = "sdk".into();
    app.test_result_query = "test".into();
    app.logs.query = "log".into();
    app.package_query = "package".into();
    app.metadata_query = "metadata".into();
    app.security.query = "security".into();
    app.qa.query = "qa".into();

    for action in [
        Action::ClearCommandPaletteQuery,
        Action::ClearCompatibilityQuery,
        Action::ClearImageArtifactQuery,
        Action::ClearSdkArtifactQuery,
        Action::ClearTestResultQuery,
        Action::ClearLogQuery,
        Action::ClearPackageQuery,
        Action::ClearMetadataQuery,
        Action::Security(SecurityAction::ClearQuery),
        Action::Qa(QaAction::ClearQuery),
    ] {
        let _ = update(&mut app, action);
    }

    assert!(app.command_palette_query.is_empty());
    assert!(app.compatibility_ui.query.is_empty());
    assert!(app.image_artifact_query.is_empty());
    assert!(app.sdk_artifact_query.is_empty());
    assert!(app.test_result_query.is_empty());
    assert!(app.logs.query.is_empty());
    assert!(app.package_query.is_empty());
    assert!(app.metadata_query.is_empty());
    assert!(app.security.query.is_empty());
    assert!(app.qa.query.is_empty());
    assert!(app.command_palette_open);
}
#[test]
fn log_match_navigation_stays_within_active_search_results() {
    let mut app = App::new(10, 1_000);
    app.logs.insert(log("alpha match"));
    app.logs.insert(log("not relevant"));
    app.logs.insert(log("beta match"));
    app.logs.query = "match".into();

    let _ = update(&mut app, Action::NextLogMatch);
    assert_eq!(app.logs.selection, 1);
    assert_eq!(app.logs.match_position(), Some((2, 2)));
    assert!(!app.logs.follow);

    let _ = update(&mut app, Action::NextLogMatch);
    assert_eq!(app.logs.selection, 1);
    let _ = update(&mut app, Action::PreviousLogMatch);
    assert_eq!(app.logs.selection, 0);
    assert_eq!(app.logs.scroll_offset, 1);
}
#[test]
fn build_target_editor_requires_confirmation_before_starting() {
    let mut app = App::new(10, 1_000);
    let _ = update(&mut app, Action::BeginBuildTargetEdit);
    if let Some(Dialog::BuildTarget { editor, .. }) = app.active_dialog_mut() {
        editor.text = "target = \"core-image-minimal\"\n".into();
        editor.cursor = editor.text.len();
    }
    let effect = update(&mut app, Action::ConfirmBuildTarget);

    assert_eq!(effect, None);
    assert_eq!(
        app.active_dialog(),
        Some(&Dialog::RecipeTaskConfirmation(BuildRequest {
            targets: vec!["core-image-minimal".into()],
            task: None,
            force: false,
        }))
    );
}
#[test]
fn image_picker_selects_an_image_then_requires_build_confirmation() {
    let mut app = App::new(10, 1_000);
    let _ = update(
        &mut app,
        Action::OpenImagePicker(vec!["core-image-base".into(), "core-image-minimal".into()]),
    );
    let _ = update(&mut app, Action::SelectImage { delta: 1 });
    let _ = update(&mut app, Action::ConfirmImagePicker);
    assert_eq!(app.build.target.as_deref(), Some("core-image-minimal"));
    let _ = update(&mut app, Action::BeginCurrentImageBuild);
    assert_eq!(
        app.active_dialog(),
        Some(&Dialog::RecipeTaskConfirmation(BuildRequest {
            targets: vec!["core-image-minimal".into()],
            task: None,
            force: false,
        }))
    );
}

#[test]
fn theme_picker_applies_named_selection_immediately_and_persists_on_accept() {
    let mut app = App::new(10, 1_000);
    app.color_enabled = false;
    assert_eq!(update(&mut app, Action::OpenThemePicker), None);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::ThemePicker { .. })
    ));
    let _ = update(&mut app, Action::SelectTheme { delta: 1 });
    assert_eq!(app.theme, Theme::WhiteClassic);
    assert!(app.color_enabled);
    assert!(app.settings_dirty);
    assert!(matches!(
        update(&mut app, Action::ApplySelectedTheme),
        Some(Effect::PersistSettings)
    ));
    assert!(app.active_dialog().is_none());
}

#[test]
fn theme_picker_respects_no_color_launch_override() {
    let mut app = App::new(10, 1_000);
    app.color_enabled = false;
    app.color_forced_off = true;
    let _ = update(&mut app, Action::OpenThemePicker);
    let _ = update(&mut app, Action::SelectTheme { delta: 1 });
    assert_eq!(app.theme, Theme::WhiteClassic);
    assert!(!app.color_enabled);

    app.settings_selection = SETTINGS
        .iter()
        .position(|setting| *setting == Setting::Color)
        .unwrap();
    assert_eq!(
        update(&mut app, Action::ChangeSelectedSetting { backwards: false }),
        None
    );
    assert_eq!(
        app.notification.as_deref(),
        Some("Disabled by --no-color for this launch; the stored choice is preserved.")
    );
}

#[test]
fn theme_picker_restores_original_theme_when_closed_with_escape() {
    let mut app = App::new(10, 1_000);
    app.color_enabled = false;
    let _ = update(&mut app, Action::OpenThemePicker);
    let _ = update(&mut app, Action::SelectTheme { delta: 2 });
    assert_eq!(app.theme, Theme::MatrixGreen);
    assert!(app.color_enabled);
    let _ = update(&mut app, Action::CloseThemePicker);
    assert_eq!(app.theme, Theme::DarkPro);
    assert!(!app.color_enabled);
    assert!(!app.settings_dirty);
    assert!(app.active_dialog().is_none());
}
#[test]
fn build_completion_stays_open_until_dismissed() {
    let mut app = App::new(10, 1_000);
    app.build.target = Some("core-image-minimal".into());
    let _ = update(
        &mut app,
        Action::BuildCompleted {
            success: true,
            exit_code: Some(0),
        },
    );
    assert!(matches!(app.active_dialog(), Some(Dialog::BuildCompletion)));
    let _ = update(&mut app, Action::DismissBuildCompletion);
    assert!(app.active_dialog().is_none());
}
#[test]
fn build_options_prefill_the_current_target_and_requested_task() {
    let mut app = App::new(10, 1_000);
    app.build.target = Some("core-image-minimal".into());

    let _ = update(&mut app, Action::OpenBuildOptions);
    assert!(matches!(app.active_dialog(), Some(Dialog::BuildOptions)));
    assert_eq!(app.focus, FocusTarget::Dialog);
    let _ = update(&mut app, Action::BeginBuildTargetTask(Some("clean".into())));

    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::BuildTarget { editor, task })
            if !editor.editing
                && editor.text.contains("target = \"core-image-minimal\"")
                && editor.selected_text() == Some("core-image-minimal")
                && task.as_deref() == Some("clean")
    ));
}
#[test]
fn bounded_telemetry_history_retains_only_the_latest_valid_samples() {
    let mut app = App::new(10, 1_000);
    for sample in 0..75 {
        let telemetry = HostTelemetry {
            cpu_utilization_percent: Some(sample),
            memory_total_bytes: Some(1_000),
            memory_available_bytes: Some(750),
            disk_available_bytes: Some(8 * 1024 * 1024 * 1024),
            disk_read_bytes_per_second: Some(u64::from(sample) * 10),
            disk_write_bytes_per_second: Some(u64::from(sample) * 20),
            network_receive_bytes_per_second: Some(u64::from(sample) * 30),
            network_transmit_bytes_per_second: Some(u64::from(sample) * 40),
            ..HostTelemetry::default()
        };
        let _ = update(&mut app, Action::HostTelemetryUpdated(telemetry));
    }
    let telemetry = app.host_telemetry.clone();
    assert_eq!(app.host_telemetry, telemetry);
    let history = &app.host_telemetry_history;
    assert_eq!(history.cpu_percent.len(), 60);
    assert_eq!(history.cpu_percent.front(), Some(&15));
    assert_eq!(history.cpu_percent.back(), Some(&74));
    assert_eq!(history.memory_percent.len(), 60);
    assert!(history.memory_percent.iter().all(|sample| *sample == 25));
    assert_eq!(history.disk_read_bytes_per_second.front(), Some(&150));
    assert_eq!(history.disk_read_bytes_per_second.back(), Some(&740));
    assert_eq!(history.disk_write_bytes_per_second.len(), 60);
    assert_eq!(history.network_receive_bytes_per_second.len(), 60);
    assert_eq!(history.network_transmit_bytes_per_second.len(), 60);

    let _ = update(
        &mut app,
        Action::HostTelemetryUpdated(HostTelemetry {
            cpu_utilization_percent: None,
            memory_total_bytes: Some(100),
            memory_available_bytes: Some(101),
            ..HostTelemetry::default()
        }),
    );
    let history = &app.host_telemetry_history;
    assert_eq!(history.cpu_percent.len(), 60);
    assert_eq!(history.memory_percent.len(), 60);
    assert_eq!(history.disk_read_bytes_per_second.len(), 60);
    assert_eq!(history.network_receive_bytes_per_second.len(), 60);

    let _ = update(
        &mut app,
        Action::HostTelemetryUpdated(HostTelemetry {
            memory_total_bytes: Some(u64::MAX),
            memory_available_bytes: Some(u64::MAX / 2),
            ..HostTelemetry::default()
        }),
    );
    assert_eq!(app.host_telemetry_history.memory_percent.back(), Some(&50));
}

#[test]
fn coexistence_diagnostic_parses_parallelism_without_mutating_workspace() {
    let mut workspace = Workspace::default();
    workspace
        .variables
        .insert("BB_NUMBER_THREADS".into(), "24".into());
    workspace
        .variables
        .insert("PARALLEL_MAKE".into(), "-l 8 --jobs=20".into());
    let original = workspace.clone();
    let diagnostic = bitbake_coexistence_diagnostic(
        &workspace,
        &HostTelemetry {
            logical_cpu_count: Some(8),
            load_average_milli: Some([16_001, 8_000, 4_000]),
            ..HostTelemetry::default()
        },
    );
    assert_eq!(
        diagnostic.pressure,
        BitBakeCoexistencePressure::Oversubscribed
    );
    assert_eq!(diagnostic.bitbake_threads, Some(24));
    assert_eq!(diagnostic.parallel_make_jobs, Some(20));
    assert_eq!(diagnostic.review_example_jobs, Some(7));
    assert_eq!(workspace, original);
}
