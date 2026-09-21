use super::*;

#[test]
fn compatibility_cache_key_rejects_weak_digest_or_invalid_identity() {
    let mut invalid = key();
    invalid.layer_configuration_digest = "not-a-digest".into();
    assert_eq!(
        invalid.normalize(),
        Err(CapabilityCacheKeyError::InvalidField)
    );

    let mut invalid = key();
    invalid.workspace_identity = "workspace\nother".into();
    assert_eq!(
        invalid.normalize(),
        Err(CapabilityCacheKeyError::InvalidField)
    );
}
