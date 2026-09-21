use super::*;
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};
use yoctui_model::{PtyClientId, PtyDimensions, PtySessionId, PtySessionKind};
use yoctui_protocol::daemon::{PtyCommand, PtyKind, TerminalDimensions};

mod gitui_real_terminal_handles_input_resize_and_exit;
mod image_console_wire_specs_preserve_qemu_and_ssh_session_kinds;
mod menuconfig_children_always_receive_a_color_capable_terminal_identity;
mod next_generation_pty_converts_typed_emulator_cells_without_ansi_leakage;
mod pty_control_deadline_outlives_child_termination_deadline;
mod raw_pty_namespace_never_changes_generic_identity_allocation;
mod resource_limits_reject_oversized_pty_dimensions;
mod ux_terminal_adapter_wire_is_sparse_exact_and_never_reparses_ansi;
