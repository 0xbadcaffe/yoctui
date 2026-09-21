use super::*;

#[test]
fn function_shortcut_catalog_is_complete_unique_and_truthful() {
    assert_eq!(FUNCTION_SHORTCUTS.len(), 10);
    let keys = FUNCTION_SHORTCUTS
        .iter()
        .map(|shortcut| shortcut.key)
        .collect::<HashSet<_>>();
    let labels = FUNCTION_SHORTCUTS
        .iter()
        .map(|shortcut| shortcut.key_label)
        .collect::<HashSet<_>>();
    assert_eq!(keys.len(), FUNCTION_SHORTCUTS.len());
    assert_eq!(labels.len(), FUNCTION_SHORTCUTS.len());
    for shortcut in FUNCTION_SHORTCUTS {
        assert!(!shortcut.action_label.is_empty());
        assert_eq!(
            function_shortcut_action(shortcut.key),
            match shortcut.route {
                FunctionShortcutRoute::Open(screen) => Action::Open(screen),
                FunctionShortcutRoute::CommandPalette => Action::OpenCommandPalette,
                FunctionShortcutRoute::ApplicationMenu => Action::OpenApplicationMenu,
            }
        );
    }
}
