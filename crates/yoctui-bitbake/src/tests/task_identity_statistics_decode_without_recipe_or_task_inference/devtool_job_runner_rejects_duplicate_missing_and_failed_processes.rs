use super::*;

#[cfg(unix)]
#[tokio::test]
async fn devtool_job_runner_rejects_duplicate_missing_and_failed_processes() {
    let (script, command) =
        fake_devtool_command("devtool-runner-failure", "printf 'failed\\n' >&2\nexit 7");
    let mut runner = DevtoolJobRunner::new(std::env::temp_dir());
    runner.start(command.clone()).await.unwrap();
    assert_eq!(runner.start(command).await, Err(DevtoolRunnerError::Busy));
    loop {
        if let DevtoolRunnerEvent::Failed { exit_code } = runner.next_event().await.unwrap() {
            assert_eq!(exit_code, Some(7));
            break;
        }
    }
    fs::remove_file(script).unwrap();

    let missing = fixture_script("missing-devtool-runner");
    let command = DevtoolCommandSpec::with_executable(
        missing.clone(),
        &DevtoolOperation::Reset {
            recipe: "busybox".into(),
        },
        &devtool_compatibility(&std::env::temp_dir(), &missing),
        1,
        &std::env::temp_dir(),
    )
    .unwrap();
    let mut runner = DevtoolJobRunner::new(std::env::temp_dir());
    assert_eq!(
        runner.start(command).await,
        Err(DevtoolRunnerError::MissingExecutable(missing))
    );

    let non_executable = fixture_script("non-executable-devtool-runner");
    fs::write(&non_executable, "#!/bin/sh\nexit 0\n").unwrap();
    let command = DevtoolCommandSpec::with_executable(
        non_executable.clone(),
        &DevtoolOperation::Reset {
            recipe: "busybox".into(),
        },
        &devtool_compatibility(&std::env::temp_dir(), &non_executable),
        1,
        &std::env::temp_dir(),
    )
    .unwrap();
    assert!(matches!(
        runner.start(command).await,
        Err(DevtoolRunnerError::Spawn(_))
    ));
    fs::remove_file(non_executable).unwrap();
}
