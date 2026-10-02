fn render_deployment_dialogs(frame: &mut Frame, app: &App, area: Rect) -> bool {
    let (title, tone, body, hint, width, height) = match app.active_dialog() {
        Some(Dialog::DevtoolDeployConfirmation(plan)) => (
            "Confirm SSH/SCP deployment",
            DialogTone::Confirmation,
            format!(
                "Deploy the built install tree with Devtool's SSH/SCP transport?\n\nCommand: `devtool deploy-target {} {}`\nProvider: {}\nTarget: {}",
                plan.identity.name, plan.target, plan.identity.file.display(), plan.target
            ),
            "Enter continues; Esc cancels.",
            area.width.saturating_sub(8).clamp(44, 100),
            14,
        ),
        Some(Dialog::DevtoolDeploy(draft)) => (
            "Deploy build with SSH/SCP",
            DialogTone::Standard,
            format!(
                "Recipe: {}\nProvider: {}\nSSH target: {}_\n\nDevtool deploys the built install tree with SSH/SCP.",
                draft.identity.name, draft.identity.file.display(), draft.target
            ),
            "Enter previews the command; Esc cancels.",
            area.width.saturating_sub(12).clamp(44, 100),
            12,
        ),
        _ => return false,
    };
    let popup = dialog_popup_rect(area, width, height);
    clear_popup(frame, app, popup);
    let block = dialog_block(app, title, tone);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    let rows = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(inner);
    frame.render_widget(Paragraph::new(body).wrap(Wrap { trim: false }), rows[0]);
    frame.render_widget(Paragraph::new(hint).style(dialog_styles(app).hint), rows[1]);
    true
}
