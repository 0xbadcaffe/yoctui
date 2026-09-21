pub(crate) fn security_dialog(frame: &mut Frame, app: &App, dialog: &SecurityDialog, area: Rect) {
    let (title, text, height) = match dialog {
        SecurityDialog::Operation(preview) => {
            let roots = display_security_paths(&preview.report_roots);
            (
                format!("Confirm {}", security_operation_label(&preview.operation)),
                format!(
                    "Session: {}\nScope: {}\n\nExact indexed shell-free operation:\n{}\n\nAuthoritative report roots:\n{}\n\nEnter starts; Esc cancels.",
                    preview.id.0,
                    security_scope_text(Some(&preview.scope)),
                    preview.indexed_arguments.join("\n"),
                    roots,
                ),
                18,
            )
        }
        SecurityDialog::Import { .. } => unreachable!("Security import uses the shared editor"),
        SecurityDialog::Cancellation(id) => (
            "Confirm Security cancellation".into(),
            format!(
                "Cancel Security session {} only?\n\nEnter requests cancellation; Esc keeps it running.",
                id.0
            ),
            7,
        ),
    };
    let popup = dialog_popup_rect(area, 94, height);
    clear_popup(frame, app, popup);
    frame.render_widget(
        Paragraph::new(text)
            .block(dialog_block(app, title, DialogTone::Confirmation))
            .wrap(Wrap { trim: false }),
        popup,
    );
}

pub(crate) fn security_operation_label(operation: &SecurityOperation) -> &'static str {
    match operation {
        SecurityOperation::CveCheck(_) => "CVE check",
        SecurityOperation::SbomBuild(_) => "SBOM generation",
        SecurityOperation::PackageMap { .. } => "CVE package mapping",
    }
}

pub(crate) fn security_filter_label(filter: yoctui_model::CveStatusFilter) -> &'static str {
    match filter {
        yoctui_model::CveStatusFilter::All => "all",
        yoctui_model::CveStatusFilter::Vulnerable => "vulnerable",
        yoctui_model::CveStatusFilter::Patched => "patched",
        yoctui_model::CveStatusFilter::Ignored => "ignored",
        yoctui_model::CveStatusFilter::NotAffected => "not affected",
        yoctui_model::CveStatusFilter::Unknown => "unknown",
    }
}

pub(crate) fn security_cve_status_label(status: yoctui_model::CveStatus) -> &'static str {
    match status {
        yoctui_model::CveStatus::Vulnerable => "vulnerable",
        yoctui_model::CveStatus::Patched => "patched",
        yoctui_model::CveStatus::Ignored => "ignored",
        yoctui_model::CveStatus::NotAffected => "not affected",
        yoctui_model::CveStatus::Unknown => "unknown",
    }
}

pub(crate) fn security_spdx_kind_label(kind: SpdxArtifactKind) -> &'static str {
    match kind {
        SpdxArtifactKind::Json => "JSON",
        SpdxArtifactKind::Archive => "archive",
    }
}

pub(crate) fn security_session_status_label(status: SecuritySessionStatus) -> &'static str {
    match status {
        SecuritySessionStatus::Starting => "starting",
        SecuritySessionStatus::Running => "running",
        SecuritySessionStatus::Cancelling => "cancelling",
        SecuritySessionStatus::Succeeded => "succeeded",
        SecuritySessionStatus::Failed => "failed",
        SecuritySessionStatus::Cancelled => "cancelled",
        SecuritySessionStatus::TimedOut => "timed out",
        SecuritySessionStatus::Lost => "lost",
    }
}

pub(crate) fn security_info_style(palette: &ThemePalette) -> Style {
    palette.role(palette.informational, Modifier::ITALIC)
}

pub(crate) fn security_warning_style(palette: &ThemePalette) -> Style {
    palette.role(palette.warning, Modifier::BOLD)
}

pub(crate) fn security_error_style(palette: &ThemePalette) -> Style {
    palette.role(palette.error, Modifier::BOLD | Modifier::UNDERLINED)
}

pub(crate) fn security_cve_status_style(
    palette: &ThemePalette,
    status: yoctui_model::CveStatus,
) -> Style {
    match status {
        yoctui_model::CveStatus::Vulnerable => security_error_style(palette),
        yoctui_model::CveStatus::Patched | yoctui_model::CveStatus::NotAffected => {
            palette.role(palette.success, Modifier::BOLD)
        }
        yoctui_model::CveStatus::Ignored | yoctui_model::CveStatus::Unknown => {
            security_warning_style(palette)
        }
    }
}

pub(crate) fn security_session_style(
    palette: &ThemePalette,
    status: SecuritySessionStatus,
) -> Style {
    match status {
        SecuritySessionStatus::Succeeded => palette.role(palette.success, Modifier::BOLD),
        SecuritySessionStatus::Failed | SecuritySessionStatus::Lost => {
            security_error_style(palette)
        }
        SecuritySessionStatus::Cancelled | SecuritySessionStatus::TimedOut => {
            security_warning_style(palette)
        }
        SecuritySessionStatus::Starting
        | SecuritySessionStatus::Running
        | SecuritySessionStatus::Cancelling => palette.role(palette.progress, Modifier::BOLD),
    }
}
