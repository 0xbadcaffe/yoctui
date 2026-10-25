use crate::{DaemonClientSnapshot, daemon_protocol_snapshot};
use yoctui_protocol::daemon::{DaemonEvent, DaemonTelemetry, SequencedEvent};

#[test]
fn resource_telemetry_rejects_gaps_and_invalid_samples_before_installation() {
    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([8; 16]),
        1,
        "boot".into(),
        Default::default(),
    )
    .unwrap();
    let initial = daemon_protocol_snapshot(&state);
    let mut client = DaemonClientSnapshot::default();
    client.replace(initial.clone());
    let mut telemetry = DaemonTelemetry {
        uptime_seconds: 1,
        bitbake: yoctui_protocol::daemon::LifecycleState::Disconnected,
        connected_clients: 1,
        active_jobs: 0,
        pty_sessions: 0,
        queue_depth: 0,
        pressure: Default::default(),
        memory_bytes: None,
        recovery: yoctui_protocol::daemon::DaemonRecoveryState::CleanStart,
        build_resources: Some(Box::new(yoctui_model::BuildResourceTelemetry {
            sampled_unix_ms: 1,
            ..Default::default()
        })),
    };
    let event = |sequence, telemetry| SequencedEvent {
        sequence,
        generation: sequence,
        event: DaemonEvent::Telemetry(telemetry),
    };
    assert!(client.apply_event(&event(2, telemetry.clone())).is_err());
    assert!(client.telemetry.is_none());
    assert_eq!(client.snapshot.as_ref(), Some(&initial));
    client.apply_event(&event(1, telemetry.clone())).unwrap();
    let accepted = client.telemetry.clone();
    telemetry.build_resources.as_mut().unwrap().recipe_peaks = vec![
        yoctui_model::RecipeMemoryPeak {
            recipe: "busybox".into(),
            task: "do_compile".into(),
            sampled_peak_rss_bytes: 100,
        };
        17
    ];
    assert!(client.apply_event(&event(2, telemetry)).is_err());
    assert_eq!(client.telemetry, accepted);
    assert_eq!(client.snapshot.as_ref().unwrap().sequence, 1);
    client.replace(initial);
    assert!(
        client.telemetry.is_none(),
        "workspace replacement must clear old daemon-host samples"
    );
}
