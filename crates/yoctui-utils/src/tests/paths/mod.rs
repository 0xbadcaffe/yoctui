use super::*;

mod xdg_overrides_require_absolute_paths_and_preserve_non_ascii_paths;

mod path_entry_detection_distinguishes_missing_and_present_entries;

#[cfg(unix)]
mod path_entry_detection_includes_dangling_symbolic_links;
