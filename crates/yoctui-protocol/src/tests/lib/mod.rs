use super::*;

mod recipe_inventory_chunking_is_opt_in_and_legacy_commands_still_decode;
use proptest::prelude::*;
mod config_metadata_round_trips_and_old_payloads_default_safely;
mod dependency_graph_round_trips_and_legacy_dependencies_remain_compatible;
mod recipe_bitbake_action_force_flag_round_trips_and_defaults_false;
mod recipe_metadata_round_trips_and_old_recipe_payloads_default_safely;
mod rejects_sequence;
mod round_trip;
mod typed_event_workspace_round_trips_without_untyped_json;
mod unknown_event_is_safe;

mod frames_partial_lines_without_losing_data;

mod oversized_partial_line_is_rejected_and_cleared;

mod hardening_stress_protocol_preserves_large_ordered_irregular_stream;

proptest! {
    #[test]
    fn framing_is_independent_of_chunk_boundaries(parts in proptest::collection::vec("[a-z]{0,12}", 0..30), chunk_sizes in proptest::collection::vec(1usize..16, 1..30)) {
        let source = parts.iter().map(|part| format!("{part}\n")).collect::<String>().into_bytes();
        let mut framer = LineFramer::default();
        let mut frames = Vec::new();
        let mut offset = 0;
        for size in chunk_sizes {
            if offset == source.len() { break; }
            let end = (offset + size).min(source.len());
            frames.extend(framer.push(&source[offset..end]).unwrap());
            offset = end;
        }
        if offset < source.len() { frames.extend(framer.push(&source[offset..]).unwrap()); }
        prop_assert_eq!(frames, parts.into_iter().map(String::into_bytes).collect::<Vec<_>>());
        prop_assert_eq!(framer.pending_len(), 0);
    }
}
