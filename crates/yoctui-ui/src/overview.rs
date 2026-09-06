//! Read-only projections for the eight Overview insight surfaces.

use super::*;

pub(super) fn overview_workspace(frame: &mut Frame, app: &App, area: Rect, now: SystemTime) {
    let block = pane_block(app, "Insights", app.focus == FocusTarget::Workspace);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.is_empty() {
        return;
    }
    let sections = Layout::vertical([Constraint::Length(2), Constraint::Min(1)]).split(inner);
    render_overview_tabs(frame, app, sections[0]);
    match app.overview_view {
        yoctui_model::OverviewView::Timeline => render_timeline(frame, app, sections[1], now),
        yoctui_model::OverviewView::RebuildCauses => render_rebuild(frame, app, sections[1]),
        yoctui_model::OverviewView::CacheAndDownloads => render_cache(frame, app, sections[1]),
        yoctui_model::OverviewView::ImageSize => render_image_size(frame, app, sections[1]),
        yoctui_model::OverviewView::MetadataProvenance => {
            render_provenance(frame, app, sections[1])
        }
        yoctui_model::OverviewView::PackageTopology => {
            render_package_topology(frame, app, sections[1])
        }
        yoctui_model::OverviewView::SupplyChain => render_supply_chain(frame, app, sections[1]),
        yoctui_model::OverviewView::DiskUsage => render_disk(frame, app, sections[1]),
    }
}

fn render_overview_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let palette = ThemePalette::for_app(app);
    let mut spans = Vec::new();
    for (index, view) in yoctui_model::OverviewView::ALL.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(
            format!("{} {}", index + 1, view.label()),
            if *view == app.overview_view {
                palette.selected()
            } else {
                palette.role(palette.disabled, Modifier::empty())
            },
        ));
    }
    frame.render_widget(Paragraph::new(vec![
        Line::from(spans),
        Line::from("authorities: tasks · signatures · pkgdata/rootfs · security reports · host telemetry"),
    ]), area);
}

fn empty_insight(frame: &mut Frame, area: Rect, title: &str, guidance: &str) {
    frame.render_widget(
        Paragraph::new(format!("{title} unavailable\n\n{guidance}"))
            .block(Block::bordered().title(title))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_timeline(frame: &mut Frame, app: &App, area: Rect, now: SystemTime) {
    let rows = app.overview_timeline(now);
    if rows.is_empty() {
        empty_insight(
            frame,
            area,
            "Build timeline / critical path",
            "Start a build or retain completed task events to populate this view.",
        );
        return;
    }
    let total = rows
        .iter()
        .map(|row| row.start_millis.saturating_add(row.duration_millis))
        .max()
        .unwrap_or(1)
        .max(1);
    let width = usize::from(area.width.saturating_sub(39)).max(8);
    let lines = rows
        .iter()
        .take(usize::from(area.height.saturating_sub(2)))
        .map(|row| {
            let start =
                usize::try_from(u128::from(row.start_millis) * width as u128 / u128::from(total))
                    .unwrap_or(0)
                    .min(width - 1);
            let span = usize::try_from(
                u128::from(row.duration_millis.max(1)) * width as u128 / u128::from(total),
            )
            .unwrap_or(1)
            .clamp(1, width.saturating_sub(start));
            let mut bar = vec![' '; width];
            for cell in bar.iter_mut().skip(start).take(span) {
                *cell = if row.critical { '█' } else { '━' };
            }
            Line::from(format!(
                "{} {:<24.24} {:>7}ms │{}",
                if row.critical { "◆" } else { " " },
                row.label,
                row.duration_millis,
                bar.into_iter().collect::<String>()
            ))
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::bordered()
                .title("Build timeline / critical path · ◆ longest observed dependency chain"),
        ),
        area,
    );
}

fn render_rebuild(frame: &mut Frame, app: &App, area: Rect) {
    let rows = app.overview_rebuild_causes();
    if rows.is_empty() {
        empty_insight(
            frame,
            area,
            "Rebuild-cause graph",
            "Compare two task signatures in Signatures to reveal changed hashes, variables, and dependencies.",
        );
        return;
    }
    let lines = rows
        .into_iter()
        .take(usize::from(area.height.saturating_sub(2)))
        .map(|row| {
            Line::from(format!(
                "{} ──{}──▶ {}",
                row.source, row.relation, row.target
            ))
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::bordered().title("Rebuild-cause graph · BitBake signature differences")),
        area,
    );
}

fn render_cache(frame: &mut Frame, app: &App, area: Rect) {
    let cache = app.overview_cache();
    let values = [
        cache.sstate_hits,
        cache.sstate_misses,
        cache.fetch_completed,
        cache.fetch_failed,
    ];
    let chart = area.width >= 64
        && area.height >= 18
        && values.iter().sum::<usize>() > 0
        && app.color_enabled
        && app.preferences.symbols == SymbolPreference::Unicode;
    let sections = if chart {
        Layout::horizontal([Constraint::Percentage(52), Constraint::Percentage(48)]).split(area)
    } else {
        Layout::horizontal([Constraint::Length(0), Constraint::Min(1)]).split(area)
    };
    if chart {
        let slices = [
            ("sstate hit", cache.sstate_hits, Color::Green),
            ("sstate miss", cache.sstate_misses, Color::Red),
            ("fetch ok", cache.fetch_completed, Color::Cyan),
            ("fetch failed", cache.fetch_failed, Color::Yellow),
        ]
        .into_iter()
        .filter(|(_, value, _)| *value > 0)
        .map(|(label, value, color)| PieSlice::new(label, value as f64, color))
        .collect();
        frame.render_widget(
            PieChart::new(slices)
                .resolution(Resolution::Braille)
                .show_legend(true)
                .show_percentages(true)
                .legend_position(LegendPosition::Bottom)
                .block(Block::bordered().title("Observed task outcomes")),
            sections[0],
        );
    }
    let text = format!(
        "SSTATE observed\n  hits       {}\n  misses     {}\n  active     {}\n\nDownloads / do_fetch\n  completed  {}\n  failed     {}\n  active     {}\n\nSSTATE_DIR  {}\nDL_DIR      {}\n\nCounts describe retained BitBake task events. Directory byte totals remain unavailable until an authoritative bounded scan reports them.",
        cache.sstate_hits,
        cache.sstate_misses,
        cache.sstate_active,
        cache.fetch_completed,
        cache.fetch_failed,
        cache.fetch_active,
        cache.sstate_dir.as_deref().unwrap_or("unavailable"),
        cache.downloads_dir.as_deref().unwrap_or("unavailable")
    );
    frame.render_widget(
        Paragraph::new(text)
            .block(Block::bordered().title("Sstate & downloads"))
            .wrap(Wrap { trim: false }),
        sections[1],
    );
}

fn render_image_size(frame: &mut Frame, app: &App, area: Rect) {
    let rows = app.overview_image_sizes();
    if rows.is_empty() {
        empty_insight(
            frame,
            area,
            "Image-size treemap",
            "Open an image artifact and load Rootfs packages to visualize exact installed bytes by package category.",
        );
        return;
    }
    let max = rows.iter().map(|row| row.bytes).max().unwrap_or(1).max(1);
    let width = usize::from(area.width.saturating_sub(32)).max(4);
    let mut lines = Vec::new();
    if let Some(delta) = app.overview_image_size_delta() {
        let sign = if delta.delta_bytes >= 0 { "+" } else { "-" };
        lines.push(Line::from(format!(
            "Current {} · previous {} · build delta {sign}{}",
            format_bytes(delta.current_bytes),
            format_bytes(delta.previous_bytes),
            format_signed_bytes(delta.delta_bytes)
        )));
    } else {
        lines.push(Line::from(
            "Build delta unavailable until this image target is loaded after another build.",
        ));
    }
    lines.push(Line::default());
    lines.extend(
        rows.iter()
            .take(usize::from(area.height.saturating_sub(4)))
            .map(|row| {
                let cells =
                    usize::try_from(u128::from(row.bytes) * width as u128 / u128::from(max))
                        .unwrap_or(width)
                        .clamp(1, width);
                Line::from(format!(
                    "{:<18.18} {:>10} {}",
                    row.label,
                    format_bytes(row.bytes),
                    "█".repeat(cells)
                ))
            }),
    );
    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::bordered().title("Image-size treemap · exact rootfs installed bytes")),
        area,
    );
}

fn format_signed_bytes(bytes: i128) -> String {
    let magnitude = bytes.unsigned_abs().min(u128::from(u64::MAX)) as u64;
    format_bytes(magnitude)
}

fn render_provenance(frame: &mut Frame, app: &App, area: Rect) {
    let rows = app.overview_provenance();
    if rows.is_empty() {
        empty_insight(
            frame,
            area,
            "Metadata provenance graph",
            "Load workspace metadata or inspect variables in Configuration to populate provenance chains.",
        );
        return;
    }
    let lines = rows
        .into_iter()
        .take(usize::from(area.height.saturating_sub(2)))
        .map(|row| {
            Line::from(format!(
                "{} ──{}──▶ {}",
                row.source, row.relation, row.target
            ))
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(lines).block(Block::bordered().title("Metadata provenance graph")),
        area,
    );
}

fn render_package_topology(frame: &mut Frame, app: &App, area: Rect) {
    let rows = app.overview_package_topology();
    if rows.is_empty() {
        empty_insight(
            frame,
            area,
            "Runtime package dependency topology",
            "Open package details in Packages; loaded oe-pkgdata-util RDEPENDS edges appear here.",
        );
        return;
    }
    let lines = rows
        .into_iter()
        .take(usize::from(area.height.saturating_sub(2)))
        .map(|row| {
            Line::from(format!(
                "{} ──{}──▶ {}",
                row.source, row.relation, row.target
            ))
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(lines).block(Block::bordered().title("Runtime package dependency topology")),
        area,
    );
}

fn render_supply_chain(frame: &mut Frame, app: &App, area: Rect) {
    let report = app.overview_supply_chain();
    if report.cve_reports
        + report.spdx_documents
        + report.cyclonedx_documents
        + report.manifest_documents
        == 0
    {
        empty_insight(
            frame,
            area,
            "CVE / license / SBOM overlay",
            "Import CVE, SPDX, CycloneDX JSON, or a deployed Yocto image .manifest in Security.",
        );
        return;
    }
    let mut lines = vec![
        Line::from(format!(
            "CVE reports       {:>5}    vulnerable findings {:>5}",
            report.cve_reports, report.vulnerable
        )),
        Line::from(format!("SPDX documents    {:>5}", report.spdx_documents)),
        Line::from(format!(
            "CycloneDX docs    {:>5}",
            report.cyclonedx_documents
        )),
        Line::from(format!(
            "Package manifests {:>5}    fallback inventory",
            report.manifest_documents
        )),
        Line::from(format!("SBOM components   {:>5}", report.components)),
        Line::default(),
        Line::from(
            "Package manifests preserve package/version inventory when the Yocto release has no SBOM generator. License, supplier, relationship, and file claims remain unavailable in that fallback.",
        ),
    ];
    lines.extend(
        report
            .limitations
            .iter()
            .map(|value| Line::from(format!("! {value}"))),
    );
    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::bordered().title("CVE / license / SBOM overlay"))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_disk(frame: &mut Frame, app: &App, area: Rect) {
    let history = &app.host_telemetry_history;
    if history.build_filesystem_percent.is_empty()
        && history.disk_read_bytes_per_second.is_empty()
        && history.disk_write_bytes_per_second.is_empty()
    {
        empty_insight(
            frame,
            area,
            "Build disk-usage timeline",
            "Wait for host telemetry samples from a connected build environment.",
        );
        return;
    }
    let rows = Layout::vertical([
        Constraint::Percentage(34),
        Constraint::Percentage(33),
        Constraint::Percentage(33),
    ])
    .split(area);
    for (title, samples, target) in [
        (
            "Build filesystem used %",
            &history.build_filesystem_percent,
            rows[0],
        ),
        (
            "Disk reads bytes/s",
            &history.disk_read_bytes_per_second,
            rows[1],
        ),
        (
            "Disk writes bytes/s",
            &history.disk_write_bytes_per_second,
            rows[2],
        ),
    ] {
        let samples = samples.iter().copied().collect::<Vec<_>>();
        frame.render_widget(
            Sparkline::default()
                .data(&samples)
                .block(Block::bordered().title(title))
                .style(Style::default().fg(Color::Cyan)),
            target,
        );
    }
}
