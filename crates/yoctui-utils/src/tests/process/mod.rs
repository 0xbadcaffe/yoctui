use super::*;

#[cfg(unix)]
mod process_group_guard_terminates_owned_child_on_drop;

mod priority_rejects_values_outside_the_portable_nice_range;

#[cfg(unix)]
mod spawned_background_process_receives_requested_priority;
