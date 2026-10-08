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
    let group_height = (area.height.saturating_sub(4) / 2)
        .min(groups.len().saturating_add(1) as u16)
        .max(2);
    let sections = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(group_height),
        Constraint::Min(3),
    ])
    .split(area);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from("Installed-package authority"),
            Line::from(format!(
                "{} packages · {} exact bytes",
                inventory.packages.len(),
                total
            )),
        ]),
        sections[0],
    );
    let selected_group = app
        .rootfs_group_selection
        .as_ref()
        .and_then(|identity| groups.iter().position(|group| &group.identity == identity));
    let viewport = yoctui_model::centered_viewport_range(
        selected_group,
        groups.len(),
        usize::from(sections[1].height.saturating_sub(1)).max(1),
    );
    let rows = groups[viewport].iter().map(|group| {
        Row::new([
            Cell::from(rootfs_group_label(&group.identity)),
            Cell::from(group.package_count.to_string()),
            Cell::from(group.installed_size_bytes.to_string()),
            Cell::from(format!(
                "{}.{:02}%",
                group.percent_basis_points / 100,
                group.percent_basis_points % 100
            )),
        ])
        .style(selected_style(
            app,
            app.rootfs_group_selection.as_ref() == Some(&group.identity),
        ))
    });
    frame.render_widget(
        Table::new(
            rows,
            [
                Constraint::Min(10),
                Constraint::Length(5),
                Constraint::Length(11),
                Constraint::Length(7),
            ],
        )
        .header(
            Row::new(["Category", "Pkgs", "Exact bytes", "%"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        ),
        sections[1],
    );
    let Some(group) = selected_group.map(|index| &groups[index]) else {
        frame.render_widget(
            Paragraph::new("No installed-package groups were reported."),
            sections[2],
        );
        return;
    };
    let selected = app
        .rootfs_package_selection
        .as_ref()
        .and_then(|identity| group.members.iter().position(|member| member == identity));
    let capacity = usize::from(sections[2].height.saturating_sub(3)).max(1);
    let viewport = yoctui_model::centered_viewport_range(selected, group.members.len(), capacity);
    let rows = group.members[viewport].iter().filter_map(|identity| {
        let package = inventory
            .packages
            .iter()
            .find(|package| &package.identity == identity)?;
        Some(
            Row::new([
                Cell::from(package.identity.name.clone()),
                Cell::from(
                    package
                        .recipe
                        .clone()
                        .unwrap_or_else(|| "unavailable".into()),
                ),
                Cell::from(package.installed_size_bytes.to_string()),
                Cell::from(package.file_count.to_string()),
            ])
            .style(selected_style(
                app,
                app.rootfs_package_selection.as_ref() == Some(identity),
            )),
        )
    });
    frame.render_widget(
        Table::new(
            rows,
            [
                Constraint::Min(12),
                Constraint::Min(8),
                Constraint::Length(11),
                Constraint::Length(5),
            ],
        )
        .header(
            Row::new(["Package", "Recipe", "Exact bytes", "Files"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::bordered()
                .title(format!(
                    "Packages in {} · {}/{} · Other membership",
                    rootfs_group_label(&group.identity),
                    selected.map_or(0, |index| index + 1),
                    group.members.len()
                ))
                .title_bottom("h/l group · j/k package · Tab files"),
        ),
        sections[2],
    );
}
