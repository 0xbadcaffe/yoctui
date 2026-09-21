pub(crate) fn sdk_popup(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    title: &str,
    tone: DialogTone,
    content: String,
    preferred_height: u16,
) {
    let popup = dialog_popup_rect(area, 92, preferred_height);
    clear_popup(frame, app, popup);
    frame.render_widget(
        Paragraph::new(content)
            .block(dialog_block(app, title, tone))
            .wrap(Wrap { trim: false }),
        popup,
    );
}

pub(crate) fn sdk_build_confirmation(
    frame: &mut Frame,
    app: &App,
    preview: &yoctui_model::SdkBuildPreview,
    area: Rect,
) {
    let action = match preview.action {
        SdkBuildAction::Populate(SdkKind::Standard) => "populate standard SDK",
        SdkBuildAction::Populate(SdkKind::Extensible) => "populate extensible SDK",
        SdkBuildAction::Test(SdkKind::Standard) => "run testsdk",
        SdkBuildAction::Test(SdkKind::Extensible) => "run testsdkext",
    };
    sdk_popup(
        frame,
        app,
        area,
        "Confirm SDK build",
        DialogTone::Confirmation,
        format!(
            "Action: {action}\nMachine: {}\nDistro: {}\nImage target: {}\nBitBake task: {}\n\nEnter starts the managed BitBake build.\nEsc closes without starting.",
            preview.machine,
            preview.distro,
            preview.image,
            preview.request.task.as_deref().unwrap_or("unavailable"),
        ),
        12,
    );
}

pub(crate) fn sdk_publish_dialog(
    frame: &mut Frame,
    app: &App,
    draft: &SdkPublishDraft,
    area: Rect,
) {
    let installer = app.selected_sdk_artifact().map_or_else(
        || "unavailable".into(),
        |artifact| artifact.identity.path.display().to_string(),
    );
    let validation = app
        .sdk_tool_capability
        .publish_executable()
        .and_then(|executable| {
            app.selected_sdk_artifact()
                .ok_or("an installer selection is required")
                .and_then(|artifact| {
                    SdkPublishPreview::new(
                        executable,
                        artifact.identity.clone(),
                        std::path::PathBuf::from(&draft.destination),
                    )
                    .map(|_| ())
                })
        })
        .map_or_else(
            |message| format!("✕ Validation: {message}"),
            |()| "✓ Validation: ready for exact preview".into(),
        );
    sdk_popup(
        frame,
        app,
        area,
        "Publish SDK installer",
        DialogTone::Standard,
        format!(
            "Tool: {}\nInstaller [read-only]: {installer}\nDestination [editing]: {}_\n{validation}\n\nDestination must be an absolute canonical empty directory.\nEnter validates and opens the exact argument preview.\nEsc closes without publishing.",
            app.sdk_tool_capability
                .publish_executable()
                .map_or_else(|_| "unavailable".into(), |path| path.display().to_string()),
            draft.destination,
        ),
        13,
    );
}

pub(crate) fn indexed_path_vector(paths: &[std::path::PathBuf]) -> String {
    paths
        .iter()
        .enumerate()
        .map(|(index, path)| format!("[{index}] {}", path.display()))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn sdk_publish_confirmation(
    frame: &mut Frame,
    app: &App,
    preview: &SdkPublishPreview,
    area: Rect,
) {
    sdk_popup(
        frame,
        app,
        area,
        "Confirm SDK publication",
        DialogTone::Confirmation,
        format!(
            "Installer: {}\nDestination: {}\n\nExact indexed shell-free argument vector:\n{}\n\nNo overwrite policy is guessed.\nEnter starts the managed publication job.\nEsc closes without publishing.",
            preview.request.artifact.path.display(),
            preview.request.destination.display(),
            indexed_path_vector(&preview.argv),
        ),
        18,
    );
}

pub(crate) fn sdk_native_dialog(
    frame: &mut Frame,
    app: &App,
    dialog: &SdkNativeDialog,
    area: Rect,
) {
    let draft = &dialog.draft;
    let arguments = if dialog.arguments_input.is_empty() {
        "none".into()
    } else {
        dialog.arguments_input.clone()
    };
    let validation = app
        .sdk_tool_capability
        .executable_for(draft.mode)
        .and_then(|executable| {
            yoctui_model::SdkNativePreview::new(yoctui_model::SdkNativeRequest {
                executable,
                mode: draft.mode,
                extracted_root: (!draft.extracted_root.is_empty())
                    .then(|| std::path::PathBuf::from(&draft.extracted_root)),
                recipe: draft.recipe.clone(),
                tool: (draft.mode == SdkNativeMode::RunNative).then(|| draft.tool.clone()),
                arguments: draft.arguments.clone(),
            })
            .map(|_| ())
        })
        .map_or_else(
            |message| format!("✕ Validation: {message}"),
            |()| "✓ Validation: ready for exact preview".into(),
        );
    let row = |field: SdkNativeField, label: &str, value: &str| {
        let marker = if dialog.selected_field == field {
            "▶"
        } else {
            " "
        };
        let editing = if dialog.selected_field == field && dialog.editing {
            " [editing]"
        } else {
            ""
        };
        format!("{marker} {label}: {value}{editing}")
    };
    let validation = dialog
        .validation_error
        .as_ref()
        .map_or(validation, |message| format!("✕ Validation: {message}"));
    sdk_popup(
        frame,
        app,
        area,
        "SDK native tool",
        DialogTone::Standard,
        format!(
            "{}\nExecutable: {}\n{}\n{}\n{}\n{}\n{validation}\n\n↑/↓ select · Enter edit/cycle · ←/→ mode · p preview · Esc close",
            row(SdkNativeField::Mode, "Mode", &format!("{:?}", draft.mode)),
            app.sdk_tool_capability
                .executable_for(draft.mode)
                .map_or_else(|_| "unavailable".into(), |path| path.display().to_string()),
            row(
                SdkNativeField::Workspace,
                "Workspace",
                if draft.extracted_root.is_empty() {
                    "active build"
                } else {
                    &draft.extracted_root
                },
            ),
            row(
                SdkNativeField::Recipe,
                "Recipe",
                if draft.recipe.is_empty() {
                    "unavailable"
                } else {
                    &draft.recipe
                },
            ),
            row(
                SdkNativeField::Tool,
                "Tool",
                if draft.mode == SdkNativeMode::FindSysroot {
                    "not applicable"
                } else if draft.tool.is_empty() {
                    "unavailable"
                } else {
                    &draft.tool
                },
            ),
            row(SdkNativeField::Arguments, "Arguments", &arguments),
        ),
        18,
    );
}

pub(crate) fn sdk_native_confirmation(
    frame: &mut Frame,
    app: &App,
    preview: &SdkNativePreview,
    area: Rect,
) {
    sdk_popup(
        frame,
        app,
        area,
        "Confirm SDK native tool",
        DialogTone::Confirmation,
        format!(
            "Mode: {:?}\nWorkspace: {}\nRecipe: {}\nTool: {}\n\nExact indexed shell-free argument vector:\n{}\n\nEnvironment changes apply only to the managed child.\nEnter starts the operation.\nEsc closes without starting.",
            preview.request.mode,
            preview
                .request
                .extracted_root
                .as_ref()
                .map_or("active build".into(), |path| path.display().to_string()),
            preview.request.recipe,
            preview.request.tool.as_deref().unwrap_or("not applicable"),
            indexed_path_vector(&preview.argv),
        ),
        19,
    );
}

pub(crate) fn sdk_cancellation_confirmation(
    frame: &mut Frame,
    app: &App,
    id: SdkSessionId,
    area: Rect,
) {
    let operation = app
        .sdk_session(id)
        .map_or("unavailable", |session| match &session.operation {
            SdkOperation::Publish(_) => "publication",
            SdkOperation::Native(_) => "native tool",
        });
    sdk_popup(
        frame,
        app,
        area,
        "Confirm SDK cancellation",
        DialogTone::Confirmation,
        format!(
            "Cancel managed SDK {operation} operation #{}?\n\nRetained output and the terminal cancellation result remain in SDK history.\nEnter requests cancellation.\nEsc keeps the operation running.",
            id.0
        ),
        9,
    );
}
