use super::*;

#[tokio::test]
async fn sdk_tool_runner_streams_bounded_output_and_terminal_status() {
    let (directory, adapter) = fixture("runner-output");
    let publish = publish_preview(&adapter, &directory);
    let tool = publish.request.executable.clone();
    executable(
        &tool,
        &format!(
            "#!/bin/sh\nprintf 'stdout\\n'\nprintf 'stderr\\n' >&2\nprintf '{}\\n'\nexit 0\n",
            "x".repeat(MAX_SDK_TOOL_LINE_BYTES + 8)
        ),
    );
    let publish =
        SdkPublishPreview::new(tool, publish.request.artifact, publish.request.destination)
            .unwrap();
    let command = adapter.publication_command(&publish).unwrap();
    let mut runner = SdkToolJobRunner::new();
    runner.start(command).await.unwrap();
    assert_eq!(
        runner.next_event().await.unwrap(),
        SdkToolRunnerEvent::Started
    );
    let mut saw_stdout = false;
    let mut saw_stderr = false;
    let mut saw_truncated = false;
    loop {
        match runner.next_event().await.unwrap() {
            SdkToolRunnerEvent::Output {
                stream, truncated, ..
            } => {
                saw_stdout |= stream == SdkOutputStream::Stdout;
                saw_stderr |= stream == SdkOutputStream::Stderr;
                saw_truncated |= truncated;
            }
            SdkToolRunnerEvent::Completed { exit_code } => {
                assert_eq!(exit_code, Some(0));
                break;
            }
            event => panic!("unexpected runner event: {event:?}"),
        }
    }
    assert!(saw_stdout && saw_stderr && saw_truncated);
}
