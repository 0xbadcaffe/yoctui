use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::BackspaceSdkPublishTomlEditor => {
            if let Some(Dialog::SdkPublishTomlEditor(editor)) = app.active_dialog_mut()
                && editor.editing
            {
                editor.backspace();
            }
        }
        Action::AppendSdkPublishDestination(character) => {
            if let Some(Dialog::SdkPublish(draft)) = app.active_dialog_mut()
                && !character.is_control()
                && draft.destination.len() < 4_096
            {
                draft.destination.push(character);
            }
        }
        Action::BackspaceSdkPublishDestination => {
            if let Some(Dialog::SdkPublish(draft)) = app.active_dialog_mut() {
                draft.destination.pop();
            }
        }
        Action::PreviewSdkPublish => {
            if let Some(Dialog::SdkPublishTomlEditor(editor)) = app.active_dialog().cloned() {
                let destination = match popup_toml_value(&editor.text, "destination") {
                    Ok(value) => value,
                    Err(message) => {
                        app.notification = Some(message);
                        return None;
                    }
                };
                let Some(artifact) = app.selected_sdk_artifact() else {
                    app.notification = Some("The selected SDK artifact is stale.".into());
                    return None;
                };
                let preview = app
                    .sdk_tool_capability
                    .publish_executable()
                    .map_err(str::to_owned)
                    .and_then(|executable| {
                        SdkPublishPreview::new(
                            executable,
                            artifact.identity.clone(),
                            PathBuf::from(destination),
                        )
                        .map_err(str::to_owned)
                    });
                match preview {
                    Ok(preview) => replace_dialog(app, Dialog::SdkPublishConfirmation(preview)),
                    Err(message) => app.notification = Some(message),
                }
                return None;
            }
            let Some(Dialog::SdkPublish(draft)) = app.active_dialog().cloned() else {
                return None;
            };
            let Some(artifact) = app.selected_sdk_artifact() else {
                app.notification = Some("The selected SDK artifact is stale.".into());
                return None;
            };
            let preview = app
                .sdk_tool_capability
                .publish_executable()
                .map_err(str::to_owned)
                .and_then(|executable| {
                    SdkPublishPreview::new(
                        executable,
                        artifact.identity.clone(),
                        PathBuf::from(draft.destination),
                    )
                    .map_err(str::to_owned)
                });
            match preview {
                Ok(preview) => replace_dialog(app, Dialog::SdkPublishConfirmation(preview)),
                Err(message) => app.notification = Some(message),
            }
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
