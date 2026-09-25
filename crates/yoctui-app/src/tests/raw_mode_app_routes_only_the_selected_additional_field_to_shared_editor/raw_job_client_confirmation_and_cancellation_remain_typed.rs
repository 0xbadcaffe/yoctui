use super::*;

#[test]
fn raw_job_client_confirmation_and_cancellation_remain_typed() {
    let mut preview_state = yoctui_model::RawModeState::new(yoctui_model::builtin_raw_catalog());
    preview_state.view = yoctui_model::RawModeView::Preview;
    assert_eq!(
        raw_mode_input(&preview_state, Input::Enter),
        Some(yoctui_model::RawModeAction::ConfirmPreview)
    );

    let state = raw_execution_state_fixture();
    let wire = raw_execution_request_to_protocol(&state.request).unwrap();
    assert_eq!(wire.request_id, state.request.id.as_str());
    assert_eq!(wire.command_id, state.request.command.as_str());
    assert_eq!(wire.preview_digest, state.request.preview_digest.to_hex());

    let request_id = state.request.id.clone();
    let mut app = yoctui_model::App::new(16, 4096);
    app.raw_mode
        .execution_states
        .insert(request_id.clone(), state);
    assert_eq!(
        yoctui_model::update(
            &mut app,
            yoctui_model::Action::RawMode(yoctui_model::RawModeAction::CancelExecution(
                request_id.clone()
            ),),
        ),
        Some(yoctui_model::Effect::CancelRaw(request_id))
    );
}

#[test]
fn repeated_raw_command_waits_for_the_new_request_identity() {
    let command = yoctui_model::builtin_raw_catalog()
        .command(&yoctui_model::RawCommandId::new("section-01-version-and-help.l0052").unwrap())
        .unwrap();
    let mut authority = compatibility_workspace_authority(7);
    authority
        .snapshot
        .capabilities
        .push(yoctui_model::CapabilityRecord {
            id: yoctui_model::CapabilityId::BitBakeRawCli,
            state: yoctui_model::CapabilityState::Available,
            evidence: vec![yoctui_model::CapabilityEvidence {
                kind: yoctui_model::CapabilityEvidenceKind::DirectProbe,
                outcome: yoctui_model::CapabilityEvidenceOutcome::Positive,
                subject: "bitbake --help".into(),
                detail: "Raw repeat regression fixture.".into(),
                argv: vec!["bitbake".into(), "--help".into()],
            }],
        });
    authority.implementations.insert(
        yoctui_model::CapabilityId::BitBakeRawCli,
        yoctui_model::CapabilityImplementation {
            id: "bitbake.raw.argv".into(),
            kind: yoctui_model::CapabilityImplementationKind::Command,
        },
    );
    let authority = authority.normalize().unwrap();

    let mut app = yoctui_model::App::new(16, 4096);
    yoctui_model::install_workspace_compatibility(&mut app, authority).unwrap();
    app.raw_mode.view = yoctui_model::RawModeView::Form;
    app.raw_mode.form = Some(yoctui_model::RawCommandForm {
        command: command.id.clone(),
        fields: std::collections::BTreeMap::new(),
        field_order: Vec::new(),
        field_selection: 0,
        additional_arguments: yoctui_model::RawArgvEditor::new("").unwrap(),
        capability_generation: 7,
        build_directory: "/work/poky/build".into(),
    });
    let _ = yoctui_model::update(
        &mut app,
        yoctui_model::Action::RawMode(yoctui_model::RawModeAction::RequestPreview),
    );
    assert_eq!(app.raw_mode.view, yoctui_model::RawModeView::Preview);

    let mut previous = raw_execution_state_fixture();
    previous.request.command = command.id.clone();
    let previous_id = previous.request.id.clone();
    app.raw_mode
        .execution_states
        .insert(previous_id.clone(), previous);
    app.raw_mode.output.request = Some(previous_id.clone());

    let Some(yoctui_model::Effect::StartRaw(request)) = yoctui_model::update(
        &mut app,
        yoctui_model::Action::RawMode(yoctui_model::RawModeAction::ConfirmPreview),
    ) else {
        panic!("confirmation must start a Raw request");
    };
    assert_ne!(request.id, previous_id);
    assert_eq!(app.raw_mode.output.request.as_ref(), Some(&request.id));
    assert!(app.raw_mode.selected_execution().is_none());
}
