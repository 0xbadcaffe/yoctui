use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::OpenYoctoUtility(command) => {
            crate::open_dialog(app, Dialog::YoctoUtility(YoctoUtilityDialog::new(command)));
        }
        Action::SelectYoctoUtilityField { delta } => {
            if let Some(Dialog::YoctoUtility(dialog)) = app.active_dialog_mut() {
                dialog.select_field(delta);
            }
        }
        Action::CycleYoctoUtilityChoice { delta } => {
            if let Some(Dialog::YoctoUtility(dialog)) = app.active_dialog_mut() {
                dialog.cycle_choice(delta);
            }
        }
        Action::AppendYoctoUtilityField(character) => {
            if let Some(Dialog::YoctoUtility(dialog)) = app.active_dialog_mut() {
                dialog.append(character);
            }
        }
        Action::BackspaceYoctoUtilityField => {
            if let Some(Dialog::YoctoUtility(dialog)) = app.active_dialog_mut() {
                dialog.backspace();
            }
        }
        Action::ClearYoctoUtilityField => {
            if let Some(Dialog::YoctoUtility(dialog)) = app.active_dialog_mut() {
                dialog.clear();
            }
        }
        Action::ReviewYoctoUtility => review(app),
        Action::CancelYoctoUtility => crate::close_dialog(app),
        _ => unreachable!("action routed to the wrong reducer"),
    }
    crate::synchronize_focus(app);
    None
}

fn review(app: &mut App) {
    let Some(Dialog::YoctoUtility(dialog)) = app.active_dialog().cloned() else {
        return;
    };
    let arguments = match dialog.arguments() {
        Ok(arguments) => arguments,
        Err(message) => {
            set_error(app, message);
            return;
        }
    };
    let request = match terminal_request(app, &dialog, arguments) {
        Ok(request) => request,
        Err(message) => {
            set_error(app, message);
            return;
        }
    };
    crate::replace_dialog(
        app,
        Dialog::TerminalLaunch(TerminalLaunchDialog {
            request,
            destination: TerminalLaunchDestination::Embedded,
            output_must_not_exist: None,
        }),
    );
}

fn set_error(app: &mut App, message: String) {
    if let Some(Dialog::YoctoUtility(active)) = app.active_dialog_mut() {
        active.validation_error = Some(message);
    }
}

fn terminal_request(
    app: &App,
    dialog: &YoctoUtilityDialog,
    arguments: Vec<String>,
) -> Result<TerminalLaunchRequest, String> {
    let authority = app
        .workspace_compatibility
        .authority()
        .ok_or_else(|| "Current daemon compatibility authority is unavailable".to_owned())?;
    let capability = dialog.capability();
    let record = authority
        .snapshot
        .capability(capability)
        .ok_or_else(|| format!("{} capability is unavailable", capability.as_str()))?;
    if !record.state.is_enabled() {
        return Err(record
            .state
            .reason()
            .map(|reason| reason.message.clone())
            .unwrap_or_else(|| format!("{} is unavailable", capability.as_str())));
    }
    if !authority.implementations.contains_key(&capability) {
        return Err(format!(
            "{} has no selected implementation",
            capability.as_str()
        ));
    }
    let program = authority
        .snapshot
        .environment
        .available_tools
        .value()
        .and_then(|tools| tools.iter().find(|tool| tool.id == dialog.command.tool()))
        .map(|tool| tool.executable.clone())
        .ok_or_else(|| format!("{} executable is unavailable", dialog.command.tool()))?;
    let cwd = authority
        .snapshot
        .environment
        .build_directory
        .value()
        .cloned()
        .ok_or_else(|| "Initialized build directory is unavailable".to_owned())?;
    Ok(TerminalLaunchRequest {
        name: dialog.command.label().into(),
        kind: TerminalCreationKind::Utility,
        cwd,
        program,
        arguments,
    })
}
