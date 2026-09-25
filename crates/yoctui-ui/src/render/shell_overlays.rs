fn render_shell_overlays(frame: &mut Frame, app: &App, area: Rect) -> bool {
    if app.menu.is_open() {
        if let Some(Dialog::RecipeEditor(editor)) = app.active_dialog() {
            recipe_editor(frame, app, editor, area);
        }
        menu_overlay(frame, app, area);
        return true;
    } else if app.onboarding.open {
        onboarding_overlay(frame, app, area);
        return true;
    } else if app.keymap_preferences_ui.open {
        keymap_preferences_overlay(frame, app, area);
        return true;
    } else if app.command_palette_open {
        command_palette(frame, app, area);
        return true;
    } else if app.screen == Screen::RawMode
        && let Some(picker) = app.raw_mode.recipe_picker.as_ref()
        && let Some(form) = app.raw_mode.form.as_ref()
    {
        raw_command_form_dialog(frame, app, form, area);
        raw_recipe_picker_dialog(frame, app, picker, area);
        return true;
    } else if app.screen == Screen::RawMode
        && let Some(form) = app
            .raw_mode
            .form
            .as_ref()
            .filter(|_| app.raw_mode.view == yoctui_model::RawModeView::Form)
    {
        raw_command_form_dialog(frame, app, form, area);
        return true;
    } else if app.screen == Screen::RawMode
        && let Some(preview) = app
            .raw_mode
            .preview
            .as_ref()
            .filter(|_| app.raw_mode.view == yoctui_model::RawModeView::Preview)
    {
        let popup = dialog_popup_rect(area, 110, 30);
        clear_popup(frame, app, popup);
        render_raw_execution_preview(frame, preview, popup);
        return true;
    }
    false
}
