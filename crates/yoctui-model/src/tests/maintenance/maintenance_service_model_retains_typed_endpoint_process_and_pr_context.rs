use super::*;

#[test]
fn maintenance_service_model_retains_typed_endpoint_process_and_pr_context() {
    let endpoint = ServiceEndpointDiagnostic::new(
        ServiceEndpointRole::Primary,
        "localhost:8585".into(),
        ServiceLocation::Local,
        ServiceReachability::Reachable,
        None,
    )
    .unwrap();
    let process = ServiceProcessEvidence::new(42, "bitbake-prserv".into()).unwrap();
    let diagnostic = ServiceDiagnostic::new(
        ServiceKind::Pr,
        ServiceState::Reachable,
        vec![endpoint.clone(), endpoint],
        vec![process.clone(), process],
        vec!["observational only".into(), "observational only".into()],
    )
    .unwrap();
    assert_eq!(diagnostic.endpoints.len(), 1);
    assert_eq!(diagnostic.process_evidence.len(), 1);
    assert_eq!(diagnostic.limitations, vec!["observational only"]);

    let request = PrServiceRequest::new(
        PrServiceOperation::Import,
        "/evidence/pr.inc".into(),
        "/build".into(),
        "localhost:8585".into(),
    )
    .unwrap();
    assert_eq!(request.build_dir, Path::new("/build"));
    assert_eq!(request.endpoint, "localhost:8585");
    assert!(
        PrServiceRequest::new(
            PrServiceOperation::Export,
            "/evidence/pr.txt".into(),
            "/build".into(),
            "localhost:8585".into(),
        )
        .is_err()
    );
}
