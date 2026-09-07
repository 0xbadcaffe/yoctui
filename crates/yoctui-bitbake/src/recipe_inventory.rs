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
mod tests {
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
    #[test]
    fn recipe_inventory_requires_contiguous_complete_bounded_transfer() {
        let mut inventory = RecipeInventory::default();
        assert!(!inventory.push(0, 2, false, vec![recipe("α")]).unwrap());
        assert!(inventory.push(1, 2, true, vec![recipe("β")]).unwrap());
        assert!(inventory.push(2, 2, true, vec![]).is_err());
        assert_eq!(inventory.finish().unwrap(), vec![recipe("α"), recipe("β")]);
        assert!(RecipeInventory::default().finish().is_err());
        let mut empty = RecipeInventory::default();
        assert!(empty.push(0, 0, true, vec![]).unwrap());
        assert!(empty.finish().unwrap().is_empty());
    }
    #[test]
    fn recipe_inventory_rejects_bad_offsets_totals_final_flags_and_bounds() {
        for (offset, total, complete, rows) in [
            (1, 2, true, vec![recipe("x")]),
            (0, 2, true, vec![recipe("x")]),
            (0, 1, false, vec![recipe("x")]),
            (0, 1, false, vec![]),
            (
                0,
                MAX_RECIPE_INVENTORY_RECORDS + 1,
                false,
                vec![recipe("x")],
            ),
            (
                0,
                1,
                true,
                vec![recipe(&"x".repeat(MAX_RECIPE_INVENTORY_BYTES))],
            ),
        ] {
            assert!(
                RecipeInventory::default()
                    .push(offset, total, complete, rows)
                    .is_err()
            );
        }
        let mut changed = RecipeInventory::default();
        changed.push(0, 3, false, vec![recipe("x")]).unwrap();
        assert!(changed.push(1, 2, true, vec![recipe("y")]).is_err());
        let mut duplicate = RecipeInventory::default();
        duplicate.push(0, 2, false, vec![recipe("x")]).unwrap();
        assert!(duplicate.push(0, 2, false, vec![recipe("x")]).is_err());
    }

    #[tokio::test]
    async fn recipe_inventory_transport_accepts_legacy_and_rejects_incomplete_or_uncorrelated_chunks()
     {
        let root =
            std::env::temp_dir().join(format!("yoctui-recipe-chunks-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let script = root.join("bridge.py");
        std::fs::write(&script, r#"import json, os, sys
seq = 0
def emit(message, correlation):
    global seq
    seq += 1
    print(json.dumps({'protocol_version':1,'sequence':seq,'correlation_id':correlation,'message':message}),flush=True)
hello = json.loads(sys.stdin.readline())
auth = hello['message']['compatibility']
emit({'type':'hello_ack','bitbake_version':'fixture','compatibility_generation':auth['generation'],'capabilities':[x['id'] for x in auth['capabilities']]},hello['correlation_id'])
request = json.loads(sys.stdin.readline())
assert request['message']['chunked'] is True
correlation = request['correlation_id']
mode = os.environ['INVENTORY_MODE']
row = {'name':'fixture','version':None,'layer':None,'file':None}
if mode == 'legacy':
    emit({'type':'recipes','recipes':[row,row]},correlation)
else:
    if mode == 'wrong': correlation = 'stale-request'
    if mode == 'oversize': row['name'] = 'x' * 600000
    emit({'type':'recipes_chunk','offset':0,'total':2,'complete':mode=='early','recipes':[row]},correlation)
    if mode != 'truncated':
        emit({'type':'recipes_chunk','offset':0 if mode=='offset' else 1,'total':2,'complete':True,'recipes':[row]},correlation)
"#).unwrap();
        for mode in [
            "legacy",
            "chunks",
            "wrong",
            "truncated",
            "offset",
            "early",
            "oversize",
        ] {
            let mut authority = crate::compatibility_fixtures::release_capability_fixtures()[1]
                .command_authority(1);
            let id = yoctui_model::CapabilityId::BitBakeRecipeInventory;
            let record = authority
                .snapshot
                .capabilities
                .iter_mut()
                .find(|r| r.id == id)
                .unwrap();
            record.state = yoctui_model::CapabilityState::Available;
            record.evidence = vec![yoctui_model::CapabilityEvidence {
                kind: yoctui_model::CapabilityEvidenceKind::BackendNegotiation,
                outcome: yoctui_model::CapabilityEvidenceOutcome::Positive,
                subject: id.as_str().into(),
                detail: "Explicit fixture recipe API".into(),
                argv: vec![],
            }];
            authority.implementations.insert(
                id,
                yoctui_model::CapabilityImplementation {
                    id: "tinfoil.recipes".into(),
                    kind: yoctui_model::CapabilityImplementationKind::BackendApi,
                },
            );
            authority.snapshot.environment.build_directory =
                yoctui_model::AuthoritativeValue::detected(
                    root.clone(),
                    yoctui_model::IdentityAuthority::InitializedEnvironment,
                );
            let mut bridge = BridgeBackend::spawn_with_compatibility(
                "python3",
                script.clone(),
                root.clone(),
                std::collections::BTreeMap::from([("INVENTORY_MODE".into(), mode.into())]),
                authority,
                1,
            )
            .await
            .unwrap();
            let result = bridge.list_recipes(None).await;
            if matches!(mode, "legacy" | "chunks") {
                assert_eq!(result.unwrap().len(), 2, "{mode}");
            } else {
                assert!(result.is_err(), "{mode}");
            }
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
