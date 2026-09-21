use super::*;

#[test]
fn qa_workflow_maps_layer_capability_and_runner_events_mechanically() {
    let layer =
        yoctui_model::QaLayerIdentity::new("meta-demo".into(), "/layers/meta-demo".into()).unwrap();
    let configured = yoctui_model::QaConfiguredLayerCapability::new(
        yoctui_model::QaCheckId::new("layer-meta-demo".into()).unwrap(),
        layer.clone(),
        vec!["walnascar".into()],
        yoctui_model::QaLayerRunCapability::Disabled("tool unavailable".into()),
        vec!["tool unavailable".into()],
    )
    .unwrap();
    let snapshot = yoctui_model::QaLayerCapabilitySnapshot::new(
        Some("6.0".into()),
        "/build".into(),
        layer,
        vec![configured],
        vec!["tool unavailable".into()],
    )
    .unwrap();
    assert_eq!(
        qa_layer_capability_action(QaLayerCapabilityResponse::Partial(snapshot.clone())),
        Action::Qa(QaAction::LayerCapabilityPartial {
            snapshot,
            limitations: vec!["tool unavailable".into()],
        })
    );

    let timestamp = SystemTime::UNIX_EPOCH;
    let session = yoctui_model::QaLayerSessionId(7);
    assert_eq!(
        qa_layer_runner_action(QaLayerRunnerEvent::Started { id: session }, timestamp),
        Some(Action::Qa(QaAction::LayerSessionRunning(session)))
    );
    assert_eq!(
        qa_layer_runner_action(
            QaLayerRunnerEvent::Output {
                id: session,
                stream: yoctui_model::QaOutputStream::Stderr,
                line: "warning".into(),
                truncated: false,
            },
            timestamp,
        ),
        Some(Action::Qa(QaAction::LayerSessionOutput {
            session,
            stream: yoctui_model::QaOutputStream::Stderr,
            line: "warning".into(),
            truncated: false,
        }))
    );
    assert_eq!(
        qa_layer_runner_action(
            QaLayerRunnerEvent::TimedOut {
                id: session,
                forced: true,
                exit_code: None,
            },
            timestamp,
        ),
        Some(Action::Qa(QaAction::TimeoutLayerSession {
            session,
            forced: true,
            exit_code: None,
            finished_at: timestamp,
        }))
    );
    assert_eq!(
        qa_layer_runner_action(
            QaLayerRunnerEvent::Lost {
                id: session,
                message: "channel lost".into(),
            },
            timestamp,
        ),
        Some(Action::Qa(QaAction::LoseLayerSession {
            session,
            message: "channel lost".into(),
            finished_at: timestamp,
        }))
    );
}
