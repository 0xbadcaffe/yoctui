use super::*;

#[tokio::test]
async fn security_mapper_reports_duplicate_nonzero_and_worker_loss() {
    let directory = TestDirectory::new("outcomes");
    let preview = preview(&directory);
    let SecurityOperation::PackageMap { executable, .. } = &preview.operation else {
        unreachable!();
    };
    write_executable(executable, "#!/bin/sh\nexit 9\n");
    let command = SecurityMapperCommandSpec::from_preview(&preview).unwrap();
    let mut runner = SecurityMapperJobRunner::new();
    runner.start(command.clone()).await.unwrap();
    assert_eq!(
        runner.start(command.clone()).await,
        Err(SecurityMapperAdapterError::Busy)
    );
    assert!(matches!(
        runner.next_event().await.unwrap(),
        SecurityMapperRunnerEvent::Started {
            id: SecuritySessionId(7)
        }
    ));
    assert!(matches!(
        runner.next_event().await.unwrap(),
        SecurityMapperRunnerEvent::Failed {
            id: SecuritySessionId(7),
            exit_code: Some(9)
        }
    ));

    write_executable(executable, "#!/bin/sh\nsleep 2\n");
    let command = SecurityMapperCommandSpec::from_preview(&preview).unwrap();
    runner.start(command).await.unwrap();
    let _ = runner.next_event().await.unwrap();
    runner.lose_output_channel();
    assert!(matches!(
        runner.next_event().await.unwrap(),
        SecurityMapperRunnerEvent::Lost {
            id: SecuritySessionId(7),
            ..
        }
    ));
}
