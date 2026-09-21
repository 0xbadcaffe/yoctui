use super::*;

#[test]
fn maintenance_service_diagnostics_keep_location_reachability_and_process_evidence_typed() {
    let fixture = TestDirectory::new("diagnostics");
    prepare_fixture(&fixture, Some("#!/bin/sh\nexit 0\n"));
    process(&fixture.join("proc"), 11, "bitbake-prserv");
    process(&fixture.join("proc"), 12, "bitbake-hashserv");
    process(&fixture.join("proc"), 13, "bitbake-worker");
    let mut input = fixture_input(
        &fixture,
        Some("localhost:8585".into()),
        Some("auto".into()),
        Some("hash.example.invalid:8686".into()),
    );
    input.endpoint_observations.push(
        MaintenanceEndpointObservation::new(
            "localhost:8585".into(),
            ServiceReachability::Reachable,
        )
        .unwrap(),
    );
    let inspection = MaintenanceServiceCapabilityInspector::inspect(input).unwrap();

    let pr = service(&inspection, ServiceKind::Pr);
    assert_eq!(pr.state, ServiceState::Reachable);
    assert_eq!(pr.endpoints[0].location, ServiceLocation::Local);
    assert_eq!(pr.endpoints[0].reachability, ServiceReachability::Reachable);
    assert_eq!(pr.process_evidence[0].pid, 11);
    assert!(
        pr.limitations
            .iter()
            .any(|line| line.contains("does not prove"))
    );

    let hash = service(&inspection, ServiceKind::Hash);
    assert_eq!(hash.state, ServiceState::Partial);
    assert_eq!(hash.endpoints.len(), 2);
    assert_eq!(hash.endpoints[0].location, ServiceLocation::Local);
    assert_eq!(hash.endpoints[1].location, ServiceLocation::Remote);
    assert_eq!(
        hash.endpoints[1].reachability,
        ServiceReachability::NotProbed
    );
    assert_eq!(
        service(&inspection, ServiceKind::Worker).process_evidence[0].pid,
        13
    );
}
