//! Domain-independent platform, text and time helpers shared across Yoctui.
mod paths;
mod text;
mod time;

pub use paths::{config_dir, home_dir, home_path, state_dir};
pub use text::{
    append_truncation_marker, is_csi_final_byte, push_bounded, strip_ansi, truncate_utf8,
    utf8_prefix,
};
pub use time::{format_duration, poll_timeout_ms, unix_ms};
