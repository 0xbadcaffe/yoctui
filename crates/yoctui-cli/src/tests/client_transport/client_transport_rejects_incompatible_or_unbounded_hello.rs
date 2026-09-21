use super::*;

#[test]
fn client_transport_rejects_incompatible_or_unbounded_hello() {
    for minor in [
        ProtocolVersion::CURRENT.minor - 1,
        ProtocolVersion::CURRENT.minor + 1,
    ] {
        let mut invalid = hello(DaemonInstanceId([1; 16]));
        invalid.selected_version.minor = minor;
        assert!(validate_hello(&invalid, &requested_capabilities()).is_err());
    }
    assert!(matches!(
        validate_client(ClientId([0; 16]), "client"),
        Err(ClientTransportError::InvalidClientIdentity)
    ));
    let mut invalid = hello(DaemonInstanceId([1; 16]));
    invalid.limits.maximum_frame_bytes = (MAX_FRAME_BYTES as u32).saturating_add(1);
    assert!(matches!(
        validate_hello(&invalid, &requested_capabilities()),
        Err(ClientTransportError::InvalidLimits)
    ));
    invalid = hello(DaemonInstanceId([1; 16]));
    invalid
        .capabilities
        .retain(|item| *item != Capability::IncrementalEvents);
    assert!(matches!(
        validate_hello(&invalid, &requested_capabilities()),
        Err(ClientTransportError::MissingCapability(
            Capability::IncrementalEvents
        ))
    ));
}
