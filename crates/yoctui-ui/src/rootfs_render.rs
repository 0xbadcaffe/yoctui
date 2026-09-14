//! Rootfs render.
use super::*;

pub(crate) fn rootfs_workspace_shell(frame: &mut Frame, app: &App, area: Rect) -> Rect {
    let block = pane_block(
        app,
        "Images · Rootfs composition",
        app.focus == FocusTarget::Workspace,
    );
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 {
        return inner;
    }
    let tabs = Rect::new(inner.x, inner.y, inner.width, 1);
    frame.render_widget(Paragraph::new(images_tabs_line(app, tabs.width)), tabs);
    Rect::new(
        inner.x,
        inner.y.saturating_add(1),
        inner.width,
        inner.height.saturating_sub(1),
    )
}

pub(crate) fn rootfs_packages_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let body = rootfs_workspace_shell(frame, app, area);
    if body.height == 0 {
        return;
    }
    if let Some(lines) = rootfs_state_lines(app) {
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), body);
        return;
    }
    let Some(composition) = app.rootfs_composition.composition() else {
        return;
    };
    let Some(inventory) = composition.package_inventory() else {
        let reason = match &composition.installed_packages {
            yoctui_model::RootfsAuthority::Unavailable { reason } => reason.as_str(),
            _ => "installed-package authority is unavailable",
        };
        frame.render_widget(
            Paragraph::new(format!(
                "Installed packages unavailable\n{reason}\n\nFilesystem evidence remains separate; press Tab."
            ))
            .wrap(Wrap { trim: false }),
            body,
        );
        return;
    };
    let groups = inventory.grouped(8);
    let total = composition.totals().0.installed_package_bytes;
    let can_render_pie = app.color_enabled
        && body.width >= 64
        && body.height >= 36
        && app.theme != Theme::Monochrome
        && app.preferences.symbols == SymbolPreference::Unicode
        && app.preferences.charts == yoctui_model::ChartPreference::Automatic
        && total > 0
        && !groups.is_empty();
    if can_render_pie {
        let chart_height = body.height.saturating_mul(52).div_ceil(100).clamp(18, 24);
        let sections = Layout::vertical([
            Constraint::Length(chart_height),
            Constraint::Length(11),
            Constraint::Min(7),
        ])
        .split(body);
        let columns = Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(sections[0]);
        let labels = groups
            .iter()
            .map(|group| rootfs_group_label(&group.identity))
            .collect::<Vec<_>>();
        let palette = ThemePalette::for_app(app);
        let colors = [
            palette.progress,
            palette.informational,
            palette.warning,
            Color::Magenta,
            Color::Blue,
            palette.muted,
            Color::LightCyan,
            Color::Gray,
        ];
        let slices = groups
            .iter()
            .zip(labels.iter())
            .enumerate()
            .map(|(index, (group, label))| {
                PieSlice::new(
                    label,
                    group.installed_size_bytes as f64,
                    if group.identity == RootfsGroupIdentity::Other || label == "Other" {
                        palette.muted
                    } else {
                        colors[index % colors.len()]
                    },
                )
            })
            .collect();
        frame.render_widget(
            PieChart::new(slices)
                .block(Block::bordered().title("Rootfs packages · installed bytes"))
                .resolution(Resolution::Braille)
                .show_percentages(true)
                .show_legend(true)
                .legend_position(LegendPosition::Right),
            columns[0],
        );
        render_rootfs_exact_group_table(frame, app, &groups, total, columns[1]);
        render_rootfs_accessible_selection(frame, app, inventory, &groups, sections[1]);
        render_rootfs_filesystem_preview(frame, app, composition, sections[2]);
    } else {
        render_rootfs_package_table(frame, app, inventory, &groups, total, body);
    }
}

pub(crate) fn render_rootfs_exact_group_table(
    frame: &mut Frame,
    app: &App,
    groups: &[yoctui_model::RootfsGroupRow],
    total: u64,
    area: Rect,
) {
    let rows = groups.iter().map(|group| {
        let selected = app.rootfs_group_selection.as_ref() == Some(&group.identity);
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
        .style(selected_style(app, selected))
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
        )
        .block(Block::bordered().title(format!("Exact composition table · {total} B"))),
        area,
    );
}

pub(crate) fn render_rootfs_accessible_selection(
    frame: &mut Frame,
    app: &App,
    inventory: &yoctui_model::RootfsPackageInventory,
    groups: &[yoctui_model::RootfsGroupRow],
    area: Rect,
) {
    let unicode = app.preferences.symbols == SymbolPreference::Unicode;
    let selected_group = app
        .rootfs_group_selection
        .as_ref()
        .and_then(|identity| groups.iter().find(|group| &group.identity == identity));
    let mut lines = Vec::new();
    if let Some(group) = selected_group {
        let mut group_state = yoctui_model::CheckboxState::new(
            "rootfs-group",
            format!(
                "{} group · {} packages",
                rootfs_group_label(&group.identity),
                group.package_count
            ),
        );
        group_state.value = yoctui_model::CheckboxValue::Indeterminate;
        lines.push(Line::from(checkbox_text(&group_state, unicode)));
        let selected_position = app
            .rootfs_package_selection
            .as_ref()
            .and_then(|selected| group.members.iter().position(|member| member == selected))
            .unwrap_or(0);
        let visible = usize::from(area.height.saturating_sub(6)).max(2);
        let start = selected_position
            .saturating_sub(visible / 2)
            .min(group.members.len().saturating_sub(visible));
        for identity in group.members.iter().skip(start).take(visible) {
            let Some(package) = inventory
                .packages
                .iter()
                .find(|package| &package.identity == identity)
            else {
                continue;
            };
            let selected = app.rootfs_package_selection.as_ref() == Some(identity);
            let mut row = yoctui_model::CheckboxState::new(
                format!("rootfs-package:{}", package.identity.name),
                format!(
                    "{} · {} B · {} files",
                    package.identity.name, package.installed_size_bytes, package.file_count
                ),
            );
            row.value = if selected {
                yoctui_model::CheckboxValue::Checked
            } else {
                yoctui_model::CheckboxValue::Unchecked
            };
            row.focused = selected;
            lines.push(Line::styled(
                checkbox_text(&row, unicode),
                selected_style(app, selected),
            ));
        }
    }
    let mut ownership =
        yoctui_model::CheckboxState::new("rootfs-ownership", "Filesystem ownership");
    ownership.set_disabled("partial; Tab for evidence");
    lines.push(Line::from(checkbox_text(&ownership, unicode)));
    lines.push(Line::from(
        "Other membership remains inspectable · exact bytes stay authoritative",
    ));
    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::bordered().title("Accessible package selection · j/k moves checked row")),
        area,
    );
}

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

pub(crate) fn rootfs_filesystem_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let body = rootfs_workspace_shell(frame, app, area);
    if body.height == 0 {
        return;
    }
    if let Some(lines) = rootfs_state_lines(app) {
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), body);
        return;
    }
    let Some(composition) = app.rootfs_composition.composition() else {
        return;
    };
    let Some(tree) = composition.filesystem_tree() else {
        let reason = match &composition.filesystem_tree {
            yoctui_model::RootfsAuthority::Unavailable { reason } => reason.as_str(),
            _ => "filesystem authority is unavailable",
        };
        frame.render_widget(
            Paragraph::new(format!(
                "Filesystem tree unavailable\n{reason}\n\nInstalled-package evidence remains separate; press Shift-Tab."
            ))
            .wrap(Wrap { trim: false }),
            body,
        );
        return;
    };
    let totals = composition.totals().0;
    let selected = app
        .rootfs_entry_selection
        .as_ref()
        .and_then(|identity| {
            tree.entries
                .iter()
                .position(|entry| &entry.identity == identity)
        })
        .unwrap_or(0);
    let header_rows = 4_usize;
    let visible = usize::from(body.height).saturating_sub(header_rows).max(1);
    let start = selected
        .saturating_sub(visible / 2)
        .min(tree.entries.len().saturating_sub(visible));
    let mut lines = vec![
        Line::from(format!(
            "Filesystem authority · {} entries · {} exact bytes",
            tree.entries.len(),
            totals.filesystem_bytes
        )),
        Line::from(format!(
            "files {} · dirs {} · symlinks {} · special {} · j/k navigate · r refresh",
            totals.files, totals.directories, totals.symlinks, totals.other
        )),
        Line::from(format!(
            "Showing {}–{} of {} · package ownership is independent authority",
            start.saturating_add(1).min(tree.entries.len()),
            start.saturating_add(visible).min(tree.entries.len()),
            tree.entries.len()
        )),
        Line::from(
            "Logical path                                      Kind          Exact bytes   Package",
        ),
    ];
    for entry in tree.entries.iter().skip(start).take(visible) {
        let is_selected = app.rootfs_entry_selection.as_ref() == Some(&entry.identity);
        let indent = "  ".repeat(entry.identity.depth().saturating_sub(1).min(12));
        let branch = if app.theme == Theme::Monochrome
            || app.preferences.symbols == SymbolPreference::Ascii
        {
            "|-"
        } else {
            "├─"
        };
        let name = if entry.identity.0 == std::path::Path::new("/") {
            "/".into()
        } else {
            entry
                .identity
                .0
                .file_name()
                .map_or_else(|| "?".into(), |name| name.to_string_lossy().into_owned())
        };
        let kind = match entry.kind {
            RootfsEntryKind::Directory => "directory",
            RootfsEntryKind::RegularFile => "file",
            RootfsEntryKind::Symlink => "symlink",
            RootfsEntryKind::Other => "special",
        };
        lines.push(
            Line::from(format!(
                "{:<49} {:<12} {:>11}   {}",
                format!("{indent}{branch} {name}"),
                kind,
                entry.size_bytes,
                entry
                    .package
                    .as_ref()
                    .map_or("unavailable", |value| value.name.as_str())
            ))
            .style(selected_style(app, is_selected)),
        );
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), body);
}
