use super::*;

#[test]
fn command_palette_global_shortcut_is_typed() {
    assert_eq!(key_action(Input::CtrlP), Some(Action::OpenCommandPalette));
    assert_eq!(focus_action(FocusTarget::CommandPalette, Input::Tab), None);
}

#[test]
fn command_palette_pages_move_ten_results() {
    assert_eq!(
        command_palette_navigation_action(Input::PageUp),
        Some(Action::SelectCommandPalette { delta: -10 })
    );
    assert_eq!(
        command_palette_navigation_action(Input::PageDown),
        Some(Action::SelectCommandPalette { delta: 10 })
    );
}

#[test]
fn global_search_target_shortcuts_are_scoped_and_preserve_regex_text() {
    let mut app = App::new(32, 4096);
    yoctui_model::update(&mut app, Action::OpenGlobalSearch);
    for character in "foo[ñ~N]\\.dts$".chars() {
        yoctui_model::update(&mut app, Action::AppendCommandPaletteQuery(character));
    }
    let query = app.command_palette_query.clone();
    for key in [Input::Tab, Input::BackTab, Input::Alt('n'), Input::Alt('N')] {
        let before = app.global_search_target;
        let generation = app.global_search_generation;
        let action = global_search_target_action(&app, key).unwrap();
        yoctui_model::update(&mut app, action);
        assert_ne!(app.global_search_target, before);
        assert_eq!(app.command_palette_query, query);
        assert!(app.global_search_generation > generation);
    }
    for character in ['n', 'N', 'ñ', '~', 'מ'] {
        assert_eq!(
            global_search_target_action(&app, Input::Char(character)),
            None
        );
    }
    app.command_palette_mode = yoctui_model::CommandPaletteMode::Commands;
    assert_eq!(global_search_target_action(&app, Input::Tab), None);
    app.command_palette_mode = yoctui_model::CommandPaletteMode::GlobalRegexSearch;
    app.command_palette_open = false;
    assert_eq!(global_search_target_action(&app, Input::Alt('n')), None);
    app.command_palette_open = true;
    app.onboarding.open = true;
    assert_eq!(global_search_target_action(&app, Input::Tab), None);
}
