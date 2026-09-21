fn render_workflow_dialogs(frame: &mut Frame, app: &App, area: Rect) -> bool {
    if let Some(Dialog::RecipeEditor(editor)) = app.active_dialog() {
        recipe_editor(frame, app, editor, area);
        return true;
    } else if let Some(Dialog::ImageConsole(dialog)) = app.active_dialog() {
        image_console_dialog(frame, app, dialog, area);
        return true;
    } else if let Some(Dialog::QemuLaunch(dialog)) = app.active_dialog() {
        qemu_launch_dialog(frame, app, dialog, area);
        return true;
    } else if let Some(Dialog::QemuLaunchConfirmation(preview)) = app.active_dialog() {
        qemu_launch_confirmation(frame, app, preview, area);
        return true;
    } else if let Some(Dialog::QemuCancellationConfirmation(id)) = app.active_dialog() {
        qemu_cancellation_confirmation(frame, app, *id, area);
        return true;
    } else if let Some(Dialog::WicCreateTomlEditor {
        editor,
        validation_error,
    }) = app.active_dialog()
    {
        toml_popup_editor(
            frame,
            app,
            area,
            "Wic create.toml",
            editor,
            validation_error.as_deref(),
        );
        return true;
    } else if let Some(Dialog::WicCreate(dialog)) = app.active_dialog() {
        wic_create_dialog(frame, app, dialog, area);
        return true;
    } else if let Some(Dialog::WicCreateConfirmation(preview)) = app.active_dialog() {
        wic_create_confirmation(frame, app, preview, area);
        return true;
    } else if let Some(Dialog::WicDevicePicker(dialog)) = app.active_dialog() {
        wic_device_picker(frame, app, dialog, area);
        return true;
    } else if let Some(Dialog::WicWritePhrase(dialog)) = app.active_dialog() {
        wic_write_phrase_dialog(frame, app, dialog, area);
        return true;
    } else if let Some(Dialog::WicWriteConfirmation(preview)) = app.active_dialog() {
        wic_write_confirmation(frame, app, preview, area);
        return true;
    } else if let Some(Dialog::WicCancellationConfirmation {
        id,
        incomplete_device_warning,
    }) = app.active_dialog()
    {
        wic_cancellation_confirmation(frame, app, *id, *incomplete_device_warning, area);
        return true;
    } else if let Some(Dialog::SdkBuildConfirmation(preview)) = app.active_dialog() {
        sdk_build_confirmation(frame, app, preview, area);
        return true;
    } else if let Some(Dialog::SdkPublishTomlEditor(editor)) = app.active_dialog() {
        toml_popup_editor(frame, app, area, "SDK publish.toml", editor, None);
        return true;
    } else if let Some(Dialog::SdkPublish(draft)) = app.active_dialog() {
        sdk_publish_dialog(frame, app, draft, area);
        return true;
    } else if let Some(Dialog::SdkPublishConfirmation(preview)) = app.active_dialog() {
        sdk_publish_confirmation(frame, app, preview, area);
        return true;
    } else if let Some(Dialog::SdkNativeTomlEditor(editor)) = app.active_dialog() {
        toml_popup_editor(frame, app, area, "SDK native.toml", editor, None);
        return true;
    } else if let Some(Dialog::SdkNative(draft)) = app.active_dialog() {
        sdk_native_dialog(frame, app, draft, area);
        return true;
    } else if let Some(Dialog::SdkNativeConfirmation(preview)) = app.active_dialog() {
        sdk_native_confirmation(frame, app, preview, area);
        return true;
    } else if let Some(Dialog::SdkCancellationConfirmation(id)) = app.active_dialog() {
        sdk_cancellation_confirmation(frame, app, *id, area);
        return true;
    } else if let Some(Dialog::TestLaunchTomlEditor {
        editor,
        validation_error,
        ..
    }) = app.active_dialog()
    {
        toml_popup_editor(
            frame,
            app,
            area,
            "Test launch.toml",
            editor,
            validation_error.as_deref(),
        );
        return true;
    } else if let Some(Dialog::TestLaunch(dialog)) = app.active_dialog() {
        test_launch_dialog(frame, app, dialog, area);
        return true;
    } else if let Some(Dialog::TestLaunchConfirmation(preview)) = app.active_dialog() {
        test_launch_confirmation(frame, app, preview, area);
        return true;
    } else if let Some(Dialog::TestCancellationConfirmation(id)) = app.active_dialog() {
        test_cancellation_confirmation(frame, app, *id, area);
        return true;
    } else if let Some(Dialog::TestResultImportTomlEditor {
        editor,
        validation_error,
    }) = app.active_dialog()
    {
        toml_popup_editor(
            frame,
            app,
            area,
            "Test result import.toml",
            editor,
            validation_error.as_deref(),
        );
        return true;
    } else if let Some(Dialog::TestResultImport(dialog)) = app.active_dialog() {
        test_result_import_dialog(frame, app, dialog, area);
        return true;
    } else if let Some(Dialog::TestComparisonTomlEditor {
        editor,
        validation_error,
    }) = app.active_dialog()
    {
        toml_popup_editor(
            frame,
            app,
            area,
            "Test comparison.toml",
            editor,
            validation_error.as_deref(),
        );
        return true;
    } else if let Some(Dialog::TestComparison(picker)) = app.active_dialog() {
        test_comparison_dialog(frame, app, picker, area);
        return true;
    } else if let Some(Dialog::TestComparisonConfirmation(preview)) = app.active_dialog() {
        test_comparison_confirmation(frame, app, preview, area);
        return true;
    } else if let Some(Dialog::TestJunitTomlEditor {
        editor,
        validation_error,
        ..
    }) = app.active_dialog()
    {
        toml_popup_editor(
            frame,
            app,
            area,
            "JUnit export.toml",
            editor,
            validation_error.as_deref(),
        );
        return true;
    } else if let Some(Dialog::TestJunitExport(dialog)) = app.active_dialog() {
        test_junit_dialog(frame, app, dialog, area);
        return true;
    } else if let Some(Dialog::TestJunitExportConfirmation(preview)) = app.active_dialog() {
        test_junit_confirmation(frame, app, preview, area);
        return true;
    } else if let Some(Dialog::Security(SecurityDialog::Import {
        editor,
        validation_error,
    })) = app.active_dialog()
    {
        toml_popup_editor(
            frame,
            app,
            area,
            "Security import.toml",
            editor,
            validation_error.as_deref(),
        );
        return true;
    } else if let Some(Dialog::Security(dialog)) = app.active_dialog() {
        security_dialog(frame, app, dialog, area);
        return true;
    } else if let Some(Dialog::Qa(QaDialog::Import {
        editor,
        validation_error,
    })) = app.active_dialog()
    {
        toml_popup_editor(
            frame,
            app,
            area,
            "QA import.toml",
            editor,
            validation_error.as_deref(),
        );
        return true;
    } else if let Some(Dialog::Qa(dialog)) = app.active_dialog() {
        qa_dialog(frame, app, dialog, area);
        return true;
    } else if let Some(Dialog::Maintenance(dialog)) = app.active_dialog()
        && let Some((title, editor, validation_error)) = match dialog.as_ref() {
            MaintenanceDialog::ReadinessToml {
                editor,
                validation_error,
            } => Some(("Sstate readiness.toml", editor, validation_error)),
            MaintenanceDialog::CleanupToml {
                editor,
                validation_error,
            } => Some(("Sstate cleanup.toml", editor, validation_error)),
            MaintenanceDialog::PrServiceToml {
                operation,
                editor,
                validation_error,
            } => Some((
                match operation {
                    yoctui_model::PrServiceOperation::Export => "PR service export.toml",
                    yoctui_model::PrServiceOperation::Import => "PR service import.toml",
                },
                editor,
                validation_error,
            )),
            MaintenanceDialog::LockedCacheToml {
                editor,
                validation_error,
            } => Some(("Locked cache.toml", editor, validation_error)),
            MaintenanceDialog::BuildHistoryToml {
                editor,
                validation_error,
            } => Some(("Build history.toml", editor, validation_error)),
            MaintenanceDialog::GitArchiveToml {
                editor,
                validation_error,
            } => Some(("Git archive.toml", editor, validation_error)),
            _ => None,
        }
    {
        toml_popup_editor(frame, app, area, title, editor, validation_error.as_deref());
        return true;
    } else if let Some(Dialog::Maintenance(dialog)) = app.active_dialog() {
        maintenance_dialog(frame, app, dialog, area);
        return true;
    }
    false
}
