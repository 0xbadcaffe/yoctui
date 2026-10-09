use super::*;

impl InteractiveRuntime {
    pub(super) fn route_hardware_workspace(&mut self, input: Input) -> bool {
        let runtime = self;
        if !hardware_route_owns_input(&runtime.app, input) {
            return false;
        }
        let Some(action) = yoctui_app::hardware_workspace_action(&runtime.app, input) else {
            return false;
        };
        match compatibility_workspace_action(&mut runtime.app, action) {
            Some(effect @ Effect::Hardware(yoctui_model::HardwareEffect::Browse { .. }))
            | Some(effect @ Effect::Hardware(yoctui_model::HardwareEffect::Load(_)))
            | Some(effect @ Effect::Hardware(yoctui_model::HardwareEffect::Project(_)))
            | Some(effect @ Effect::Hardware(yoctui_model::HardwareEffect::OpenDesktop { .. }))
            | Some(effect @ Effect::Hardware(yoctui_model::HardwareEffect::LoadProject { .. })) => {
                runtime.hardware_io.submit(effect);
            }
            Some(Effect::Hardware(yoctui_model::HardwareEffect::Persist(_))) => {
                if let Err(error) = persist_hardware(
                    runtime.session_path.as_deref(),
                    &mut runtime.session,
                    &runtime.app,
                ) {
                    runtime.app.notification =
                        Some(format!("Could not save the Hardware library: {error}"));
                }
            }
            _ => {}
        }

        true
    }
}

fn hardware_route_owns_input(app: &App, input: Input) -> bool {
    app.screen == Screen::Hardware
        && (app.focus == yoctui_model::FocusTarget::Workspace
            || (matches!(input, Input::Esc | Input::Backspace) && app.hardware.viewer.is_some()))
}

#[cfg(test)]
#[path = "../tests/hardware_workspace.rs"]
mod tests;
