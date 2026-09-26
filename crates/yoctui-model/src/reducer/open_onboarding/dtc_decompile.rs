use super::*;

fn request_dtc_directory(app: &mut App, path: PathBuf, initial: bool) -> Option<Effect> {
    app.environment_setup_generation = app.environment_setup_generation.wrapping_add(1);
    let request = app.environment_setup_generation;
    let Some(Dialog::DtcDecompile(dialog)) = app.active_dialog_mut() else {
        return None;
    };
    dialog.error = None;
    let previous = dialog.browser.take().and_then(|browser| browser.directory);
    dialog.browser = Some(EnvironmentBrowser {
        request,
        loading: true,
        directory: previous,
        selection: 0,
    });
    Some(Effect::ReadEnvironmentDirectory {
        request,
        path,
        initial,
    })
}

fn validate_dtc_output(output: &Path) -> Result<(), String> {
    if !yoctui_utils::is_absolute_normal_path(output) {
        return Err("Choose an absolute destination without parent traversal.".into());
    }
    if output.extension().and_then(|value| value.to_str()) != Some("dts") {
        return Err("The decompiled destination must end in .dts.".into());
    }
    let parent = output
        .parent()
        .ok_or_else(|| "The destination has no parent directory.".to_owned())?;
    if !parent.is_dir() {
        return Err(format!(
            "Destination directory {} does not exist.",
            parent.display()
        ));
    }
    Ok(())
}

pub(super) fn reduce(app: &mut App, action: DtcDecompileAction) -> Option<Effect> {
    use DtcDecompileAction as A;
    if matches!(action, A::Browse) {
        let Some(Dialog::DtcDecompile(dialog)) = app.active_dialog() else {
            return None;
        };
        if dialog.editor.is_some() || dialog.browser.is_some() {
            return None;
        }
        let output = dialog.output_path();
        let directory = output.parent().map(PathBuf::from).unwrap_or(output);
        return request_dtc_directory(app, directory, true);
    }
    if matches!(action, A::EnterDirectory | A::ParentDirectory) {
        let Some(Dialog::DtcDecompile(dialog)) = app.active_dialog() else {
            return None;
        };
        let browser = dialog.browser.as_ref().filter(|browser| !browser.loading)?;
        let Some(directory) = &browser.directory else {
            return None;
        };
        let path = if matches!(action, A::ParentDirectory) {
            directory.path.parent().map(PathBuf::from)
        } else {
            directory.children.get(browser.selection).cloned()
        };
        return path.and_then(|path| request_dtc_directory(app, path, false));
    }
    let Some(Dialog::DtcDecompile(dialog)) = app.active_dialog_mut() else {
        return None;
    };
    match action {
        A::Field(delta) if dialog.editor.is_none() && dialog.browser.is_none() => {
            dialog.field = match (dialog.field, delta.is_positive()) {
                (DtcDecompileField::Destination, true) | (DtcDecompileField::ViewAfter, false) => {
                    DtcDecompileField::ViewAfter
                }
                _ => DtcDecompileField::Destination,
            };
        }
        A::Edit if dialog.browser.is_none() && dialog.editor.is_none() => {
            dialog.field = DtcDecompileField::Destination;
            let mut editor = TextAreaState::new(dialog.output.clone());
            editor.selection = Some((0, editor.text.len()));
            dialog.editor = Some(editor);
            dialog.error = None;
        }
        A::Insert(text) => {
            if let Some(editor) = &mut dialog.editor {
                if text.chars().any(char::is_control) {
                    dialog.error = Some("Destination cannot contain control characters.".into());
                } else {
                    editor.insert(&text);
                    dialog.error = None;
                }
            }
        }
        A::Move(motion) => {
            if let Some(editor) = &mut dialog.editor {
                editor.move_cursor(motion);
            }
        }
        A::Backspace => {
            if let Some(editor) = &mut dialog.editor {
                editor.backspace();
            }
        }
        A::Clear => {
            if let Some(editor) = &mut dialog.editor {
                *editor = TextAreaState::new(String::new());
            }
        }
        A::AcceptEdit => {
            if let Some(editor) = dialog.editor.take() {
                dialog.output = editor.text;
                dialog.error = None;
            }
        }
        A::ToggleViewAfter if dialog.editor.is_none() && dialog.browser.is_none() => {
            dialog.view_after = !dialog.view_after;
        }
        A::SelectDirectory(delta) => {
            if let Some(browser) = &mut dialog.browser
                && !browser.loading
            {
                let count = browser.directory.as_ref().map_or(0, |d| d.children.len());
                browser.selection = browser
                    .selection
                    .saturating_add_signed(delta)
                    .min(count.saturating_sub(1));
            }
        }
        A::DirectoryLoaded { request, result } => {
            if let Some(browser) = &mut dialog.browser
                && browser.loading
                && browser.request == request
            {
                browser.loading = false;
                match result {
                    Ok(mut directory) => {
                        directory.children.truncate(ENVIRONMENT_DIRECTORY_LIMIT);
                        browser.directory = Some(directory);
                        dialog.error = None;
                    }
                    Err(error) => dialog.error = Some(error),
                }
            }
        }
        A::ChooseDirectory => {
            if let Some(browser) = &dialog.browser
                && !browser.loading
                && let Some(directory) = &browser.directory
            {
                let filename = dialog
                    .output_path()
                    .file_name()
                    .map(ToOwned::to_owned)
                    .unwrap_or_else(|| "device-tree.yoctui.dts".into());
                dialog.output = directory.path.join(filename).display().to_string();
                dialog.browser = None;
                dialog.error = None;
            }
        }
        A::Review if dialog.editor.is_none() && dialog.browser.is_none() => {
            let output = dialog.output_path();
            if let Err(error) = validate_dtc_output(&output) {
                dialog.error = Some(error);
            } else {
                let launch = dialog.terminal_request();
                if !ensure_output_absent(app, &output) {
                    return None;
                }
                replace_dialog(
                    app,
                    Dialog::TerminalLaunch(TerminalLaunchDialog {
                        request: launch,
                        destination: TerminalLaunchDestination::Embedded,
                        output_must_not_exist: Some(output),
                    }),
                );
            }
        }
        A::Cancel => {
            if dialog.editor.take().is_none() && dialog.browser.take().is_none() {
                close_dialog(app);
            } else {
                dialog.error = None;
            }
        }
        A::Browse | A::EnterDirectory | A::ParentDirectory => {}
        _ => {}
    }
    None
}
