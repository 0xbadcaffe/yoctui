use super::*;

#[test]
fn wic_model_reducer_correlates_creation_inventory_and_lifecycle() {
    let mut app = App::new(20, 20_000);
    app.wic_capability = wic_model_capability();
    let draft = WicCreateDraft {
        machine: "qemux86-64".into(),
        image: "core-image-minimal".into(),
        kickstart: WicKickstartIdentity {
            name: "directdisk".into(),
            path: Some("/layers/meta/wic/directdisk.wks".into()),
        },
        output_directory: "/build/wic-output".into(),
        generate_bmap: true,
        compression: WicCompression::None,
    };
    let preview = draft.preview(&app.wic_capability).unwrap();
    let Some(Effect::StartWicSession { id, operation }) =
        update(&mut app, Action::StartConfirmedWicCreate(preview))
    else {
        panic!("Wic start effect");
    };
    assert!(matches!(operation, WicOperation::Create(_)));
    let background_job_id = app.wic_session(id).unwrap().background_job_id;
    assert_eq!(
        background_job_id,
        BackgroundJobId(WIC_BACKGROUND_JOB_NAMESPACE | id.0)
    );
    assert_ne!(
        background_job_id,
        qemu_background_job_id(QemuSessionId(id.0))
    );
    let _ = update(
        &mut app,
        Action::WicSessionStarting {
            id,
            started_at: SystemTime::UNIX_EPOCH,
        },
    );
    let _ = update(&mut app, Action::WicSessionRunning { id });
    let _ = update(
        &mut app,
        Action::AppendWicSessionOutput {
            id,
            stream: WicOutputStream::Stdout,
            line: "creating".into(),
            truncated: false,
            timestamp: SystemTime::UNIX_EPOCH,
        },
    );
    let output = WicOutput {
        identity: WicOutputIdentity {
            path: "/build/wic-output/image.wic".into(),
            size_bytes: 1024,
            modified_unix_seconds: 1,
        },
        kind: WicOutputKind::Wic,
    };
    let _ = update(
        &mut app,
        Action::CompleteWicSession {
            id,
            exit_code: 0,
            outputs: vec![output.clone()],
            limitations: vec!["dynamic partition size".into()],
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    assert_eq!(
        app.background_jobs
            .get(background_job_id)
            .map(|job| job.status),
        Some(BackgroundJobStatus::Succeeded)
    );
    assert!(matches!(
        &app.wic_outputs,
        WicOutputInventoryState::Partial { outputs, .. } if outputs == &vec![output]
    ));

    let ignored = app.background_jobs.ignored_transitions;
    let _ = update(
        &mut app,
        Action::WicSessionRunning {
            id: WicSessionId(99_999),
        },
    );
    assert_eq!(app.background_jobs.ignored_transitions, ignored + 1);
}
