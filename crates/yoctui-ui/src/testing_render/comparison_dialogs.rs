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
