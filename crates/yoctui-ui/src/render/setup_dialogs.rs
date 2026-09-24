fn render_setup_dialogs(frame: &mut Frame, app: &App, area: Rect) -> bool {
    if let Some(Dialog::EnvironmentSetup(setup)) = app.active_dialog() {
        environment_setup::environment_setup_popup(frame, app, setup, area);
        return true;
    } else if let Some(Dialog::BuildEnvironmentCloneEditor(editor)) = app.active_dialog() {
        build_environment_clone_editor(frame, app, editor, area);
        return true;
    } else if let Some(Dialog::BuildEnvironmentCloneReview(plan)) = app.active_dialog() {
        build_environment_clone_review(frame, app, plan, area);
        return true;
    } else if let Some(Dialog::BuildEnvironmentEditor(editor)) = app.active_dialog() {
        build_environment_editor(frame, app, editor, area);
        return true;
    } else if let Some(Dialog::ThemePicker { selection, .. }) = app.active_dialog() {
        theme_picker(frame, app, *selection, area);
        return true;
    } else if matches!(app.active_dialog(), Some(Dialog::BuildOptions)) {
        let machine = app
            .workspace
            .variables
            .get("MACHINE")
            .map_or("unknown", String::as_str);
        let width = area.width.saturating_sub(12).clamp(38, 84);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            area.height.saturating_sub(11) / 2,
            width,
            11,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Machine: {machine}\nCurrent image target: {}\n\nb  Build image\nc  Clean image\nm  Run menuconfig\ni  Choose image recipe\ne  Enter a target name\n\nEsc closes this menu.",
                app.build.target.as_deref().unwrap_or("not selected")
            ))
            .block(dialog_block(
                app,
                "Image build options",
                DialogTone::Standard,
            )),
            popup,
        );
        return true;
    } else if let Some(Dialog::BuildTarget { editor, task }) = app.active_dialog() {
        let title = format!(
            "Build target.toml | requested task: {}",
            task.as_deref().unwrap_or("default")
        );
        toml_popup_editor(frame, app, area, &title, editor, None);
        return true;
    }
    false
}
