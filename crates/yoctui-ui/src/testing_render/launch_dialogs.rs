pub(crate) fn testing_popup(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    title: &str,
    tone: DialogTone,
    text: String,
    preferred_height: u16,
) {
    let popup = dialog_popup_rect(area, 92, preferred_height);
    clear_popup(frame, app, popup);
    frame.render_widget(
        Paragraph::new(text)
            .block(dialog_block(app, title, tone))
            .wrap(Wrap { trim: false }),
        popup,
    );
}

pub(crate) fn test_launch_dialog(
    frame: &mut Frame,
    app: &App,
    dialog: &TestLaunchDialog,
    area: Rect,
) {
    let marker = |field| {
        if dialog.selected_field == Some(field) {
            "▶"
        } else {
            " "
        }
    };
    let editing = if dialog.editing { " [editing]" } else { "" };
    let validation = dialog.validation_error.as_ref().map_or_else(
        || "✓ Validation: exact typed choices only.".into(),
        |error| format!("✕ Validation: {error}"),
    );
    let text = format!(
        "Family: {}\nMACHINE: {}\nDISTRO: {}\nImage: {}\n\n{} Scope: {:?}\n{} Selector: {}{}\n{} Parallelism: {}{}\n{} Verbose: {}\n{} Skip network: {}\n\n{}\n↑/↓ field | ←/→ or Enter choice | Enter edit | p preview | Esc cancel",
        dialog.draft.family.label(),
        dialog.draft.machine,
        dialog.draft.distro,
        dialog.draft.image,
        marker(TestLaunchField::Scope),
        dialog.draft.scope,
        marker(TestLaunchField::Selector),
        if dialog.draft.selector.is_empty() {
            "(none)"
        } else {
            &dialog.draft.selector
        },
        if dialog.selected_field == Some(TestLaunchField::Selector) {
            editing
        } else {
            ""
        },
        marker(TestLaunchField::Parallelism),
        dialog.parallelism_input,
        if dialog.selected_field == Some(TestLaunchField::Parallelism) {
            editing
        } else {
            ""
        },
        marker(TestLaunchField::Verbose),
        dialog.draft.verbose,
        marker(TestLaunchField::SkipNetwork),
        dialog.draft.skip_network,
        validation,
    );
    testing_popup(
        frame,
        app,
        area,
        "Testing launch",
        DialogTone::Standard,
        text,
        19,
    );
}

pub(crate) fn test_launch_confirmation(
    frame: &mut Frame,
    app: &App,
    preview: &TestLaunchPreview,
    area: Rect,
) {
    let text = match preview {
        TestLaunchPreview::Selftest(request) => format!(
            "Family: {}\nExact indexed shell-free argv:\n{}\nChild-only environment: {}\n\nEnter starts; Esc cancels.",
            request.family.label(),
            request
                .argv()
                .iter()
                .enumerate()
                .map(|(index, value)| format!("[{index}] {}", value.display()))
                .collect::<Vec<_>>()
                .join("\n"),
            if request.skip_network {
                "BB_SKIP_NETTESTS=yes"
            } else {
                "none"
            },
        ),
        TestLaunchPreview::Build {
            family,
            machine,
            distro,
            image,
            request,
        } => format!(
            "Family: {}\nMACHINE: {machine}\nDISTRO: {distro}\nImage: {image}\nExact managed BuildRequest:\ntargets={:?}\ntask={}\nforce={}\n\nEnter starts; Esc cancels.",
            family.label(),
            request.targets,
            request.task.as_deref().unwrap_or("none"),
            request.force,
        ),
    };
    testing_popup(
        frame,
        app,
        area,
        "Confirm Testing launch",
        DialogTone::Confirmation,
        text,
        18,
    );
}

pub(crate) fn test_cancellation_confirmation(
    frame: &mut Frame,
    app: &App,
    id: yoctui_model::TestSessionId,
    area: Rect,
) {
    testing_popup(
        frame,
        app,
        area,
        "Confirm Testing cancellation",
        DialogTone::Confirmation,
        format!(
            "Cancel Testing session {} only?\n\nEnter requests cancellation; Esc keeps it running.",
            id.0
        ),
        7,
    );
}
