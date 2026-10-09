//! Mouse ownership is restricted to the visible native Hardware document.
use super::*;
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use yoctui_model::{Action, HardwareAction};

impl HardwareNativeGraphics {
    pub(crate) fn mouse_action(
        &mut self,
        app: &App,
        width: u16,
        height: u16,
        event: MouseEvent,
    ) -> Option<Action> {
        if matches!(event.kind, MouseEventKind::Up(_)) || !app.preferences.mouse_enabled {
            self.drag_origin = None;
            return None;
        }
        let Some(projection) = yoctui_ui::hardware_native_raster_projection(app, width, height)
        else {
            self.drag_origin = None;
            return None;
        };
        if !projection.area.contains((event.column, event.row).into()) {
            self.drag_origin = None;
            return None;
        }
        let action = match event.kind {
            MouseEventKind::ScrollUp if event.modifiers.contains(KeyModifiers::CONTROL) => {
                HardwareAction::Zoom { delta: 25 }
            }
            MouseEventKind::ScrollDown if event.modifiers.contains(KeyModifiers::CONTROL) => {
                HardwareAction::Zoom { delta: -25 }
            }
            MouseEventKind::Down(MouseButton::Left) => {
                self.drag_origin = Some((event.column, event.row));
                return None;
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                let previous = self.drag_origin.replace((event.column, event.row))?;
                HardwareAction::Pan {
                    horizontal: previous.0 as isize - event.column as isize,
                    vertical: previous.1 as isize - event.row as isize,
                }
            }
            _ => return None,
        };
        Some(Action::Hardware(action))
    }
}
