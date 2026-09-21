//! Mouse.

include!("mouse/routing.rs");

include!("mouse/workbench_geometry.rs");

include!("mouse/dialog_and_workspace.rs");

include!("mouse/workspace_and_terminal.rs");

include!("mouse/navigator_and_menu.rs");

#[cfg(test)]
#[path = "tests/mouse/mod.rs"]
mod menu_mouse_tests;
