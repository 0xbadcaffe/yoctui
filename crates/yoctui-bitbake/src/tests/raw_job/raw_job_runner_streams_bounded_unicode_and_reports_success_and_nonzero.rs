use super::*;

#[tokio::test]
async fn raw_job_runner_streams_bounded_unicode_and_reports_success_and_nonzero() {
    let fixture = Fixture::new("outcomes");
    let executable =
        fixture.executable("#!/bin/sh\nprintf 'hello 界\\n'\nprintf 'warning\\n' >&2\nexit 0\n");
    let authority = fixture_authority(&fixture.0, &executable);
    let mut runner = RawJobRunner::new();
    runner.start(spec(&authority)).await.unwrap();
    assert_eq!(
        runner.next_event().await.unwrap(),
        RawJobRunnerEvent::Started
    );
    let mut streams = Vec::new();
    loop {
        match runner.next_event().await.unwrap() {
            RawJobRunnerEvent::Output(chunk) => streams.push((chunk.stream, chunk.text)),
            RawJobRunnerEvent::Completed { exit_code } => {
                assert_eq!(exit_code, 0);
                break;
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }
    assert!(streams.contains(&(RawOutputStream::Stdout, "hello 界".into())));
    assert!(streams.contains(&(RawOutputStream::Stderr, "warning".into())));

    let executable = fixture.executable("#!/bin/sh\nexit 9\n");
    let authority = fixture_authority(&fixture.0, &executable);
    runner.start(spec(&authority)).await.unwrap();
    assert_eq!(
        runner.next_event().await.unwrap(),
        RawJobRunnerEvent::Started
    );
    assert!(matches!(
        runner.next_event().await.unwrap(),
        RawJobRunnerEvent::Failed {
            exit_code: Some(9),
            ..
        }
    ));
}
