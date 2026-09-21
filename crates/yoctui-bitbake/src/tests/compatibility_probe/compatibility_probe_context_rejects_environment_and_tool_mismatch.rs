use super::*;

#[test]
fn compatibility_probe_context_rejects_environment_and_tool_mismatch() {
    let fixture = Fixture::new("#!/bin/sh\necho ok\n");
    let mut identity = fixture.context().environment().clone();
    identity.build_directory = AuthoritativeValue::detected(
        "/other/build".into(),
        IdentityAuthority::InitializedEnvironment,
    );
    let result = CapabilityProbeContext::new(
        identity,
        fixture.root.clone(),
        BTreeMap::from([(CapabilityToolId::Devtool, fixture.tool.clone())]),
        BTreeMap::new(),
        Some(BTreeSet::new()),
        Some(BTreeSet::new()),
        Some(BTreeSet::new()),
        Some(BTreeSet::new()),
        Some(BTreeSet::new()),
        Some(BTreeSet::new()),
    );
    assert!(matches!(
        result,
        Err(CapabilityProbeContextError::EnvironmentMismatch)
    ));

    let mut identity = fixture.context().environment().clone();
    identity.available_tools = AuthoritativeValue::detected(
        vec![ToolIdentity {
            id: "devtool".into(),
            executable: "/other/devtool".into(),
            version: None,
        }],
        IdentityAuthority::ExecutableProbe,
    );
    let result = CapabilityProbeContext::new(
        identity,
        fixture.root.clone(),
        BTreeMap::from([(CapabilityToolId::Devtool, fixture.tool.clone())]),
        BTreeMap::new(),
        Some(BTreeSet::new()),
        Some(BTreeSet::new()),
        Some(BTreeSet::new()),
        Some(BTreeSet::new()),
        Some(BTreeSet::new()),
        Some(BTreeSet::new()),
    );
    assert!(matches!(
        result,
        Err(CapabilityProbeContextError::EnvironmentMismatch)
    ));
}
