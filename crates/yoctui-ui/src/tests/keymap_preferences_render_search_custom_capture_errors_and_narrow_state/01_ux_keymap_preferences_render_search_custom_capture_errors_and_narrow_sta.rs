use super::*;

#[test]
fn ux_keymap_preferences_render_search_custom_capture_errors_and_narrow_state() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Settings;
    app.install_keymap(yoctui_model::KeymapPreferences {
        schema_version: yoctui_model::KEYMAP_SCHEMA_VERSION,
        overrides: vec![yoctui_model::KeymapOverride {
            action_id: "navigate.logs".into(),
            scope: yoctui_model::KeymapScope::Global,
            sequences: vec!["z".parse().unwrap()],
        }],
    })
    .unwrap();
    app.keymap_preferences_ui.open = true;
    app.keymap_preferences_ui.query = "navigate.logs".into();
    app.focus = FocusTarget::Dialog;

    let output = rendered_text(&app, 120, 35);
    assert!(output.contains("Keybinding preferences"), "{output}");
    assert!(output.contains("Effective keymap"), "{output}");
    assert!(output.contains("Open Logs"), "{output}");
    assert!(output.contains("Global"), "{output}");
    assert!(output.contains("custom"), "{output}");
    assert!(output.contains("navigate.logs"), "{output}");
    assert!(output.contains("z"), "{output}");
    assert!(output.contains("capture"), "{output}");
    assert!(output.contains("export"), "{output}");

    app.keymap_preferences_ui.capture = Some(yoctui_model::KeymapCaptureState {
        strokes: vec![
            yoctui_model::KeyStroke::Char('g'),
            yoctui_model::KeyStroke::Char('l'),
        ],
    });
    let output = rendered_text(&app, 80, 24);
    assert!(output.contains("Pending capture: g l"), "{output}");
    assert!(output.contains("Ctrl+S"), "{output}");

    app.keymap_preferences_ui.capture = None;
    app.keymap_preferences_ui.validation_error =
        Some("keymap collision in Global: exact test reason".into());
    app.color_enabled = false;
    let output = rendered_text(&app, 80, 24);
    assert!(output.contains("Conflict/disabled"), "{output}");
    assert!(output.contains("exact test reason"), "{output}");
}

#[test]
fn ux_onboarding_renders_real_typed_workflow_states_responsively_and_accessibly() {
    let mut app = App::new_unconfigured(10, 1_000);
    let _ = update(&mut app, Action::OpenOnboarding);
    for (width, height) in [(160, 40), (100, 30), (80, 24)] {
        let output = rendered_text(&app, width, height);
        assert!(
            output.contains("Yoctui workflow guide"),
            "{width}x{height}: {output}"
        );
        assert!(
            output.contains("opening/resuming starts no build or process"),
            "{output}"
        );
        assert!(output.contains("Verify environment"), "{output}");
        assert!(output.contains("CURRENT"), "{output}");
        assert!(output.contains("BLOCKED"), "{output}");
        assert!(output.contains("Enter open"), "{output}");
        assert!(output.contains("Esc dismiss"), "{output}");
    }

    app.onboarding
        .progress
        .completed
        .insert(yoctui_model::OnboardingStep::Environment);
    app.color_enabled = false;
    app.reduced_motion = true;
    let stale = rendered_text(&app, 100, 30);
    assert!(stale.contains("[!] Verify environment"), "{stale}");
    assert!(stale.contains("STALE"), "{stale}");

    app.onboarding.progress.completed.clear();
    app.onboarding
        .progress
        .skipped
        .insert(yoctui_model::OnboardingStep::Environment);
    app.onboarding.progress.current = yoctui_model::OnboardingStep::Target;
    app.onboarding.selected = yoctui_model::OnboardingStep::Target;
    let unavailable = rendered_text(&app, 80, 24);
    assert!(unavailable.contains("SKIPPED"), "{unavailable}");
    assert!(unavailable.contains("UNAVAILABLE"), "{unavailable}");
    assert!(unavailable.contains("[?] Select a target"), "{unavailable}");
}

#[test]
fn ux_menu_renders_groups_context_disabled_safety_and_accessible_responsive_states() {
    let mut app = App::new(10, 1_000);
    let _ = update(&mut app, Action::OpenApplicationMenu);
    for (width, height) in [(160, 40), (100, 30), (80, 24)] {
        let output = rendered_text(&app, width, height);
        assert!(
            output.contains("Application menu"),
            "{width}x{height}: {output}"
        );
        assert!(output.contains("Workspace"), "{output}");
        assert!(output.contains("Build"), "{output}");
        assert!(output.contains("Navigate"), "{output}");
        assert!(output.contains("View"), "{output}");
        assert!(output.contains("Tools"), "{output}");
        assert!(output.contains("Help"), "{output}");
        assert!(output.contains("Edit BBMASK"), "{output}");
        assert!(output.contains("Load a Yocto workspace first"), "{output}");
        assert!(output.contains("Esc/F12 close"), "{output}");
    }

    let _ = update(&mut app, Action::CloseMenu);
    app.screen = Screen::Recipes;
    app.workspace.build_dir = Some("/work/build".into());
    app.reduced_motion = true;
    app.color_enabled = false;
    let _ = update(&mut app, Action::OpenContextMenu);
    let _ = update(&mut app, Action::SelectMenuItem { delta: 2 });
    let output = rendered_text(&app, 100, 30);
    assert!(output.contains("Recipes actions"), "{output}");
    assert!(output.contains("Build selected recipe"), "{output}");
    assert!(output.contains("confirm"), "{output}");
    assert!(output.contains("Select a recipe first."), "{output}");
    assert!(output.contains("> Build selected reci"), "{output}");

    let last_label = app
        .active_menu_items()
        .last()
        .expect("recipe context menu item")
        .label
        .to_owned();
    let _ = update(&mut app, Action::SelectMenuItem { delta: isize::MAX });
    let bottom = rendered_text(&app, 80, 24);
    let selected_prefix = format!("> {}", last_label.chars().take(12).collect::<String>());
    assert!(
        bottom.contains(&selected_prefix),
        "last menu row {last_label:?} lost its highlight: {bottom}"
    );
}

#[test]
fn ux_viewport_chrome_reports_position_and_available_directions() {
    let mut navigator_app = App::new(10, 1_000);
    navigator_app.focus = FocusTarget::Navigator;
    let total_navigation_rows = navigator_app.navigator_visible_row_count();
    let first_navigation_row = navigator_app.navigator_visual_row() + 1;
    let top = rendered_text(&navigator_app, 80, 24);
    assert!(
        top.contains(&format!(
            "Navigator · {first_navigation_row}/{total_navigation_rows} ↓"
        )),
        "{top}"
    );

    navigator_app.navigator_selection = 24;
    let bottom = rendered_text(&navigator_app, 80, 24);
    assert!(
        bottom.contains(&format!(
            "Navigator · {total_navigation_rows}/{total_navigation_rows} ↑"
        )),
        "{bottom}"
    );
    navigator_app.preferences.symbols = SymbolPreference::Ascii;
    let ascii = rendered_text(&navigator_app, 80, 24);
    assert!(
        ascii.contains(&format!(
            "Navigator · {total_navigation_rows}/{total_navigation_rows} ^"
        )),
        "{ascii}"
    );

    let mut palette_app = App::new(10, 1_000);
    palette_app.command_palette_open = true;
    palette_app.focus = FocusTarget::CommandPalette;
    let command_count = palette_app.command_palette_commands().len();
    palette_app.command_palette_selection = command_count.saturating_sub(1);
    let palette = rendered_text(&palette_app, 80, 24);
    assert!(
        palette.contains(&format!(
            "Commands · {command_count}/{command_count} · ↑ · rows"
        )),
        "{palette}"
    );

    palette_app.command_palette_query = "Open Settings".into();
    palette_app.command_palette_selection = 0;
    let fitting = rendered_text(&palette_app, 80, 24);
    assert!(fitting.contains("Commands · 1 match"), "{fitting}");
    assert!(!fitting.contains("Commands · 1/1 · rows"), "{fitting}");

    let mut menu_app = App::new(10, 1_000);
    menu_app.screen = Screen::Recipes;
    menu_app.workspace.build_dir = Some("/work/build".into());
    let _ = update(&mut menu_app, Action::OpenContextMenu);
    let menu_count = menu_app.active_menu_items().len();
    let _ = update(&mut menu_app, Action::SelectMenuItem { delta: isize::MAX });
    let menu = rendered_text(&menu_app, 80, 24);
    assert!(
        menu.contains(&format!(
            "Catalog actions · {menu_count}/{menu_count} · ↑ · rows"
        )),
        "{menu}"
    );
}

#[test]
fn build_environment_workspace_renders_disconnected_state_and_unlock_rule() {
    let app = App::new_unconfigured(10, 1_000);
    let output = rendered_text(&app, 100, 30);
    assert!(output.contains("Build environment"));
    assert!(output.contains("not configured"));
    assert!(output.contains("available images"));
    assert!(output.contains("verification"));
}

#[test]
fn project_profile_renders_team_intent_and_explicit_resolution_states() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::BuildEnvironment;
    app.focus = FocusTarget::Workspace;
    app.project_profile = yoctui_model::ProjectProfileState::Loaded(yoctui_model::ProjectProfile {
        schema_version: yoctui_model::PROJECT_PROFILE_SCHEMA_VERSION,
        favorites: yoctui_model::ProjectFavorites {
            recipes: vec!["busybox".into(), "removed-recipe".into()],
            images: Vec::new(),
            layers: Vec::new(),
        },
        build_presets: Vec::new(),
        workflows: Vec::new(),
    });
    app.workspace.recipes.push(yoctui_model::Recipe {
        name: "busybox".into(),
        ..yoctui_model::Recipe::default()
    });
    for (width, height) in [(160, 40), (100, 30), (80, 24)] {
        let output = rendered_text(&app, width, height);
        assert!(output.contains("Project profile: team intent"), "{output}");
        assert!(output.contains("Recipe favorite: busybox"), "{output}");
        if height >= 30 {
            assert!(
                output.contains("STALE: not reported by BitBake"),
                "{output}"
            );
        }
        assert!(output.contains("p preview/open"), "{output}");
    }

    app.project_profile =
        yoctui_model::ProjectProfileState::Invalid("unsupported schema version 9".into());
    let output = rendered_text(&app, 100, 30);
    assert!(output.contains("Project profile: invalid"), "{output}");
    assert!(output.contains("unsupported schema version 9"), "{output}");
}

#[test]
fn build_environment_form_renders_selected_typed_fields() {
    let mut app = App::new_unconfigured(10, 1_000);
    let _ = update(&mut app, Action::BeginBuildEnvironmentEdit);
    let output = rendered_text(&app, 120, 36);
    assert!(output.contains("Edit profile"));
    assert!(output.contains("source:"));
    assert!(output.contains("script:"));
}

#[test]
fn build_environment_editor_renders_large_vi_style_popup() {
    let mut app = App::new_unconfigured(10, 1_000);
    let mut editor =
        yoctui_model::PopupEditor::new("source = \"/home/poky\"\nbuild = \"/home/build\"".into());
    editor.select_toml_value("source").unwrap();
    app.dialogs
        .push_back(Dialog::BuildEnvironmentEditor(editor));
    for (width, height) in [(80, 24), (120, 40)] {
        let output = rendered_text(&app, width, height);
        assert!(output.contains("Build environment.toml"));
        assert!(output.contains("NORMAL"));
        assert!(output.contains("⟦/home/poky⟧▏"));
        assert!(output.contains("Home/End line"));
        assert!(output.contains("Ctrl+V paste"));
    }
}

#[test]
fn build_environment_clone_editor_uses_shared_popup_at_minimum_size() {
    let mut app = App::new_unconfigured(10, 1_000);
    let _ = update(&mut app, Action::OpenBuildEnvironmentCloneEditor);
    let output = rendered_text(&app, 80, 24);
    assert!(output.contains("Clone Poky.toml"), "{output}");
    assert!(output.contains("repository = \"⟦▏⟧\""), "{output}");
    assert!(output.contains("e change value"), "{output}");
    assert!(output.contains("Ctrl+C copy"), "{output}");
}

#[test]
fn active_task_indicator_uses_braille_motion_and_accessible_fallbacks() {
    let mut app = App::new(10, 1_000);
    app.animation_frame = 0;
    let first = task_activity(&app, None);
    app.animation_frame = 1;
    assert_ne!(task_activity(&app, None), first);
    assert!(
        throbber_widgets_tui::BRAILLE_EIGHT_DOUBLE
            .symbols
            .contains(&first.as_str())
    );
    assert_eq!(task_activity(&app, Some(0)), "");

    app.reduced_motion = true;
    assert_eq!(task_activity(&app, None), "⣿");
    app.preferences.symbols = SymbolPreference::Ascii;
    assert_eq!(task_activity(&app, None), "#");
}

#[test]
fn ux_throbber_uses_reviewed_symbols_with_ascii_motion_and_terminal_fallbacks() {
    let running = yoctui_model::ActivityProjection::new(
        yoctui_model::ActivityLifecycle::Running,
        5,
        yoctui_model::AnimationSpeed::Fast,
        false,
    );
    assert_eq!(
        activity_symbol(running, true),
        throbber_widgets_tui::BRAILLE_EIGHT_DOUBLE.symbols
            [running.phase.expect("running activity phase") % 8]
    );
    assert_eq!(
        activity_symbol(running, false),
        throbber_widgets_tui::ASCII.symbols[1]
    );

    let reduced = yoctui_model::ActivityProjection::new(
        yoctui_model::ActivityLifecycle::Waiting,
        u64::MAX,
        yoctui_model::AnimationSpeed::Fast,
        true,
    );
    assert_eq!(activity_symbol(reduced, true), "");
    assert_eq!(reduced.text(), "waiting");

    for (lifecycle, marker) in [
        (yoctui_model::ActivityLifecycle::Succeeded, "+"),
        (yoctui_model::ActivityLifecycle::Failed, "x"),
        (yoctui_model::ActivityLifecycle::Cancelled, "#"),
    ] {
        let terminal = yoctui_model::ActivityProjection::new(
            lifecycle,
            u64::MAX,
            yoctui_model::AnimationSpeed::Fast,
            false,
        );
        assert_eq!(terminal.phase, None);
        assert_eq!(activity_symbol(terminal, false), marker);
    }

    let mut app = App::new(10, 1_000);
    app.color_enabled = false;
    app.reduced_motion = true;
    app.build.status = BuildStatus::Running;
    app.build.completed = 3;
    app.screen = Screen::Tasks;
    let output = rendered_text(&app, 100, 30);
    assert!(output.contains("progress unknown ⣿  3/—"), "{output}");
    assert!(!output.contains("0%"), "{output}");
}
