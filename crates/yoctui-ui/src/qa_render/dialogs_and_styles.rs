pub(crate) fn qa_dialog(frame: &mut Frame, app: &App, dialog: &QaDialog, area: Rect) {
    let (title, tone, body) = match dialog {
        QaDialog::Operation(preview) => (
            "Confirm recipe/kernel QA",
            DialogTone::Confirmation,
            format!(
                "Operation {}\nCheck: {}\nRecipe: {}\nProvider: {}\n\nIndexed BitBake request:\n{}\n\nReport roots:\n{}\n\nEnter runs | Esc cancels",
                preview.id.0,
                preview.check.0,
                preview.scope.recipe.name,
                preview.scope.recipe.file.display(),
                preview.indexed_arguments.join("\n"),
                preview
                    .report_roots
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
        ),
        QaDialog::LayerOperation(preview) => (
            "Confirm layer QA",
            DialogTone::Confirmation,
            format!(
                "Operation {}\nLayer: {} ({})\nExecutable: {}\n\nIndexed native vector:\n{}\n\nEnter runs | Esc cancels",
                preview.id.0,
                preview.layer.name,
                preview.layer.root.display(),
                preview.executable.path.display(),
                preview.indexed_arguments.join("\n")
            ),
        ),
        QaDialog::Import { .. } => unreachable!("QA import uses the shared editor"),
        QaDialog::Cancellation {
            session,
            background_job,
        } => (
            "Cancel managed QA",
            DialogTone::Confirmation,
            format!(
                "Cancel QA session {} attached to build job {}?\n\nEnter confirms | Esc keeps running",
                session.0, background_job.0
            ),
        ),
        QaDialog::LayerCancellation(session) => (
            "Cancel layer QA",
            DialogTone::Confirmation,
            format!(
                "Cancel exact layer-QA session {}?\n\nEnter confirms | Esc keeps running",
                session.0
            ),
        ),
    };
    let popup = dialog_popup_rect(area, 78, 18);
    clear_popup(frame, app, popup);
    frame.render_widget(
        Paragraph::new(body)
            .block(dialog_block(app, title, tone))
            .wrap(Wrap { trim: false }),
        popup,
    );
}

pub(crate) fn qa_family_label(family: QaCheckFamily) -> &'static str {
    match family {
        QaCheckFamily::KernelConfiguration => "kernel configuration",
        QaCheckFamily::UriFetch => "URI / fetch",
        QaCheckFamily::Patch => "patch",
        QaCheckFamily::License => "license",
        QaCheckFamily::RecipePackage => "recipe / package",
    }
}

pub(crate) fn qa_filter_label(filter: QaStatusFilter) -> &'static str {
    match filter {
        QaStatusFilter::All => "all",
        QaStatusFilter::Failed => "failed",
        QaStatusFilter::Warning => "warning",
        QaStatusFilter::Passed => "passed",
        QaStatusFilter::Skipped => "skipped",
        QaStatusFilter::Unknown => "unknown",
    }
}

pub(crate) fn qa_status_label(status: Option<QaFindingStatus>) -> &'static str {
    match status {
        Some(QaFindingStatus::Passed) => "passed",
        Some(QaFindingStatus::Warning) => "warning",
        Some(QaFindingStatus::Failed) => "failed",
        Some(QaFindingStatus::Skipped) => "skipped",
        Some(QaFindingStatus::Unknown) => "unknown",
        None => "unavailable",
    }
}

pub(crate) fn qa_worst_status(
    values: impl Iterator<Item = QaFindingStatus>,
) -> Option<QaFindingStatus> {
    values.max_by_key(|status| match status {
        QaFindingStatus::Failed => 5,
        QaFindingStatus::Warning => 4,
        QaFindingStatus::Unknown => 3,
        QaFindingStatus::Skipped => 2,
        QaFindingStatus::Passed => 1,
    })
}

pub(crate) fn qa_failure_label(kind: QaReportFailureKind) -> &'static str {
    match kind {
        QaReportFailureKind::Missing => "missing",
        QaReportFailureKind::PermissionDenied => "permission denied",
        QaReportFailureKind::Stale => "stale",
        QaReportFailureKind::Malformed => "malformed",
        QaReportFailureKind::Failed => "failed",
    }
}

pub(crate) fn qa_session_status_label(status: QaSessionStatus) -> &'static str {
    match status {
        QaSessionStatus::Starting => "starting",
        QaSessionStatus::Running => "running",
        QaSessionStatus::Cancelling => "cancelling",
        QaSessionStatus::Succeeded => "succeeded",
        QaSessionStatus::Failed => "failed",
        QaSessionStatus::Cancelled => "cancelled",
        QaSessionStatus::TimedOut => "timed out",
        QaSessionStatus::Lost => "lost",
    }
}

pub(crate) fn qa_status_style(palette: &ThemePalette, status: Option<QaFindingStatus>) -> Style {
    match status {
        Some(QaFindingStatus::Passed) => palette.role(palette.success, Modifier::BOLD),
        Some(QaFindingStatus::Warning | QaFindingStatus::Skipped) => {
            security_warning_style(palette)
        }
        Some(QaFindingStatus::Failed) => security_error_style(palette),
        Some(QaFindingStatus::Unknown) | None => Style::default(),
    }
}

pub(crate) fn qa_session_style(palette: &ThemePalette, status: QaSessionStatus) -> Style {
    match status {
        QaSessionStatus::Succeeded => palette.role(palette.success, Modifier::BOLD),
        QaSessionStatus::Failed | QaSessionStatus::TimedOut | QaSessionStatus::Lost => {
            security_error_style(palette)
        }
        QaSessionStatus::Cancelled | QaSessionStatus::Cancelling => security_warning_style(palette),
        QaSessionStatus::Starting | QaSessionStatus::Running => security_info_style(palette),
    }
}
