use super::*;

#[tokio::test]
async fn wic_adapter_runner_reports_only_new_outputs_and_nonzero_failure() {
    let (directory, output, command) = runner_fixture(
        "runner-success",
        "printf 'before\\n'; printf 'warning\\n' >&2; printf image > \"$6/new.wic\"; exit 0",
    )
    .await;
    fs::write(output.join("existing.wic"), "old").unwrap();
    let mut runner = WicJobRunner::new(directory.clone());
    runner.start(command, output.clone()).await.unwrap();
    assert_eq!(runner.next_event().await.unwrap(), WicRunnerEvent::Starting);
    assert_eq!(runner.next_event().await.unwrap(), WicRunnerEvent::Started);
    let terminal = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let event = runner.next_event().await.unwrap();
            if matches!(event, WicRunnerEvent::Completed { .. }) {
                break event;
            }
        }
    })
    .await
    .unwrap();
    let WicRunnerEvent::Completed { outputs, .. } = terminal else {
        unreachable!()
    };
    assert_eq!(outputs.len(), 1);
    assert!(outputs[0].identity.path.ends_with("new.wic"));
    fs::remove_dir_all(directory).unwrap();

    let (directory, output, command) =
        runner_fixture("runner-failure", "printf failed >&2; exit 9").await;
    let mut runner = WicJobRunner::new(directory.clone());
    runner.start(command, output).await.unwrap();
    let _ = runner.next_event().await.unwrap();
    let _ = runner.next_event().await.unwrap();
    let terminal = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let event = runner.next_event().await.unwrap();
            if matches!(event, WicRunnerEvent::Failed { .. }) {
                break event;
            }
        }
    })
    .await
    .unwrap();
    assert!(matches!(
        terminal,
        WicRunnerEvent::Failed {
            exit_code: Some(9),
            ..
        }
    ));
    fs::remove_dir_all(directory).unwrap();
}
