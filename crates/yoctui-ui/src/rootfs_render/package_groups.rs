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
        let slices = groups
            .iter()
            .zip(labels.iter())
            .enumerate()
            .map(|(index, (group, label))| {
                PieSlice::new(
                    label,
                    group.installed_size_bytes as f64,
                    rootfs_group_color(app, group, index),
                )
            })
            .collect();
        frame.render_widget(
            PieChart::new(slices)
                .block(Block::bordered().title("Rootfs packages · installed bytes"))
                .resolution(Resolution::Braille)
                .show_percentages(false)
                .show_legend(false),
            columns[0],
        );
        render_rootfs_exact_group_table(frame, app, &groups, total, columns[1]);
        render_rootfs_accessible_selection(frame, app, inventory, &groups, sections[1]);
        render_rootfs_filesystem_preview(frame, app, composition, sections[2]);
    } else {
        render_rootfs_package_table(frame, app, inventory, &groups, total, body);
    }
}

fn rootfs_group_color(app: &App, group: &yoctui_model::RootfsGroupRow, index: usize) -> Color {
    let palette = ThemePalette::for_app(app);
    if group.identity == RootfsGroupIdentity::Other {
        return palette.muted;
    }
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
    colors[index % colors.len()]
}

pub(crate) fn render_rootfs_exact_group_table(
    frame: &mut Frame,
    app: &App,
    groups: &[yoctui_model::RootfsGroupRow],
    total: u64,
    area: Rect,
) {
    let rows = groups.iter().enumerate().map(|(index, group)| {
        let selected = app.rootfs_group_selection.as_ref() == Some(&group.identity);
        Row::new([
            Cell::from(Line::from(vec![
                Span::styled(
                    "● ",
                    Style::default().fg(rootfs_group_color(app, group, index)),
                ),
                Span::raw(rootfs_group_label(&group.identity)),
            ])),
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
