use super::*;

#[tokio::test]
async fn raw_job_runner_truncates_oversized_lines_and_revalidates_at_start() {
    let fixture = Fixture::new("bounds");
    let executable = fixture.executable(&format!(
        "#!/bin/sh\nprintf '%s\\n' '{}'\n",
        "界".repeat(yoctui_model::MAX_RAW_OUTPUT_CHUNK_BYTES)
    ));
    let authority = fixture_authority(&fixture.0, &executable);
    let mut runner = RawJobRunner::new();
    runner.start(spec(&authority)).await.unwrap();
    runner.next_event().await.unwrap();
    let RawJobRunnerEvent::Output(chunk) = runner.next_event().await.unwrap() else {
        panic!("expected bounded output");
    };
    assert!(chunk.text.len() <= yoctui_model::MAX_RAW_OUTPUT_CHUNK_BYTES);
    assert!(chunk.truncated_bytes > 0);

    let command = spec(&authority);
    fs::remove_file(executable).unwrap();
    let mut denied = RawJobRunner::new();
    assert!(matches!(
        denied.start(command).await,
        Err(RawJobRunnerError::Authorization(_))
    ));
    assert!(!denied.is_active());

    let executable = fixture.executable("not an executable image\n");
    let authority = fixture_authority(&fixture.0, &executable);
    let mut rejected = RawJobRunner::new();
    assert!(matches!(
        rejected.start(spec(&authority)).await,
        Err(RawJobRunnerError::Spawn(_))
    ));
    assert!(!rejected.is_active());
}
