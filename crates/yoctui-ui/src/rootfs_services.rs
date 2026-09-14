//! Rootfs services.
use super::*;

pub(crate) fn rootfs_udev_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let body = rootfs_workspace_shell(frame, app, area);
    let Some(inventory) = app
        .rootfs_composition
        .composition()
        .and_then(yoctui_model::RootfsComposition::system_inventory)
    else {
        frame.render_widget(
            Paragraph::new(
                "udev inventory unavailable: inspect the selected image's IMAGE_ROOTFS.",
            ),
            body,
        );
        return;
    };
    if inventory.udev_rules.is_empty() {
        frame.render_widget(Paragraph::new("No .rules files found in the image's udev search directories. Offline inventory only."), body);
        return;
    }
    let panes =
        Layout::vertical([Constraint::Percentage(45), Constraint::Percentage(55)]).split(body);
    let capacity = usize::from(panes[0].height.saturating_sub(3));
    let selected = app
        .rootfs_udev_selection
        .min(inventory.udev_rules.len() - 1);
    let start = selected.saturating_sub(capacity.saturating_sub(1));
    let rows = inventory
        .udev_rules
        .iter()
        .enumerate()
        .skip(start)
        .take(capacity)
        .map(|(index, rule)| {
            Row::new([
                rule.logical_path.0.display().to_string(),
                rule.status().into(),
            ])
            .style(selected_style(app, index == selected))
        });
    frame.render_widget(
        Table::new(rows, [Constraint::Min(12), Constraint::Length(14)])
            .header(Row::new(["Image rule path", "File selection"]))
            .block(Block::bordered().title(format!(
                "udev rules · {}/{} · offline",
                selected + 1,
                inventory.udev_rules.len()
            ))),
        panes[0],
    );
    let rule = &inventory.udev_rules[selected];
    let lines = rule
        .preview
        .lines()
        .skip(app.rootfs_udev_preview_offset)
        .take(usize::from(panes[1].height.saturating_sub(2)))
        .map(Line::from)
        .collect::<Vec<_>>();
    let preview = if lines.is_empty() {
        vec![Line::from(if rule.masked {
            "Masked by image /dev/null; no rules executed."
        } else {
            rule.limitation.as_deref().unwrap_or("Empty rule file")
        })]
    } else {
        lines
    };
    frame.render_widget(
        Paragraph::new(preview).block(Block::bordered().title(format!(
            "Rule preview · [/] scroll{}",
            if rule.preview_truncated {
                " · truncated"
            } else {
                ""
            }
        ))),
        panes[1],
    );
}

pub(crate) fn rootfs_systemd_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let body = rootfs_workspace_shell(frame, app, area);
    if let Some(lines) = rootfs_state_lines(app) {
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), body);
        return;
    }
    let Some(composition) = app.rootfs_composition.composition() else {
        return;
    };
    let Some(inventory) = composition.system_inventory() else {
        frame.render_widget(
            Paragraph::new("Offline systemd inventory is unavailable for this image."),
            body,
        );
        return;
    };
    if inventory.systemd_services.is_empty() {
        frame.render_widget(
            Paragraph::new("No systemd .service files were found in the staged IMAGE_ROOTFS."),
            body,
        );
        return;
    }
    let rows = inventory
        .systemd_services
        .iter()
        .enumerate()
        .map(|(index, service)| {
            Row::new([
                service.name.clone(),
                service
                    .description
                    .as_deref()
                    .unwrap_or("unavailable")
                    .to_owned(),
                service.bus_name.as_deref().unwrap_or("—").to_owned(),
                if service.enabled_by.is_empty() {
                    "disabled/static".into()
                } else {
                    service.enabled_by.join(", ")
                },
            ])
            .style(selected_style(app, index == app.rootfs_systemd_selection))
        });
    frame.render_widget(
        Table::new(
            rows,
            [
                Constraint::Length(28),
                Constraint::Min(28),
                Constraint::Length(28),
                Constraint::Length(24),
            ],
        )
        .header(
            Row::new(["Service", "Description", "BusName", "Enablement evidence"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::bordered()
                .title("Offline systemd service files · e edit · Enter/→ rootfs explorer"),
        ),
        body,
    );
}

pub(crate) fn rootfs_dbus_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let body = rootfs_workspace_shell(frame, app, area);
    if let Some(lines) = rootfs_state_lines(app) {
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), body);
        return;
    }
    let Some(composition) = app.rootfs_composition.composition() else {
        return;
    };
    let Some(inventory) = composition.system_inventory() else {
        frame.render_widget(
            Paragraph::new("Offline system D-Bus mapping is unavailable for this image."),
            body,
        );
        return;
    };
    if inventory.dbus_services.is_empty() {
        frame.render_widget(
            Paragraph::new(
                "No system-bus activation files or systemd BusName declarations were found.",
            ),
            body,
        );
        return;
    }
    let rows = inventory
        .dbus_services
        .iter()
        .enumerate()
        .map(|(index, service)| {
            Row::new([
                service.name.clone(),
                service.systemd_service.as_deref().unwrap_or("—").to_owned(),
                service.user.as_deref().unwrap_or("—").to_owned(),
                service.exec.as_deref().unwrap_or("—").to_owned(),
                service.policy_files.len().to_string(),
            ])
            .style(selected_style(app, index == app.rootfs_dbus_selection))
        });
    frame.render_widget(
        Table::new(
            rows,
            [
                Constraint::Length(34),
                Constraint::Length(28),
                Constraint::Length(16),
                Constraint::Min(24),
                Constraint::Length(8),
            ],
        )
        .header(
            Row::new([
                "System bus name",
                "systemd unit",
                "User",
                "Exec",
                "Policies",
            ])
            .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::bordered()
                .title("Offline system-bus activation map · e edit · Enter/→ rootfs explorer"),
        ),
        body,
    );
}
