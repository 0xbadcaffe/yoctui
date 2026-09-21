use super::*;

#[test]
fn keymap_function_key_catalog_maps_every_named_route() {
    let inputs = [
        Input::F1,
        Input::F2,
        Input::F3,
        Input::F4,
        Input::F5,
        Input::F6,
        Input::F7,
        Input::F8,
        Input::F9,
        Input::F10,
    ];
    let labels = [
        ("F1", "Help"),
        ("F2", "Tasks"),
        ("F3", "History"),
        ("F4", "Dashboard"),
        ("F5", "Logs"),
        ("F6", "Layers"),
        ("F7", "Recipes"),
        ("F8", "Images"),
        ("F9", "Commands"),
        ("F10", "Menu"),
    ];
    for ((input, shortcut), label) in inputs
        .into_iter()
        .zip(yoctui_model::FUNCTION_SHORTCUTS)
        .zip(labels)
    {
        assert_eq!((shortcut.key_label, shortcut.action_label), label);
        assert_eq!(
            key_action(input),
            Some(yoctui_model::function_shortcut_action(shortcut.key))
        );
    }
}
