use super::*;

#[test]
fn compatibility_api_authorizes_bridge_getvar_from_metadata_authority() {
    let snapshot = authority(
        10,
        "2.18",
        &[
            (CapabilityId::BitBakeGetVar, "bitbake_getvar.argv"),
            (
                CapabilityId::BitBakeRecipeMetadata,
                "tinfoil.recipe_metadata",
            ),
        ],
    );
    let mut api = BitBakeApiAuthority::new(snapshot, 10, Path::new("/work/build")).unwrap();
    let handshake = api.bridge_handshake();
    assert!(handshake.capabilities.iter().any(|capability| {
        capability.id == CapabilityId::BitBakeGetVar.as_str()
            && capability.implementation == "tinfoil.getvar"
    }));

    negotiate_all(&mut api);
    assert!(api.require(BitBakeApiOperation::Variable).is_ok());
}
