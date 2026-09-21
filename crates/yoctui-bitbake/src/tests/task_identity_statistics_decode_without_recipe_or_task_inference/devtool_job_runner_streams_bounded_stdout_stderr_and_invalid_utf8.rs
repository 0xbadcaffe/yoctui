use super::*;

#[cfg(unix)]
#[tokio::test]
async fn devtool_job_runner_streams_bounded_stdout_stderr_and_invalid_utf8() {
    let (script, command) = fake_devtool_command(
        "devtool-runner-output",
        "printf 'stdout line\\n'\nprintf 'stderr line\\n' >&2\nprintf '\\377bad\\n'\nhead -c 70000 /dev/zero | tr '\\000' x\nprintf '\\n'",
    );
    let mut runner = DevtoolJobRunner::new(std::env::temp_dir());
    runner.start(command).await.unwrap();
    assert_eq!(
        runner.next_event().await.unwrap(),
        DevtoolRunnerEvent::Started
    );
    let mut output = Vec::new();
    loop {
        match runner.next_event().await.unwrap() {
            DevtoolRunnerEvent::Output {
                stream,
                line,
                truncated,
            } => output.push((stream, line, truncated)),
            DevtoolRunnerEvent::Completed { exit_code } => {
                assert_eq!(exit_code, Some(0));
                break;
            }
            event => panic!("unexpected runner event: {event:?}"),
        }
    }
    assert!(output.iter().any(|(stream, line, _)| {
        *stream == DevtoolOutputStream::Stdout && line == "stdout line"
    }));
    assert!(output.iter().any(|(stream, line, _)| {
        *stream == DevtoolOutputStream::Stderr && line == "stderr line"
    }));
    assert!(output.iter().any(|(_, line, _)| line.contains('\u{fffd}')));
    let (_, truncated_line, truncated) = output
        .iter()
        .find(|(_, _, truncated)| *truncated)
        .expect("oversized output was not marked truncated");
    assert!(*truncated);
    assert!(truncated_line.len() <= MAX_DEVTOOL_LINE_BYTES);
    fs::remove_file(script).unwrap();
}
