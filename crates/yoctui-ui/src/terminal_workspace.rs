//! Terminal workspace.
use super::*;
use ratatui::widgets::{List, ListItem, ListState};

include!("terminal_workspace/sessions.rs");
include!("terminal_workspace/browser.rs");

include!("terminal_workspace/panes_and_inspector.rs");

include!("terminal_workspace/pane_tree.rs");
