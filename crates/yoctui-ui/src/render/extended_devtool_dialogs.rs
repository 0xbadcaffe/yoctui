fn render_extended_devtool_dialogs(frame: &mut Frame, app: &App, area: Rect) -> bool {
    let (title, text, tone, width) = match app.active_dialog() {
        Some(Dialog::DevtoolUpgradeConfirmation(plan)) => (
            "Confirm Devtool upgrade",
            format!(
                "Run `devtool upgrade {}`?\n\nProvider: {}\n\nEnter continues; Esc cancels.",
                plan.identity.name,
                plan.identity.file.display()
            ),
            DialogTone::Confirmation,
            area.width.saturating_sub(8).clamp(44, 100),
        ),
        Some(Dialog::DevtoolUndeployConfirmation(plan)) => (
            "Confirm Devtool undeploy-target",
            format!(
                "Run `devtool undeploy-target {} {}`?\n\nProvider: {}\nTarget: {}\n\nEnter continues; Esc cancels.",
                plan.identity.name,
                plan.target,
                plan.identity.file.display(),
                plan.target
            ),
            DialogTone::Confirmation,
            area.width.saturating_sub(8).clamp(44, 100),
        ),
        Some(Dialog::DevtoolUndeploy(draft)) => (
            "Devtool undeploy target",
            format!(
                "Recipe: {}\nProvider: {}\nDeployment target: {}_\n\nEnter previews the command; Esc cancels.",
                draft.identity.name,
                draft.identity.file.display(),
                draft.target
            ),
            DialogTone::Standard,
            area.width.saturating_sub(12).clamp(44, 100),
        ),
        _ => return false,
    };
    let popup = Rect::new(
        (area.width.saturating_sub(width)) / 2,
        area.height.saturating_sub(8) / 2,
        width,
        8,
    );
    clear_popup(frame, app, popup);
    frame.render_widget(
        Paragraph::new(text)
            .block(dialog_block(app, title, tone))
            .wrap(Wrap { trim: true }),
        popup,
    );
    true
}
