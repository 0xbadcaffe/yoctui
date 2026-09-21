use super::*;
use std::{fs, os::unix::fs::PermissionsExt};

fn profile(root: &std::path::Path) -> BuildEnvironmentProfile {
    BuildEnvironmentProfile {
        source_dir: root.join("poky"),
        build_dir: root.join("build"),
        init_script: root.join("poky/oe-init-build-env"),
    }
}

mod fresh_clone_initialization_creates_reviewed_build_directory;

mod fresh_clone_initialization_rejects_files_links_and_missing_parents;

mod initializes_child_environment_without_mutating_parent;

mod reports_interactive_setup_instead_of_answering_prompts;

mod poky_clone_requires_empty_destination_and_uses_reviewed_vectors;

mod fresh_clone_creates_nested_parents_only_on_execution;

#[cfg(unix)]
mod fresh_clone_preview_rejects_dangling_destination;

#[cfg(unix)]
mod poky_clone_retries_only_text_file_busy_spawn_errors;
