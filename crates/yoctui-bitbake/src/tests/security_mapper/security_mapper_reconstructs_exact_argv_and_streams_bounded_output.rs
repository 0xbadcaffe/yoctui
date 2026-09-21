use super::*;

#[tokio::test]
async fn security_mapper_reconstructs_exact_argv_and_streams_bounded_output() {
    let directory = TestDirectory::new("output");
    let preview = preview(&directory);
    let SecurityOperation::PackageMap { executable, .. } = &preview.operation else {
        unreachable!();
    };
    write_executable(
        executable,
        &format!(
            "#!/bin/sh\nprintf 'arg=%s\\n' \"$1\"\nprintf 'stderr\\n' >&2\nprintf '{}\\n'\nprintf '\\377\\n'\n",
            "x".repeat(MAX_SECURITY_MAPPER_LINE_BYTES + 8)
        ),
    );
    let command = SecurityMapperCommandSpec::from_preview(&preview).unwrap();
    assert_eq!(command.arguments().len(), 1);
    let mut runner = SecurityMapperJobRunner::new();
    runner.start(command).await.unwrap();
    assert_eq!(
        runner.next_event().await.unwrap(),
        SecurityMapperRunnerEvent::Started {
            id: SecuritySessionId(7)
        }
    );
    let mut saw_argument = false;
    let mut saw_stderr = false;
    let mut saw_truncated = false;
    let mut saw_invalid_utf8 = false;
    loop {
        match runner.next_event().await.unwrap() {
            SecurityMapperRunnerEvent::Output {
                id,
                stream,
                line,
                truncated,
            } => {
                assert_eq!(id, SecuritySessionId(7));
                saw_argument |= line == format!("arg={}", preview.report_roots[0].display());
                saw_stderr |= stream == SecurityOutputStream::Stderr;
                saw_truncated |= truncated;
                saw_invalid_utf8 |= line.contains('\u{fffd}');
            }
            SecurityMapperRunnerEvent::Completed { id, exit_code } => {
                assert_eq!(id, SecuritySessionId(7));
                assert_eq!(exit_code, Some(0));
                break;
            }
            event => panic!("unexpected event: {event:?}"),
        }
    }
    assert!(saw_argument && saw_stderr && saw_truncated && saw_invalid_utf8);
}
