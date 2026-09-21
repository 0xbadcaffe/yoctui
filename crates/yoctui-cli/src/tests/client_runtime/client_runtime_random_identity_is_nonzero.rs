use super::*;

#[test]
fn client_runtime_random_identity_is_nonzero() {
    assert_ne!(random_client_id().unwrap().0, [0; 16]);
}
