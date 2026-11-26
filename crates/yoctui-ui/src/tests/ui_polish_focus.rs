use super::*;

fn fixture(mode: usize) -> App {
    let mut app = App::new(20, 2000);
    app.screen = Screen::Recipes;
    app.workspace.recipes = vec![Recipe {
        name: "r000".into(),
        ..Default::default()
    }];
    match mode {
        1 => app.color_enabled = false,
        2 => app.preferences.symbols = SymbolPreference::Ascii,
        3 => app.theme = Theme::HighContrast,
        _ => {}
    }
    app
}

fn recipe_cell(app: &App, width: u16, height: u16) -> ratatui::buffer::Cell {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| package_render::recipes(frame, app, frame.area()))
        .unwrap();
    let cells = &terminal.backend().buffer().content;
    let name = "r000";
    let start = cells
        .windows(name.len())
        .enumerate()
        .find(|(index, row)| {
            *index % usize::from(width)
                < if width >= 110 {
                    usize::from(width) * 42 / 100
                } else {
                    usize::from(width)
                }
                && row.iter().map(|cell| cell.symbol()).collect::<String>() == name
        })
        .map(|(index, _)| index)
        .expect("selected recipe stays visible");
    cells[start].clone()
}

#[test]
fn ui_polish_focus_inactive_selection_remains_visible_without_active_highlight() {
    for mode in 0..4 {
        for (width, height) in [(80, 24), (100, 30), (160, 48)] {
            let mut app = fixture(mode);
            app.focus = FocusTarget::Workspace;
            let active = recipe_cell(&app, width, height);
            app.focus = FocusTarget::Navigator;
            let inactive = recipe_cell(&app, width, height);
            assert_ne!(
                active.style(),
                inactive.style(),
                "mode={mode} {width}x{height}: selection must distinguish its input owner"
            );
            assert!(
                inactive.modifier.contains(Modifier::UNDERLINED),
                "inactive selection must retain a visible non-color cue"
            );
            assert_eq!(app.recipe_selection, 0);
            assert_eq!(app.focus, FocusTarget::Navigator);
        }
    }
}

#[test]
fn ui_polish_focus_modal_owns_highlight_while_underlying_recipe_is_inactive() {
    for mode in 0..4 {
        let mut app = fixture(mode);
        app.focus = FocusTarget::Workspace;
        let _ = update(
            &mut app,
            Action::EnvironmentSetup(yoctui_model::EnvironmentSetupAction::Open { browse: false }),
        );
        assert!(app.active_dialog().is_some());
        assert_eq!(app.input_owner(), FocusTarget::Dialog);
        let row = recipe_cell(&app, 100, 30);
        assert!(row.modifier.contains(Modifier::UNDERLINED));
        assert!(!row.modifier.contains(Modifier::REVERSED));
        let Some(Dialog::EnvironmentSetup(setup)) = app.active_dialog() else {
            panic!("environment dialog missing")
        };
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        terminal
            .draw(|frame| {
                environment_setup::environment_setup_popup(frame, &app, setup, frame.area())
            })
            .unwrap();
        let cells = &terminal.backend().buffer().content;
        let palette = ThemePalette::for_app(&app);
        if palette.attribute_only {
            assert!(
                cells
                    .iter()
                    .any(|cell| cell.modifier.contains(Modifier::REVERSED))
            );
        } else {
            assert!(
                cells
                    .iter()
                    .any(|cell| cell.bg == palette.selection_background)
            );
        }
    }
}

#[test]
fn ui_polish_focus_navigator_border_and_retained_row_follow_the_input_owner() {
    for mode in 0..4 {
        for (width, height) in [(80, 24), (100, 30), (160, 48)] {
            let mut app = fixture(mode);
            let mut styles = Vec::new();
            for owner in [FocusTarget::Navigator, FocusTarget::Workspace] {
                app.focus = owner;
                let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
                terminal
                    .draw(|frame| navigator(frame, &app, frame.area(), None))
                    .unwrap();
                let buffer = terminal.backend().buffer();
                let name = "Dashboard";
                let index = buffer
                    .content
                    .windows(name.len())
                    .position(|cells| cells.iter().map(|c| c.symbol()).collect::<String>() == name)
                    .expect("navigator selected destination remains visible");
                let cell = &buffer.content[index];
                if owner == FocusTarget::Workspace {
                    assert!(cell.modifier.contains(Modifier::UNDERLINED));
                }
                styles.push(cell.style());
                if app.color_enabled {
                    let palette = ThemePalette::for_app(&app);
                    assert_eq!(
                        buffer[(0, 0)].fg,
                        if owner == FocusTarget::Navigator {
                            palette.focused_border
                        } else {
                            palette.inactive_border
                        }
                    );
                }
            }
            assert_ne!(styles[0], styles[1]);
        }
    }
}

#[test]
fn ui_polish_colors_status_words_remain_readable_in_every_render_mode() {
    for mode in 0..4 {
        let app = fixture(mode);
        let palette = ThemePalette::for_app(&app);
        let lines = [
            (StatusTone::Success, "Succeeded"),
            (StatusTone::Warning, "Attention"),
            (StatusTone::Error, "Failed"),
        ]
        .map(|(tone, text)| {
            Line::from(status_label(tone, text, status_tone_style(&palette, tone)))
        });
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal
            .draw(|frame| frame.render_widget(Paragraph::new(lines.to_vec()), frame.area()))
            .unwrap();
        let text = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        for label in ["Succeeded", "Attention", "Failed"] {
            assert!(text.contains(label));
        }
    }
}

#[test]
fn ui_polish_focus_preferences_and_palette_suspend_the_stored_pane_focus() {
    let mut app = fixture(0);
    app.focus = FocusTarget::Workspace;
    app.keymap_preferences_ui.open = true;
    assert_eq!(app.input_owner(), FocusTarget::Dialog);
    assert!(
        recipe_cell(&app, 100, 30)
            .modifier
            .contains(Modifier::UNDERLINED)
    );
    app.keymap_preferences_ui.open = false;
    app.command_palette_open = true;
    assert_eq!(app.input_owner(), FocusTarget::CommandPalette);
    assert!(
        recipe_cell(&app, 100, 30)
            .modifier
            .contains(Modifier::UNDERLINED)
    );
    app.command_palette_open = false;
    assert_eq!(app.input_owner(), FocusTarget::Workspace);
    assert!(
        !recipe_cell(&app, 100, 30)
            .modifier
            .contains(Modifier::UNDERLINED)
    );
}
