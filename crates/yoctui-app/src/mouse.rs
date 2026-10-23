//! Mouse.

include!("mouse/routing.rs");
include!("mouse/raw_workspace.rs");

include!("mouse/workbench_geometry.rs");

include!("mouse/dialog_and_workspace.rs");

include!("mouse/workspace_and_terminal.rs");

include!("mouse/navigator_and_menu.rs");
include!("mouse/file_diff.rs");
include!("mouse/development.rs");

#[cfg(test)]
#[path = "tests/mouse/mod.rs"]
mod menu_mouse_tests;
