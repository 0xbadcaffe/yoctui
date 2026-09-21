use super::*;

#[test]
fn ux_menu_groups_context_availability_prefix_and_focus_are_typed_and_bounded() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Tasks;
    let _ = update(&mut app, Action::OpenApplicationMenu);
    assert!(app.menu.is_open());
    assert_eq!(app.focus, FocusTarget::Dialog);
    assert_eq!(
        ApplicationMenuGroup::ALL.map(ApplicationMenuGroup::label),
        ["Workspace", "Build", "Navigate", "View", "Tools", "Help"]
    );
    assert_eq!(app.active_menu_items()[0].label, "Edit BBMASK");
    assert_eq!(
        app.active_menu_items()[0].disabled_reason.as_deref(),
        Some("Load a Yocto workspace first")
    );

    let _ = update(&mut app, Action::SelectMenuGroup { delta: 2 });
    for character in "open layers".chars() {
        let _ = update(&mut app, Action::AppendMenuPrefix(character));
    }
    assert_eq!(app.selected_menu_item().unwrap().label, "Open Layers");
    for _ in 0..64 {
        let _ = update(&mut app, Action::AppendMenuPrefix('x'));
    }
    assert_eq!(app.menu.typed_prefix.chars().count(), MAX_MENU_PREFIX_CHARS);
    let _ = update(&mut app, Action::SelectMenuItem { delta: 999 });
    assert_eq!(
        app.menu.item_selection,
        app.active_menu_items().len().saturating_sub(1)
    );

    let _ = update(&mut app, Action::CloseMenu);
    assert_eq!(app.focus, FocusTarget::Navigator);
    app.screen = Screen::Recipes;
    app.workspace.build_dir = Some(PathBuf::from("/work/build"));
    let _ = update(&mut app, Action::OpenContextMenu);
    let build = app
        .active_menu_items()
        .into_iter()
        .find(|item| item.action_id.as_str() == "recipes.build")
        .unwrap();
    assert_eq!(
        build.disabled_reason.as_deref(),
        Some("Select a recipe first.")
    );
    assert_eq!(build.safety, OperatorActionSafety::ConfirmationRequired);
}
