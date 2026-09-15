//! Domain-independent platform, text and time helpers shared across Yoctui.
mod paths;
mod process;
mod text;
mod time;
mod validation;

pub use paths::{config_dir, home_dir, home_path, path_entry_exists, state_dir};
pub use process::{ProcessGroupGuard, is_transient_spawn_error, lower_process_priority};
pub use text::{
    append_truncation_marker, is_csi_final_byte, push_bounded, strip_ansi, truncate_utf8,
    utf8_prefix,
};
pub use time::{format_duration, poll_timeout_ms, unix_ms};
pub use validation::{
    is_absolute_normal_path, is_absolute_normal_path_within, is_bounded_identifier,
    is_bounded_plain_text, push_unique_bounded,
};
