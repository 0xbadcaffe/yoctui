use super::*;

#[test]
fn security_workflow_normalizes_reports_and_rejects_stale_generations() {
    let mut state = SecurityState::default();
    let request = SecurityReportRequest::new(1, vec!["/reports".into()]).unwrap();
    state.inventory = SecurityInventoryState::Loading {
        request: request.clone(),
    };
    let report = SecurityReport::Cve(CveReport {
        identity: identity("/reports/cve.json", "abc"),
        scope: Some(SecurityScope::Recipe(recipe())),
        findings: vec![
            finding(CveStatus::Vulnerable),
            finding(CveStatus::Vulnerable),
        ],
        metadata: vec![],
        limitations: vec![],
    });
    let stale = SecurityReportRequest::new(2, vec!["/reports".into()]).unwrap();
    let _ = update_security(
        &mut state,
        SecurityAction::ReportsLoaded {
            request: stale,
            reports: vec![report.clone()],
            limitations: vec![],
        },
    );
    assert!(matches!(
        state.inventory,
        SecurityInventoryState::Loading { .. }
    ));
    let _ = update_security(
        &mut state,
        SecurityAction::ReportsLoaded {
            request,
            reports: vec![report],
            limitations: vec![],
        },
    );
    assert_eq!(state.visible_findings().len(), 1);
    assert!(matches!(
        state.inventory,
        SecurityInventoryState::Available { .. }
    ));
    let _ = update_security(&mut state, SecurityAction::CycleCveFilter);
    assert_eq!(state.cve_filter, CveStatusFilter::Vulnerable);
    let _ = update_security(&mut state, SecurityAction::BeginSearch);
    for character in "CVE-2026".chars() {
        let _ = update_security(&mut state, SecurityAction::AppendQuery(character));
    }
    assert_eq!(state.visible_findings().len(), 1);
}
