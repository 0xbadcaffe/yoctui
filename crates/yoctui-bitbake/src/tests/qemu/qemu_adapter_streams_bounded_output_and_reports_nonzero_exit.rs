use super::*;

#[tokio::test]
async fn qemu_adapter_streams_bounded_output_and_reports_nonzero_exit() {
    let (directory, _, command) = fixture_preview(
        "output",
        "printf 'stdout\\n'; printf 'stderr\\n' >&2; printf '\\377bad\\n'; head -c 70000 /dev/zero | tr '\\000' x; printf '\\n'; exit 7",
    );
    let mut runner = QemuJobRunner::new(directory.clone());
    runner.start(command.clone()).await.unwrap();
    assert_eq!(runner.start(command).await, Err(QemuAdapterError::Busy));
    assert_eq!(
        runner.next_event().await.unwrap(),
        QemuRunnerEvent::Starting
    );
    assert_eq!(runner.next_event().await.unwrap(), QemuRunnerEvent::Started);
    let mut output = Vec::new();
    loop {
        match runner.next_event().await.unwrap() {
            QemuRunnerEvent::Output {
                stream,
                line,
                truncated,
            } => output.push((stream, line, truncated)),
            QemuRunnerEvent::Failed { exit_code, .. } => {
                assert_eq!(exit_code, Some(7));
                break;
            }
            event => panic!("unexpected event: {event:?}"),
        }
    }
    assert!(output.iter().any(|(stream, line, _)| {
        *stream == QemuRunnerOutputStream::Stdout && line == "stdout"
    }));
    assert!(output.iter().any(|(stream, line, _)| {
        *stream == QemuRunnerOutputStream::Stderr && line == "stderr"
    }));
    assert!(output.iter().any(|(_, line, _)| line.contains('\u{fffd}')));
    assert!(
        output
            .iter()
            .any(|(_, line, truncated)| { *truncated && line.len() <= MAX_QEMU_LINE_BYTES })
    );
    fs::remove_dir_all(directory).unwrap();
}
