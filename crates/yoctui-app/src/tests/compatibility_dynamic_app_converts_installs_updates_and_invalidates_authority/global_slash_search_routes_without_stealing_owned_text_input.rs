use super::*;

#[test]
fn global_slash_search_routes_without_stealing_owned_text_input() {
    let mut app = yoctui_model::App::new(16, 4096);
    app.screen = yoctui_model::Screen::Recipes;
    assert_eq!(
        global_search_action(&app, Input::Char('/')),
        Some(Action::OpenGlobalSearch)
    );

    app.metadata_searching = true;
    assert_eq!(global_search_action(&app, Input::Char('/')), None);
    app.metadata_searching = false;
    app.screen = yoctui_model::Screen::Dashboard;
    app.logs.searching = true;
    assert_eq!(
        global_search_action(&app, Input::Char('/')),
        Some(Action::OpenGlobalSearch),
        "a retained search owned by another screen must not suppress global search"
    );
    app.logs.searching = false;
    app.screen = yoctui_model::Screen::TerminalSessions;
    assert_eq!(global_search_action(&app, Input::Char('/')), None);

    app.screen = yoctui_model::Screen::Dashboard;
    app.dialogs
        .push_back(yoctui_model::Dialog::QuitConfirmation);
    assert_eq!(global_search_action(&app, Input::Char('/')), None);
    assert_eq!(global_search_action(&app, Input::Char('x')), None);
}
