use super::*;
use ratatui::{Terminal, backend::TestBackend};
use std::time::{Duration, Instant, UNIX_EPOCH};
use yoctui_app::{
    Input, PrefixCommand, PrefixEvent, PrefixState, devtool_reset_confirmation_action,
    focus_action, key_action, logs_action, tasks_action,
};
use yoctui_model::{Action, FocusTarget, FunctionShortcutRoute, Screen as AppScreen};

include!("screen_and_pty.rs");
include!("keyboard_and_navigation.rs");
include!("focus_flow.rs");
