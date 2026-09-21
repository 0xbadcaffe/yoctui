use super::*;

#[tokio::test]
async fn recipe_inventory_transport_accepts_legacy_and_rejects_incomplete_or_uncorrelated_chunks() {
    let root = std::env::temp_dir().join(format!("yoctui-recipe-chunks-{}", std::process::id()));
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
        let mut authority =
            crate::compatibility_fixtures::release_capability_fixtures()[1].command_authority(1);
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
        authority.snapshot.environment.build_directory = yoctui_model::AuthoritativeValue::detected(
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
