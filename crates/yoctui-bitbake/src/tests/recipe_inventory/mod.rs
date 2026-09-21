use super::*;
use crate::{BitBakeBackend, BridgeBackend};
fn recipe(name: &str) -> RecipeData {
    RecipeData {
        name: name.into(),
        version: None,
        layer: None,
        preferred_version: None,
        file: None,
        append_count: None,
    }
}
mod recipe_inventory_rejects_bad_offsets_totals_final_flags_and_bounds;
mod recipe_inventory_requires_contiguous_complete_bounded_transfer;

mod recipe_inventory_transport_accepts_legacy_and_rejects_incomplete_or_uncorrelated_chunks;
