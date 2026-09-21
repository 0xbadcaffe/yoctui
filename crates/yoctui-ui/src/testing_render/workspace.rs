pub(crate) fn testing_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let palette = ThemePalette::for_app(app);
    let active = |view| {
        if app.test_view == view {
            palette.focus()
        } else {
            Style::default()
        }
    };
    let mut lines = vec![
        Line::from(vec![
            Span::styled(" Launches ", active(TestWorkspaceView::Launches)),
            Span::raw(" | "),
            Span::styled(" Results ", active(TestWorkspaceView::Results)),
            Span::raw(" | "),
            Span::styled(" Comparison ", active(TestWorkspaceView::Comparison)),
        ]),
        Line::from(format!(
            "MACHINE={} | DISTRO={} | image={} | resulttool={}",
            app.workspace
                .variables
                .get("MACHINE")
                .map_or("unavailable", String::as_str),
            app.workspace
                .variables
                .get("DISTRO")
                .map_or("unavailable", String::as_str),
            app.build.target.as_deref().unwrap_or("unavailable"),
            resulttool_capability_label(&app.result_tool_capability),
        )),
        Line::from(""),
    ];
    match app.test_view {
        TestWorkspaceView::Launches => testing_launch_lines(app, &palette, &mut lines),
        TestWorkspaceView::Results => testing_result_lines(
            app,
            &palette,
            &mut lines,
            area.width.saturating_sub(2),
            usize::from(area.height.saturating_sub(10)).max(1),
        ),
        TestWorkspaceView::Comparison => testing_comparison_lines(
            app,
            &palette,
            &mut lines,
            usize::from(area.height.saturating_sub(10)).max(1),
        ),
    }
    frame.render_widget(
        Paragraph::new(lines)
            .block(pane_block(
                app,
                "Testing",
                app.focus == FocusTarget::Workspace,
            ))
            .wrap(Wrap { trim: false }),
        area,
    );
}

pub(crate) fn testing_inspector_text(app: &App) -> String {
    match app.test_view {
        TestWorkspaceView::Launches => testing_launch_inspector(app),
        TestWorkspaceView::Results => testing_result_inspector(app),
        TestWorkspaceView::Comparison => testing_comparison_inspector(app),
    }
}

pub(crate) fn testing_launch_lines(
    app: &App,
    palette: &ThemePalette,
    lines: &mut Vec<Line<'static>>,
) {
    lines.push(Line::from(
        "  Family                  Authority / availability",
    ));
    for family in yoctui_model::TestFamily::ALL {
        let selected = family == app.test_family_selection;
        lines.push(
            Line::from(format!(
                "{} {:<23} {}",
                if selected { "▶" } else { " " },
                family.label(),
                test_family_capability(app, family),
            ))
            .style(if selected {
                palette.selected()
            } else {
                Style::default()
            }),
        );
    }
    lines.push(Line::from(""));
    match app.latest_test_session() {
        None => lines.push(Line::from("No Testing session has run.")),
        Some(session) => {
            let status = session
                .background_job_id
                .and_then(|id| app.background_jobs.get(id))
                .map_or_else(
                    || {
                        session.outcome.map_or_else(
                            || "awaiting runner attachment".into(),
                            |outcome| format!("{outcome:?}"),
                        )
                    },
                    |job| format!("{:?}", job.status),
                );
            lines.push(Line::from(format!(
                "Latest session {} | {} | {status} | exit={} | structured results={}",
                session.id.0,
                session.operation.family().label(),
                session
                    .exit_code
                    .map_or_else(|| "—".into(), |code| code.to_string()),
                session.result_paths.len(),
            )));
            if let Some(detail) = &session.error_detail {
                lines.push(Line::styled(
                    format!("Failure: {detail}"),
                    testing_error_style(palette),
                ));
            }
        }
    }
}

pub(crate) fn testing_result_lines(
    app: &App,
    palette: &ThemePalette,
    lines: &mut Vec<Line<'static>>,
    width: u16,
    capacity: usize,
) {
    let filtered = app.filtered_test_results();
    let filtered_selection = filtered
        .iter()
        .position(|record| app.test_result_selection.as_ref() == Some(&record.identity));
    lines.push(search_line(
        app,
        &app.test_result_query,
        app.test_result_searching,
        filtered_selection,
        filtered.len(),
        SearchNavigation::Results,
        SearchExit::Done,
        width,
    ));
    if app.test_result_drilled {
        let Some(record) = app.selected_test_result() else {
            lines.push(Line::styled(
                "Selected result is no longer available.",
                testing_warning_style(palette),
            ));
            return;
        };
        lines.push(Line::from(format!(
            "Result {} | fingerprint {}",
            record.identity.path.display(),
            record.identity.fingerprint
        )));
        lines.push(Line::from("  Status    Duration     Exact suite / case"));
        let mut detail_lines = Vec::new();
        let mut selected_line = None;
        for suite in &record.suites {
            detail_lines.push(Line::styled(
                format!("  suite                  {}", suite.identity),
                testing_info_style(palette),
            ));
            for case in &suite.cases {
                let selected = app.test_case_selection.as_ref() == Some(&case.identity);
                if selected {
                    selected_line = Some(detail_lines.len());
                }
                detail_lines.push(
                    Line::from(format!(
                        "{} {:<9} {:<12} {}/{}",
                        if selected { "▶" } else { " " },
                        format!("{:?}", case.outcome).to_ascii_lowercase(),
                        case.duration
                            .map(format_duration)
                            .unwrap_or_else(|| "—".into()),
                        case.identity.suite,
                        case.identity.case,
                    ))
                    .style(if selected {
                        palette.selected()
                    } else {
                        test_outcome_style(palette, case.outcome)
                    }),
                );
            }
        }
        let viewport =
            yoctui_model::centered_viewport_range(selected_line, detail_lines.len(), capacity);
        lines.extend(detail_lines.drain(viewport));
        return;
    }
    lines.push(Line::from(
        "  Family       Machine / image             P/F/S/E/U  Exact result",
    ));
    match &app.test_results {
        TestResultInventoryState::NotLoaded => lines.push(Line::from(
            "Results are not loaded. Press I to import an exact path.",
        )),
        TestResultInventoryState::Loading { request } => lines.push(Line::styled(
            format!("Loading result generation {}…", request.generation),
            testing_info_style(palette),
        )),
        TestResultInventoryState::AvailableEmpty { .. } => {
            lines.push(Line::from("No structured test results were found."))
        }
        TestResultInventoryState::Available { .. } | TestResultInventoryState::Partial { .. } => {
            let viewport =
                yoctui_model::centered_viewport_range(filtered_selection, filtered.len(), capacity);
            for record in filtered[viewport].iter().copied() {
                let selected = app.test_result_selection.as_ref() == Some(&record.identity);
                let counts = record.counts();
                lines.push(
                    Line::from(format!(
                        "{} {:<12} {:<27} {}/{}/{}/{}/{}  {}",
                        if selected { "▶" } else { " " },
                        record.family.map_or("unknown", |family| family.label()),
                        format!(
                            "{} / {}",
                            record.machine.as_deref().unwrap_or("—"),
                            record.image.as_deref().unwrap_or("—")
                        ),
                        counts.passed,
                        counts.failed,
                        counts.skipped,
                        counts.errors,
                        counts.unknown,
                        record.identity.path.display(),
                    ))
                    .style(if selected {
                        palette.selected()
                    } else if counts.failed + counts.errors > 0 {
                        testing_error_style(palette)
                    } else {
                        Style::default()
                    }),
                );
            }
            if filtered.is_empty() {
                lines.push(Line::from("No results match the active search."));
            }
            if let TestResultInventoryState::Partial { limitations, .. } = &app.test_results {
                lines.push(Line::styled(
                    format!("Partial: {}", limitations.join(" | ")),
                    testing_warning_style(palette),
                ));
            }
        }
        TestResultInventoryState::Failed { message, .. } => lines.push(Line::styled(
            format!("Result import failed: {message}"),
            testing_error_style(palette),
        )),
        TestResultInventoryState::Cancelled { .. } => lines.push(Line::styled(
            "Result import cancelled.",
            testing_warning_style(palette),
        )),
        TestResultInventoryState::TimedOut { .. } => lines.push(Line::styled(
            "Result import timed out.",
            testing_error_style(palette),
        )),
        TestResultInventoryState::Lost { message, .. } => lines.push(Line::styled(
            format!("Result import worker lost: {message}"),
            testing_error_style(palette),
        )),
    }
}

pub(crate) fn testing_comparison_lines(
    app: &App,
    palette: &ThemePalette,
    lines: &mut Vec<Line<'static>>,
    capacity: usize,
) {
    match &app.test_comparison {
        TestComparisonState::NotSelected => lines.push(Line::from(
            "No comparison selected. Press c to choose exact results.",
        )),
        TestComparisonState::Loading { request } => lines.push(Line::styled(
            format!(
                "Comparing {} → {}…",
                request.baseline.path.display(),
                request.candidate.path.display()
            ),
            testing_info_style(palette),
        )),
        TestComparisonState::Available { comparison, .. }
        | TestComparisonState::Partial { comparison, .. } => {
            let count = |category| {
                comparison
                    .transitions
                    .iter()
                    .filter(|transition| transition.category == category)
                    .count()
            };
            lines.push(Line::from(format!(
                "regressions={} | new failures={} | new passes={} | removed={} | other={}",
                count(TestComparisonCategory::Regression),
                count(TestComparisonCategory::NewFailure),
                count(TestComparisonCategory::NewPass),
                count(TestComparisonCategory::Removed),
                count(TestComparisonCategory::UnchangedOther),
            )));
            lines.push(Line::from(format!(
                "Baseline: {}",
                comparison.baseline.path.display()
            )));
            lines.push(Line::from(format!(
                "Candidate: {}",
                comparison.candidate.path.display()
            )));
            lines.push(Line::from(
                "  Category          Baseline → candidate  Exact case",
            ));
            let selection = comparison.transitions.iter().position(|transition| {
                app.test_comparison_selection.as_ref() == Some(&transition.identity)
            });
            let viewport = yoctui_model::centered_viewport_range(
                selection,
                comparison.transitions.len(),
                capacity,
            );
            for transition in &comparison.transitions[viewport] {
                let selected = app.test_comparison_selection.as_ref() == Some(&transition.identity);
                lines.push(
                    Line::from(format!(
                        "{} {:<17} {:<9} → {:<9} {}/{}",
                        if selected { "▶" } else { " " },
                        comparison_category_label(transition.category),
                        transition.baseline.map_or_else(
                            || "absent".into(),
                            |value| { format!("{value:?}").to_ascii_lowercase() }
                        ),
                        transition.candidate.map_or_else(
                            || "absent".into(),
                            |value| { format!("{value:?}").to_ascii_lowercase() }
                        ),
                        transition.identity.suite,
                        transition.identity.case,
                    ))
                    .style(if selected {
                        palette.selected()
                    } else {
                        comparison_category_style(palette, transition.category)
                    }),
                );
            }
            if let TestComparisonState::Partial { limitations, .. } = &app.test_comparison {
                lines.push(Line::styled(
                    format!("Partial: {}", limitations.join(" | ")),
                    testing_warning_style(palette),
                ));
            }
        }
        TestComparisonState::Failed { message, .. } => lines.push(Line::styled(
            format!("Comparison failed: {message}"),
            testing_error_style(palette),
        )),
        TestComparisonState::Cancelled { .. } => lines.push(Line::styled(
            "Comparison cancelled.",
            testing_warning_style(palette),
        )),
        TestComparisonState::TimedOut { .. } => lines.push(Line::styled(
            "Comparison timed out.",
            testing_error_style(palette),
        )),
        TestComparisonState::Lost { message, .. } => lines.push(Line::styled(
            format!("Comparison worker lost: {message}"),
            testing_error_style(palette),
        )),
    }
}
