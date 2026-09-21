use super::*;

#[tokio::test]
async fn raw_output_job_attachment_is_ordered_idempotent_and_does_not_cancel() {
    let fixture = Fixture::new(
        "output-attachment",
        "#!/bin/sh\nprintf 'ready\\n'\nsleep 1\nexit 0\n",
    );
    let authority = authority(&fixture);
    let wire = request(&authority, "raw-request:daemon-output-attachment");
    let mut supervisor = DaemonRawSupervisor::default();
    supervisor.replace_compatibility(Some(authority)).unwrap();
    supervisor.start(wire.clone()).unwrap();
    loop {
        if next_event(&mut supervisor).await.state.phase == yoctui_model::RawExecutionPhase::Running
        {
            break;
        }
    }
    assert!(matches!(
        supervisor.set_attachment(&wire.request_id, RawAttachmentState::Detached),
        Ok(DaemonRawAttachment::Job)
    ));
    let detached = loop {
        let event = next_event(&mut supervisor).await;
        if event.state.attachment == RawAttachmentState::Detached {
            break event.state;
        }
    };
    assert_eq!(detached.phase, yoctui_model::RawExecutionPhase::Running);
    assert!(!detached.cancellation_requested);
    assert!(matches!(
        supervisor.set_attachment(&wire.request_id, RawAttachmentState::Detached),
        Err(DaemonRawError::AttachmentUnchanged(_))
    ));
    assert!(matches!(
        supervisor.set_attachment(&wire.request_id, RawAttachmentState::Attached),
        Ok(DaemonRawAttachment::Job)
    ));
    let attached = loop {
        let event = next_event(&mut supervisor).await;
        if event.state.attachment == RawAttachmentState::Attached {
            break event.state;
        }
    };
    assert_eq!(attached.phase, yoctui_model::RawExecutionPhase::Running);
    assert!(!attached.cancellation_requested);
}
