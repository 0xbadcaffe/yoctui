use super::*;

#[tokio::test]
async fn bounded_priority_ingress_drops_only_cosmetic_events() {
    let (reliable_tx, mut reliable_rx) = mpsc::channel(2);
    let (cosmetic_tx, cosmetic_rx) = mpsc::channel(2);
    let pressure = DaemonBitBakePressureShared::default();

    for message in ["ordinary-1", "ordinary-2", "ordinary-dropped"] {
        send_bitbake_event(
            &reliable_tx,
            &cosmetic_tx,
            &pressure,
            None,
            log_event(yoctui_model::Severity::Info, message),
        )
        .await;
    }
    send_bitbake_event(
        &reliable_tx,
        &cosmetic_tx,
        &pressure,
        None,
        log_event(yoctui_model::Severity::Warning, "warning-retained"),
    )
    .await;
    send_bitbake_event(
        &reliable_tx,
        &cosmetic_tx,
        &pressure,
        None,
        DaemonBitBakeEvent::Backend {
            job_id: JobId(1),
            event: Box::new(BackendEvent::BuildCompleted {
                success: false,
                exit_code: Some(1),
            }),
        },
    )
    .await;

    assert_eq!(pressure.cosmetic_dropped.load(Ordering::Relaxed), 1);
    assert_eq!(pressure.reliable_enqueued.load(Ordering::Relaxed), 2);
    assert_eq!(pressure.cosmetic_enqueued.load(Ordering::Relaxed), 2);
    assert_eq!(pressure.maximum_queue_depth.load(Ordering::Relaxed), 4);
    assert!(bitbake_event_is_diagnostic(
        &reliable_rx.try_recv().unwrap()
    ));
    assert!(matches!(
        reliable_rx.try_recv().unwrap(),
        DaemonBitBakeEvent::Backend { event, .. }
            if matches!(event.as_ref(), BackendEvent::BuildCompleted { .. })
    ));
    assert_eq!(cosmetic_rx.len(), 2);
}
