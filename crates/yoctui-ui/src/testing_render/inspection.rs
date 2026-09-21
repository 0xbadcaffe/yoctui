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
