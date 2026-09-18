use super::*;

#[test]
fn tokio_runtime_two_workers_isolate_a_bounded_blocking_poll() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_time()
        .build()
        .expect("two-worker Tokio runtime");

    runtime.block_on(async {
        let (poll_started_tx, poll_started_rx) = tokio::sync::oneshot::channel();
        let bounded_poll = tokio::spawn(async move {
            let _ = poll_started_tx.send(());
            std::thread::sleep(Duration::from_millis(750));
        });
        poll_started_rx.await.expect("bounded poll started");

        let (reactor_tx, reactor_rx) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            let _ = reactor_tx.send(());
        });
        tokio::time::timeout(Duration::from_millis(500), reactor_rx)
            .await
            .expect("second worker kept the reactor responsive")
            .expect("reactor response sender remained alive");
        bounded_poll.await.expect("bounded poll task joined");
    });
}
