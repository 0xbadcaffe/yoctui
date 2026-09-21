//! Bounded assembly of one correlated recipe inventory; no partial authority.
use crate::BackendError;
use yoctui_protocol::{MAX_RECIPE_INVENTORY_BYTES, MAX_RECIPE_INVENTORY_RECORDS, RecipeData};

#[derive(Default)]
pub(crate) struct RecipeInventory {
    total: Option<usize>,
    recipes: Vec<RecipeData>,
    record_bytes: usize,
    complete: bool,
}

impl RecipeInventory {
    pub fn push(
        &mut self,
        offset: usize,
        total: usize,
        complete: bool,
        recipes: Vec<RecipeData>,
    ) -> Result<bool, BackendError> {
        let invalid =
            |message: &str| BackendError::Bridge(format!("invalid recipe inventory: {message}"));
        if self.complete
            || offset != self.recipes.len()
            || total > MAX_RECIPE_INVENTORY_RECORDS
            || self.total.is_some_and(|previous| previous != total)
            || recipes.len() > total.saturating_sub(offset)
            || offset > total
        {
            return Err(invalid("offset, total or record limit"));
        }
        let end = offset + recipes.len();
        if complete != (end == total) || (recipes.is_empty() && !complete) {
            return Err(invalid(
                "missing/early completion or empty intermediate chunk",
            ));
        }
        let mut bytes = self.record_bytes;
        for recipe in &recipes {
            bytes = bytes
                .checked_add(
                    serde_json::to_vec(recipe)
                        .map_err(|e| invalid(&e.to_string()))?
                        .len(),
                )
                .ok_or_else(|| invalid("byte count overflow"))?;
        }
        if bytes
            .saturating_add(end.saturating_sub(1))
            .saturating_add(2)
            > MAX_RECIPE_INVENTORY_BYTES
        {
            return Err(invalid("aggregate byte limit"));
        }
        self.total = Some(total);
        self.record_bytes = bytes;
        self.recipes.extend(recipes);
        self.complete = complete;
        Ok(complete)
    }

    pub fn finish(self) -> Result<Vec<RecipeData>, BackendError> {
        if !self.complete {
            return Err(BackendError::Bridge(
                "recipe inventory is incomplete".into(),
            ));
        }
        Ok(self.recipes)
    }
}

#[cfg(test)]
#[path = "tests/recipe_inventory/mod.rs"]
mod tests;
