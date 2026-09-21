use super::*;

#[test]
fn daemon_listener_wait_wakes_for_additional_readiness_fd() {
    let paths = test_paths("activity-additional-fd");
    let listener = DaemonListener::bind(&paths).unwrap();
    let (mut writer, reader) = UnixStream::pair().unwrap();
    let notifier = thread::spawn(move || {
        thread::sleep(Duration::from_millis(20));
        writer.write_all(&[1]).unwrap();
    });
    let started = Instant::now();

    assert!(
        listener
            .wait_for_activity_with_additional_fd(
                &[],
                Some(reader.as_raw_fd()),
                Duration::from_secs(1),
            )
            .unwrap()
    );
    assert!(started.elapsed() < Duration::from_millis(500));

    notifier.join().unwrap();
    drop(listener);
    cleanup(&paths);
}
