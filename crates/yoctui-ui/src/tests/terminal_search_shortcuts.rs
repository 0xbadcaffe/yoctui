use super::*;
use crate::footer::footer_context_items;

#[test]
fn terminal_tree_footer_distinguishes_global_search_from_console_search() {
    let mut app = super::demo_terminal_pane_binding::terminal_binding_fixture();
    app.terminal.browser_open = true;
    let footer = footer_shortcuts(&app);
    assert!(footer.contains("/ global search"), "{footer}");
    assert!(!footer.contains("Alt+/ search"), "{footer}");
    for width in [80, 100, 160] {
        let items = footer_context_items(&app, width);
        assert!(
            items.contains(&"/ global search".to_owned()),
            "{width}: {items:?}"
        );
    }
}

#[test]
fn terminal_search_footer_advertises_one_writer_search_and_working_navigation() {
    let mut app = super::demo_terminal_pane_binding::terminal_binding_fixture();
    assert!(app.selected_terminal_is_writer());
    let footer = footer_shortcuts(&app);
    assert!(footer.contains("Alt+/ find output"), "{footer}");
    assert!(!footer.contains("Ctrl+B /"), "{footer}");
    assert!(footer.contains("Alt+o sessions"), "{footer}");
    for width in [80, 100, 160] {
        let items = footer_context_items(&app, width);
        assert!(
            items.iter().any(|item| item == "Alt+/ find output"),
            "{width}: {items:?}"
        );
        let rail = footer_rail_shortcuts(&app, width);
        for misleading in ["Tab Focus", "F1 Help", "F2 Tasks", "F12 Menu", "q Quit"] {
            assert!(!rail.contains(misleading), "{width}: {rail}");
        }
        assert!(rail.contains("Alt+o sessions"), "{width}: {rail}");
        assert!(rail.contains("Ctrl+B ? shortcuts"), "{width}: {rail}");
    }
    app.daemon.pty_details.last_mut().unwrap().writer = None;
    assert!(
        footer_shortcuts(&app)
            .split(" | ")
            .any(|item| item == "/ find output")
    );
}

#[test]
fn terminal_search_footer_exposes_history_navigation_and_clear() {
    let mut app = super::demo_terminal_pane_binding::terminal_binding_fixture();
    app.terminal.mode = yoctui_model::TerminalWorkbenchMode::Search;
    let items = footer_context_items(&app, 160);
    assert!(items.contains(&"PgUp/PgDn history".to_owned()), "{items:?}");
    assert!(items.contains(&"Ctrl+U clear".to_owned()), "{items:?}");
    assert!(items.contains(&"Enter/Esc done".to_owned()), "{items:?}");
}

#[test]
fn terminal_modal_footer_only_advertises_modal_controls() {
    let mut app = super::demo_terminal_pane_binding::terminal_binding_fixture();
    for (mode, expected) in [
        (
            yoctui_model::TerminalWorkbenchMode::Help,
            "Esc close shortcuts",
        ),
        (yoctui_model::TerminalWorkbenchMode::Rename, "Enter save"),
        (
            yoctui_model::TerminalWorkbenchMode::PasteReview,
            "Enter send paste",
        ),
        (
            yoctui_model::TerminalWorkbenchMode::KillConfirmation,
            "Enter stop session",
        ),
        (yoctui_model::TerminalWorkbenchMode::Copy, "Enter copy"),
    ] {
        app.terminal.mode = mode;
        let footer = footer_rail_shortcuts(&app, 160);
        assert!(footer.contains(expected), "{mode:?}: {footer}");
        assert!(!footer.contains("q Quit"), "{mode:?}: {footer}");
        assert!(!footer.contains("F12 Menu"), "{mode:?}: {footer}");
    }
}
