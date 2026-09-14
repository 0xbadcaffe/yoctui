//! Testing render.
use super::*;

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

pub(crate) fn testing_info_style(palette: &ThemePalette) -> Style {
    palette.role(palette.informational, Modifier::ITALIC)
}

pub(crate) fn testing_warning_style(palette: &ThemePalette) -> Style {
    palette.role(palette.warning, Modifier::BOLD)
}

pub(crate) fn testing_error_style(palette: &ThemePalette) -> Style {
    palette.role(palette.error, Modifier::BOLD | Modifier::UNDERLINED)
}

pub(crate) fn test_outcome_style(
    palette: &ThemePalette,
    outcome: yoctui_model::TestCaseOutcome,
) -> Style {
    match outcome {
        yoctui_model::TestCaseOutcome::Passed => palette.role(palette.success, Modifier::BOLD),
        yoctui_model::TestCaseOutcome::Skipped | yoctui_model::TestCaseOutcome::Unknown => {
            palette.role(palette.warning, Modifier::ITALIC)
        }
        yoctui_model::TestCaseOutcome::Failed | yoctui_model::TestCaseOutcome::Error => {
            testing_error_style(palette)
        }
    }
}

pub(crate) fn comparison_category_label(category: TestComparisonCategory) -> &'static str {
    match category {
        TestComparisonCategory::Regression => "regression",
        TestComparisonCategory::NewFailure => "new failure",
        TestComparisonCategory::NewPass => "new pass",
        TestComparisonCategory::Removed => "removed",
        TestComparisonCategory::UnchangedOther => "unchanged/other",
    }
}

pub(crate) fn comparison_category_style(
    palette: &ThemePalette,
    category: TestComparisonCategory,
) -> Style {
    match category {
        TestComparisonCategory::Regression | TestComparisonCategory::NewFailure => {
            testing_error_style(palette)
        }
        TestComparisonCategory::NewPass => palette.role(palette.success, Modifier::BOLD),
        TestComparisonCategory::Removed => testing_warning_style(palette),
        TestComparisonCategory::UnchangedOther => Style::default(),
    }
}

pub(crate) fn resulttool_capability_label(
    capability: &yoctui_model::ResultToolCapability,
) -> String {
    match capability {
        yoctui_model::ResultToolCapability::NotInspected => "pending".into(),
        yoctui_model::ResultToolCapability::Missing => "missing".into(),
        yoctui_model::ResultToolCapability::Available(path) => path.display().to_string(),
        yoctui_model::ResultToolCapability::Failed(message) => format!("failed: {message}"),
    }
}

pub(crate) fn test_family_capability(app: &App, family: yoctui_model::TestFamily) -> String {
    let executable = |capability: &TestExecutableCapability| match capability {
        TestExecutableCapability::NotInspected => "capability pending".into(),
        TestExecutableCapability::Missing => "executable missing".into(),
        TestExecutableCapability::Available(path) => path.display().to_string(),
        TestExecutableCapability::Failed(message) => format!("inspection failed: {message}"),
    };
    match family {
        yoctui_model::TestFamily::OeSelftest => executable(&app.test_capability.oe_selftest),
        yoctui_model::TestFamily::BitbakeSelftest => {
            executable(&app.test_capability.bitbake_selftest)
        }
        yoctui_model::TestFamily::Ptest => match &app.test_capability.ptest {
            yoctui_model::PtestCapability::NotInspected => "prerequisites pending".into(),
            yoctui_model::PtestCapability::Configured => {
                "Configured do_testimage (ptest suite)".into()
            }
            yoctui_model::PtestCapability::Unavailable(reason) => {
                format!("unavailable: {reason}")
            }
            yoctui_model::PtestCapability::Failed(message) => {
                format!("inspection failed: {message}")
            }
        },
        family => format!(
            "managed BitBake do_{}",
            family.task().unwrap_or("unavailable")
        ),
    }
}

pub(crate) fn testing_launch_inspector(app: &App) -> String {
    let family = app.test_family_selection;
    let latest = app.latest_test_session().map_or_else(
        || "No Testing session has run.".into(),
        |session| {
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
            format!(
                "Latest session: {}\nStatus: {status}\nExit: {}\nStructured results: {}\n{}",
                session.id.0,
                session
                    .exit_code
                    .map_or_else(|| "unavailable".into(), |code| code.to_string()),
                session.result_paths.len(),
                session
                    .error_detail
                    .as_deref()
                    .unwrap_or("No error detail.")
            )
        },
    );
    format!(
        "Family: {}\nAuthority: {}\nMACHINE: {}\nDISTRO: {}\nImage: {}\nTask: {}\n\n{}",
        family.label(),
        test_family_capability(app, family),
        app.workspace
            .variables
            .get("MACHINE")
            .map_or("unavailable", String::as_str),
        app.workspace
            .variables
            .get("DISTRO")
            .map_or("unavailable", String::as_str),
        app.build.target.as_deref().unwrap_or("unavailable"),
        family.task().map_or("selftest executable", |task| task),
        latest,
    )
}

pub(crate) fn testing_result_inspector(app: &App) -> String {
    let Some(record) = app.selected_test_result() else {
        return format!(
            "Resulttool: {}\n\nSelect an exact structured result.",
            resulttool_capability_label(&app.result_tool_capability)
        );
    };
    let counts = record.counts();
    let metadata = record
        .metadata
        .iter()
        .map(|entry| format!("{}={}", entry.key, entry.value))
        .collect::<Vec<_>>()
        .join("\n");
    let limitations = if record.limitations.is_empty() {
        "none".into()
    } else {
        record.limitations.join("\n")
    };
    let case = app.selected_test_case().map_or_else(String::new, |case| {
        let metadata = case
            .metadata
            .iter()
            .map(|entry| format!("{}={}", entry.key, entry.value))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "\n\nCase: {}/{}\nStatus: {:?}\nDuration: {}\nRelated log: {}\n{}",
            case.identity.suite,
            case.identity.case,
            case.outcome,
            case.duration
                .map(format_duration)
                .unwrap_or_else(|| "unavailable".into()),
            case.log_path
                .as_deref()
                .map_or_else(|| "unavailable".into(), |path| path.display().to_string()),
            metadata,
        )
    });
    format!(
        "Exact path:\n{}\nFingerprint: {}\nBytes: {}\nModified: {}\nFamily: {}\nMachine: {}\nImage: {}\nRevision: {}\nCounts P/F/S/E/U: {}/{}/{}/{}/{}\nDuration: {}\nOriginating session: {}\n\nMetadata:\n{}\n\nLimitations:\n{}{}",
        record.identity.path.display(),
        record.identity.fingerprint,
        record.identity.byte_size,
        timestamp_text(record.identity.modified_at),
        record.family.map_or("unknown", |family| family.label()),
        record.machine.as_deref().unwrap_or("unavailable"),
        record.image.as_deref().unwrap_or("unavailable"),
        record.revision.as_deref().unwrap_or("unavailable"),
        counts.passed,
        counts.failed,
        counts.skipped,
        counts.errors,
        counts.unknown,
        record
            .duration
            .map(format_duration)
            .unwrap_or_else(|| "unavailable".into()),
        record
            .originating_session
            .map_or_else(|| "unavailable".into(), |id| id.0.to_string()),
        if metadata.is_empty() {
            "unavailable"
        } else {
            &metadata
        },
        limitations,
        case,
    )
}

pub(crate) fn testing_comparison_inspector(app: &App) -> String {
    let export = match &app.test_junit_export {
        TestJunitExportState::NotStarted => "not started".into(),
        TestJunitExportState::Inspecting { destination, .. } => {
            format!("validating {}", destination.display())
        }
        TestJunitExportState::Ready(preview) => {
            format!("ready: {}", preview.request.destination.display())
        }
        TestJunitExportState::Running(request) => {
            format!("running: {}", request.destination.display())
        }
        TestJunitExportState::Succeeded(request) => {
            format!("succeeded: {}", request.destination.display())
        }
        TestJunitExportState::Failed { request, message } => {
            format!("failed {}: {message}", request.destination.display())
        }
        TestJunitExportState::Cancelled(request) => {
            format!("cancelled: {}", request.destination.display())
        }
        TestJunitExportState::TimedOut(request) => {
            format!("timed out: {}", request.destination.display())
        }
        TestJunitExportState::Lost { request, message } => {
            format!("lost {}: {message}", request.destination.display())
        }
    };
    app.selected_test_transition().map_or_else(
        || format!("Select an exact comparison transition.\n\nJUnit export: {export}"),
        |transition| {
            format!(
                "Case: {}/{}\nCategory: {}\nBaseline: {}\nCandidate: {}\nBaseline log: {}\nCandidate log: {}\n\nJUnit export: {}",
                transition.identity.suite,
                transition.identity.case,
                comparison_category_label(transition.category),
                transition
                    .baseline
                    .map_or_else(|| "absent".into(), |value| format!("{value:?}")),
                transition
                    .candidate
                    .map_or_else(|| "absent".into(), |value| format!("{value:?}")),
                transition.baseline_log.as_deref().map_or_else(
                    || "unavailable".into(),
                    |path| path.display().to_string()
                ),
                transition.candidate_log.as_deref().map_or_else(
                    || "unavailable".into(),
                    |path| path.display().to_string()
                ),
                export,
            )
        },
    )
}

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

pub(crate) fn test_result_import_dialog(
    frame: &mut Frame,
    app: &App,
    dialog: &yoctui_model::TestResultImportDialog,
    area: Rect,
) {
    let validation = dialog.validation_error.as_ref().map_or_else(
        || "✓ Validation: only the exact selected root is scanned within bounded limits.".into(),
        |error| format!("✕ Validation: {error}"),
    );
    testing_popup(
        frame,
        app,
        area,
        "Import structured test results",
        DialogTone::Standard,
        format!(
            "Normalized absolute testresults.json file or retained directory:\n{}_\n\n{}\nEnter imports; Esc cancels.",
            dialog.input, validation,
        ),
        10,
    );
}

pub(crate) fn toml_popup_editor(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    title: &str,
    editor: &yoctui_model::PopupEditor,
    validation_error: Option<&str>,
) {
    let popup = dialog_popup_rect(area, 110, area.height.saturating_sub(4).min(34));
    clear_popup(frame, app, popup);
    let mode = textarea_mode_label(editor.mode());
    let content = popup_editor_text(editor);
    let block = dialog_block(app, format!("{title} — {mode}"), DialogTone::Standard);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    let diagnostic = textarea_diagnostic(editor, validation_error);
    let rows = Layout::vertical(if diagnostic.is_some() {
        vec![
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(2),
            Constraint::Length(3),
        ]
    } else {
        vec![
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(3),
        ]
    })
    .split(inner);
    frame.render_widget(
        Paragraph::new(textarea_status(editor)).style(dialog_styles(app).hint),
        rows[0],
    );
    frame.render_widget(Paragraph::new(content).wrap(Wrap { trim: false }), rows[1]);
    let shortcuts = if let Some(diagnostic) = diagnostic {
        frame.render_widget(
            Paragraph::new(
                dialog_shell(app, title, DialogTone::Standard)
                    .validation(Some(diagnostic.as_str())),
            )
            .wrap(Wrap { trim: false }),
            rows[2],
        );
        rows[3]
    } else {
        rows[2]
    };
    let shell = dialog_shell(app, title, DialogTone::Standard);
    frame.render_widget(
        Paragraph::new(Text::from(vec![
            shell.controls(
                Some(("Enter", "Save/preview")),
                &[("Esc", "Normal"), ("q", "Close")],
            ),
            Line::styled(
                "i insert  v visual  e change value  u undo  r redo  h/j/k/l move",
                dialog_styles(app).hint,
            ),
            Line::styled(
                "Ctrl+C copy  Ctrl+V paste  Home/End line  b/w word  PgUp/PgDn page",
                dialog_styles(app).hint,
            ),
        ]))
        .wrap(Wrap { trim: false }),
        shortcuts,
    );
}

pub(crate) fn popup_editor_text(editor: &yoctui_model::PopupEditor) -> String {
    let mut raw = String::with_capacity(editor.text.len() + 7);
    let selection = editor.selection;
    for (index, character) in editor.text.char_indices() {
        let empty_selection = selection.is_some_and(|(start, end)| start == index && end == index);
        if selection.is_some_and(|(start, _)| start == index) {
            raw.push('⟦');
        }
        if editor.cursor == index {
            raw.push('▏');
        }
        if empty_selection {
            raw.push('⟧');
        }
        raw.push(character);
        let next = index + character.len_utf8();
        if selection.is_some_and(|(start, end)| start < end && end == next) {
            raw.push('⟧');
        }
    }
    let empty_selection_at_end = selection
        .is_some_and(|(start, end)| start == editor.text.len() && end == editor.text.len());
    if selection.is_some_and(|(start, _)| start == editor.text.len()) {
        raw.push('⟦');
    }
    if editor.cursor == editor.text.len() {
        raw.push('▏');
    }
    if empty_selection_at_end
        || selection.is_some_and(|(start, end)| start < end && end == editor.text.len())
    {
        raw.push('⟧');
    }
    let line_count = editor.line_count();
    let number_width = line_count.to_string().len();
    raw.split('\n')
        .enumerate()
        .map(|(line, text)| {
            let diagnostic = editor.validation().iter().any(|span| {
                editor.text[..span.start]
                    .bytes()
                    .filter(|byte| *byte == b'\n')
                    .count()
                    == line
            });
            format!(
                "{:>number_width$} │ {}{}",
                line + 1,
                text,
                if diagnostic { "  [validation]" } else { "" }
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn textarea_mode_label(mode: yoctui_model::TextAreaMode) -> &'static str {
    match mode {
        yoctui_model::TextAreaMode::Normal => "NORMAL",
        yoctui_model::TextAreaMode::Insert => "INSERT",
        yoctui_model::TextAreaMode::Visual => "VISUAL",
    }
}

pub fn checkbox_text(row: &yoctui_model::CheckboxState, unicode: bool) -> String {
    let focus = if row.focused { ">" } else { " " };
    let reason = row
        .disabled_reason
        .as_deref()
        .map_or(String::new(), |reason| format!(" — {reason}"));
    format!(
        "{focus} {} {} ({}){reason}",
        row.marker(unicode),
        row.label,
        row.semantic_state()
    )
}

pub(crate) fn textarea_status(editor: &yoctui_model::PopupEditor) -> String {
    let position = editor.position();
    let save = match editor.save_state() {
        yoctui_model::TextAreaSaveState::Clean { .. } => "clean",
        yoctui_model::TextAreaSaveState::Modified { .. } => "modified",
        yoctui_model::TextAreaSaveState::Preview { .. } => "diff preview",
        yoctui_model::TextAreaSaveState::Conflict { .. } => "external conflict",
        yoctui_model::TextAreaSaveState::Saving { .. } => "saving atomically",
        yoctui_model::TextAreaSaveState::Saved { .. } => "saved",
        yoctui_model::TextAreaSaveState::Failed {
            recoverable: true, ..
        } => "save failed · retry available",
        yoctui_model::TextAreaSaveState::Failed { .. } => "save failed",
    };
    let wrap = editor
        .layout()
        .wrap_width
        .map_or_else(|| "wrap off".to_owned(), |width| format!("wrap {width}"));
    let search = if editor.search_state().query.is_empty() {
        String::new()
    } else {
        format!(
            " · find {}/{}",
            editor.search_state().selected.map_or(0, |index| index + 1),
            editor.search_state().matches.len()
        )
    };
    format!(
        "{} · Ln {}, Col {} · UTF-8 · {} · {} lines · {}{}",
        textarea_mode_label(editor.mode()),
        position.line + 1,
        position.column + 1,
        save,
        editor.line_count(),
        wrap,
        search
    )
}

pub(crate) fn textarea_diagnostic(
    editor: &yoctui_model::PopupEditor,
    legacy: Option<&str>,
) -> Option<String> {
    if let Some(message) = legacy {
        return Some(format!("ERROR: {message}"));
    }
    if let Some(span) = editor.validation().first() {
        return Some(format!(
            "{:?} bytes {}..{}: {}",
            span.severity, span.start, span.end, span.message
        ));
    }
    match editor.save_state() {
        yoctui_model::TextAreaSaveState::Conflict { .. } => {
            Some("CONFLICT: file changed externally; review before saving".into())
        }
        yoctui_model::TextAreaSaveState::Failed { message, .. } => {
            Some(format!("SAVE FAILED: {message}"))
        }
        _ => None,
    }
}

pub(crate) fn test_comparison_dialog(
    frame: &mut Frame,
    app: &App,
    picker: &yoctui_model::TestComparisonPicker,
    area: Rect,
) {
    let rows = app
        .test_results
        .records()
        .iter()
        .map(|record| {
            format!(
                "{} {} [{}]",
                if picker.cursor.as_ref() == Some(&record.identity) {
                    "▶"
                } else {
                    " "
                },
                record.identity.path.display(),
                record.identity.fingerprint,
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let validation = picker.validation_error.as_ref().map_or_else(
        || "✓ Validation: baseline and candidate must be distinct.".into(),
        |error| format!("✕ Validation: {error}"),
    );
    testing_popup(
        frame,
        app,
        area,
        "Choose exact comparison inputs",
        DialogTone::Standard,
        format!(
            "Active field: {:?}\nBaseline: {}\nCandidate: {}\n\n{}\n\n{}\nTab field | ↑/↓ choose | Enter set | p preview | Esc cancel",
            picker.active_field,
            picker.baseline.as_ref().map_or_else(
                || "unavailable".into(),
                |value| value.path.display().to_string()
            ),
            picker.candidate.as_ref().map_or_else(
                || "unavailable".into(),
                |value| value.path.display().to_string()
            ),
            rows,
            validation,
        ),
        20,
    );
}

pub(crate) fn test_comparison_confirmation(
    frame: &mut Frame,
    app: &App,
    preview: &yoctui_model::TestComparisonPreview,
    area: Rect,
) {
    testing_popup(
        frame,
        app,
        area,
        "Confirm result comparison",
        DialogTone::Confirmation,
        format!(
            "Baseline:\n{}\nfingerprint: {}\n\nCandidate:\n{}\nfingerprint: {}\n\nExact indexed shell-free argv:\n{}\n\nEnter compares; Esc cancels.",
            preview.request.baseline.path.display(),
            preview.request.baseline.fingerprint,
            preview.request.candidate.path.display(),
            preview.request.candidate.fingerprint,
            preview
                .argv
                .iter()
                .enumerate()
                .map(|(index, value)| format!("[{index}] {}", value.display()))
                .collect::<Vec<_>>()
                .join("\n"),
        ),
        22,
    );
}

pub(crate) fn test_junit_dialog(
    frame: &mut Frame,
    app: &App,
    dialog: &yoctui_model::TestJunitExportDialog,
    area: Rect,
) {
    let validation = dialog.validation_error.as_ref().map_or_else(
        || {
            "✓ Validation: destination must not exist and its canonical parent must remain unchanged."
                .into()
        },
        |error| format!("✕ Validation: {error}"),
    );
    testing_popup(
        frame,
        app,
        area,
        "JUnit export destination",
        DialogTone::Standard,
        format!(
            "Result:\n{}\nfingerprint: {}\n\nNew absolute .xml destination:\n{}_\n\n{}\nEnter validates; Esc cancels.",
            dialog.result.path.display(),
            dialog.result.fingerprint,
            dialog.destination_input,
            validation,
        ),
        14,
    );
}

pub(crate) fn test_junit_confirmation(
    frame: &mut Frame,
    app: &App,
    preview: &yoctui_model::TestJunitExportPreview,
    area: Rect,
) {
    testing_popup(
        frame,
        app,
        area,
        "Confirm JUnit export",
        DialogTone::Confirmation,
        format!(
            "Result:\n{}\nfingerprint: {}\nDestination:\n{}\n\nExact indexed shell-free argv:\n{}\n\nThis never overwrites. Enter exports; Esc cancels.",
            preview.request.result.path.display(),
            preview.request.result.fingerprint,
            preview.request.destination.display(),
            preview
                .argv
                .iter()
                .enumerate()
                .map(|(index, value)| format!("[{index}] {}", value.display()))
                .collect::<Vec<_>>()
                .join("\n"),
        ),
        19,
    );
}
