use super::*;

#[test]
fn qa_check_workflow_capability_catalog_never_guesses_tasks() {
    assert!(
        QaScope::new(RecipeIdentity {
            name: "bad/name".into(),
            file: "/provider.bb".into(),
        })
        .is_err()
    );
    assert!(
        QaCheckCapability::new(
            QaCheckId::new("missing".into()).unwrap(),
            QaCheckFamily::Patch,
            "Patch".into(),
            scope("busybox"),
            None,
            vec![],
            QaCheckAvailability::Available,
            vec![],
        )
        .is_err()
    );
    assert!(
        QaReportRequest::new(
            1,
            vec![PathBuf::from("/reports"), PathBuf::from("../escape")]
        )
        .is_err()
    );
    let mut state = QaState::default();
    load(&mut state);
    assert_eq!(state.visible_checks().len(), 3);
    let _ = update_qa(&mut state, QaAction::SelectCheck(1));
    let transition = update_qa(&mut state, QaAction::BeginSelectedCheck);
    assert_eq!(
        transition.notification.as_deref(),
        Some("task not reported")
    );
    assert!(state.sessions.is_empty());

    let mut invalid = capability();
    invalid.checks[0].task = Some("guessed/task".into());
    let transition = update_qa(&mut state, QaAction::CapabilityLoaded(invalid));
    assert_eq!(
        transition.notification.as_deref(),
        Some("QA capability response is invalid.")
    );
    assert!(matches!(state.capability, QaCapability::Failed(_)));

    let mut partial = QaState::default();
    let _ = update_qa(
        &mut partial,
        QaAction::CapabilityPartial {
            snapshot: capability(),
            limitations: vec!["license metadata unavailable".into()],
        },
    );
    assert!(matches!(partial.capability, QaCapability::Partial { .. }));
}
