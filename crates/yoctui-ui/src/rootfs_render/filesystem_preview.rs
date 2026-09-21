pub(crate) fn render_rootfs_filesystem_preview(
    frame: &mut Frame,
    app: &App,
    composition: &yoctui_model::RootfsComposition,
    area: Rect,
) {
    let Some(tree) = composition.filesystem_tree() else {
        frame.render_widget(
            Paragraph::new("Filesystem tree unavailable · use Tab for authority details")
                .block(Block::bordered().title("Filesystem drill-down")),
            area,
        );
        return;
    };
    let selected = app
        .rootfs_entry_selection
        .as_ref()
        .and_then(|identity| {
            tree.entries
                .iter()
                .position(|entry| &entry.identity == identity)
        })
        .unwrap_or(0);
    let visible = usize::from(area.height.saturating_sub(4)).max(1);
    let start = selected
        .saturating_sub(visible / 2)
        .min(tree.entries.len().saturating_sub(visible));
    let position = BoundedScrollIndicator::new(start, visible, tree.entries.len()).label();
    let rows = tree.entries.iter().skip(start).take(visible).map(|entry| {
        let selected = app.rootfs_entry_selection.as_ref() == Some(&entry.identity);
        let kind = match entry.kind {
            RootfsEntryKind::Directory => "dir",
            RootfsEntryKind::RegularFile => "file",
            RootfsEntryKind::Symlink => "link",
            RootfsEntryKind::Other => "special",
        };
        Row::new([
            Cell::from(entry.identity.0.display().to_string()),
            Cell::from(kind),
            Cell::from(entry.size_bytes.to_string()),
            Cell::from(
                entry
                    .package
                    .as_ref()
                    .map_or("unavailable", |package| package.name.as_str()),
            ),
        ])
        .style(selected_style(app, selected))
    });
    let state = match &composition.filesystem_tree {
        yoctui_model::RootfsAuthority::Available(_) => "available",
        yoctui_model::RootfsAuthority::Partial { .. } => "partial",
        yoctui_model::RootfsAuthority::Unavailable { .. } => "unavailable",
    };
    frame.render_widget(
        Table::new(
            rows,
            [
                Constraint::Min(18),
                Constraint::Length(7),
                Constraint::Length(9),
                Constraint::Length(14),
            ],
        )
        .header(
            Row::new(["Logical path", "Kind", "Exact B", "Package"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(Block::bordered().title(format!(
            "Filesystem tree: {state} · {position} · Tab drill-down"
        ))),
        area,
    );
}

pub(crate) fn rootfs_group_label(identity: &RootfsGroupIdentity) -> String {
    match identity {
        RootfsGroupIdentity::Category(category) => category.clone(),
        RootfsGroupIdentity::Other => "Other".into(),
    }
}

pub(crate) fn render_rootfs_package_table(
    frame: &mut Frame,
    app: &App,
    inventory: &yoctui_model::RootfsPackageInventory,
    groups: &[yoctui_model::RootfsGroupRow],
    total: u64,
    area: Rect,
) {
    let mut lines = vec![
        Line::from(format!(
            "Installed-package authority · {} packages · {} exact bytes",
            inventory.packages.len(),
            total
        )),
        Line::from("h/l group · j/k package · r refresh · Tab filesystem"),
        Line::from("Category                     Packages   Exact bytes        Percent"),
    ];
    for group in groups {
        let selected = app.rootfs_group_selection.as_ref() == Some(&group.identity);
        let bar_width = usize::from(area.width.saturating_sub(66).min(20));
        let filled = usize::from(group.percent_basis_points) * bar_width / 10_000;
        let bar = if bar_width == 0 {
            String::new()
        } else {
            format!(" {}{}", "#".repeat(filled), ".".repeat(bar_width - filled))
        };
        lines.push(
            Line::from(format!(
                "{:<28} {:>8} {:>13} {:>6}.{:02}%{}",
                rootfs_group_label(&group.identity),
                group.package_count,
                group.installed_size_bytes,
                group.percent_basis_points / 100,
                group.percent_basis_points % 100,
                bar
            ))
            .style(selected_style(app, selected)),
        );
    }
    let selected_group = app
        .rootfs_group_selection
        .as_ref()
        .and_then(|identity| groups.iter().find(|group| &group.identity == identity));
    lines.push(Line::from(""));
    if let Some(group) = selected_group {
        let selected_position = app
            .rootfs_package_selection
            .as_ref()
            .and_then(|selected| group.members.iter().position(|member| member == selected))
            .unwrap_or(0);
        lines.push(Line::from(format!(
            "Packages in {} · {} of {} · Other membership remains inspectable",
            rootfs_group_label(&group.identity),
            selected_position.saturating_add(1).min(group.members.len()),
            group.members.len()
        )));
        lines.push(Line::from(
            "Package                       Recipe                 Exact bytes   Files",
        ));
        let remaining = usize::from(area.height).saturating_sub(lines.len()).max(1);
        let start = selected_position
            .saturating_sub(remaining / 2)
            .min(group.members.len().saturating_sub(remaining));
        for identity in group.members.iter().skip(start).take(remaining) {
            if let Some(package) = inventory
                .packages
                .iter()
                .find(|package| &package.identity == identity)
            {
                let selected = app.rootfs_package_selection.as_ref() == Some(identity);
                lines.push(
                    Line::from(format!(
                        "{:<29} {:<22} {:>11} {:>7}",
                        package.identity.name,
                        package.recipe.as_deref().unwrap_or("unavailable"),
                        package.installed_size_bytes,
                        package.file_count
                    ))
                    .style(selected_style(app, selected)),
                );
            }
        }
    } else {
        lines.push(Line::from("No installed-package groups were reported."));
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}
