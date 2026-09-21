#[derive(Clone, Copy)]
pub(crate) enum TelemetryCell {
    Cpu,
    Ram,
    BuildFilesystem,
    Sstate,
    DiskRead,
    DiskWrite,
    NetworkReceive,
    NetworkTransmit,
    DiskIo,
}

pub(crate) fn render_telemetry_cell(
    frame: &mut Frame,
    app: &App,
    projection: &yoctui_model::HostTelemetryProjection,
    area: Rect,
    cell: TelemetryCell,
    divider: bool,
) {
    let block = Block::default().borders(if divider {
        Borders::RIGHT
    } else {
        Borders::NONE
    });
    let inner = block.inner(area);
    frame.render_widget(block, area);
    match cell {
        TelemetryCell::Cpu => render_cpu_telemetry_cell(frame, app, inner),
        TelemetryCell::Ram => render_ram_telemetry_cell(frame, app, inner),
        TelemetryCell::BuildFilesystem => {
            render_build_filesystem_telemetry_cell(frame, app, inner);
        }
        TelemetryCell::Sstate => {
            let palette = ThemePalette::for_app(app);
            frame.render_widget(
                Paragraph::new("SSTATE ! unavailable")
                    .style(palette.role(palette.disabled, Modifier::DIM)),
                inner,
            );
        }
        TelemetryCell::DiskRead => {
            render_disk_io_projection(frame, app, projection, inner, Rect::default());
        }
        TelemetryCell::DiskWrite => {
            render_disk_io_projection(frame, app, projection, Rect::default(), inner);
        }
        TelemetryCell::NetworkReceive => {
            render_network_io_projection(frame, app, projection, inner, Rect::default());
        }
        TelemetryCell::NetworkTransmit => {
            render_network_io_projection(frame, app, projection, Rect::default(), inner);
        }
        TelemetryCell::DiskIo => {
            let rows = Layout::vertical([Constraint::Length(1); 2]).split(inner);
            render_disk_io_projection(frame, app, projection, rows[0], rows[1]);
        }
    }
}

pub(crate) fn render_compact_telemetry_strip(frame: &mut Frame, app: &App, area: Rect) {
    let palette = ThemePalette::for_app(app);
    let block = pane_block(app, "Resources", false).style(palette.base());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.is_empty() {
        return;
    }
    let telemetry = &app.host_telemetry;
    let values = [
        (
            "CPU",
            telemetry
                .cpu_utilization_percent
                .map(|value| value.min(100)),
        ),
        (
            "RAM",
            utilization_percent(
                telemetry.memory_total_bytes,
                telemetry.memory_available_bytes,
            ),
        ),
        (
            "FS",
            app.workspace.build_dir.as_ref().and_then(|_| {
                utilization_percent(telemetry.disk_total_bytes, telemetry.disk_available_bytes)
            }),
        ),
    ];
    let cells = Layout::horizontal([Constraint::Ratio(1, 3); 3]).split(inner);
    let unicode = app.preferences.symbols == SymbolPreference::Unicode;
    for ((label, percent), cell) in values.into_iter().zip(cells.iter()) {
        let text = percent.map_or_else(
            || format!("{label} --"),
            |value| format!("{label} {value}%"),
        );
        frame.render_widget(
            Paragraph::new(text).style(palette.role(palette.informational, Modifier::BOLD)),
            Rect::new(cell.x, cell.y, cell.width, 1),
        );
        if cell.height < 2 {
            continue;
        }
        let bar = Rect::new(cell.x, cell.y + 1, cell.width.saturating_sub(1), 1);
        let Some(percent) = percent else {
            frame.render_widget(
                Paragraph::new("unavailable").style(palette.role(palette.muted, Modifier::DIM)),
                bar,
            );
            continue;
        };
        let filled = (u32::from(percent) * u32::from(bar.width)).div_ceil(100);
        for index in 0..bar.width {
            let active = u32::from(index) < filled;
            let segment = u32::from(index + 1) * 100 / u32::from(bar.width.max(1));
            let color = if !active {
                palette.muted
            } else if segment >= 90 {
                palette.error
            } else if segment >= 70 {
                palette.warning
            } else {
                palette.success
            };
            let symbol = match (unicode, active) {
                (true, true) => "▪",
                (true, false) => "▫",
                (false, true) => "#",
                (false, false) => ".",
            };
            if let Some(cell) = frame.buffer_mut().cell_mut((bar.x + index, bar.y)) {
                cell.set_symbol(symbol).set_style(palette.role(
                    color,
                    if active {
                        Modifier::BOLD
                    } else {
                        Modifier::DIM
                    },
                ));
            }
        }
    }
}
