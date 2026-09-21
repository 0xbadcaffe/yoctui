use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires installed GitUI; run with YOCTUI_GITUI_PROGRAM"]
async fn gitui_real_terminal_handles_input_resize_and_exit() {
    let program = std::env::var("YOCTUI_GITUI_PROGRAM").expect("set YOCTUI_GITUI_PROGRAM");
    let root = std::env::temp_dir().join(format!("yoctui-gitui-smoke-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    for args in [
        vec!["init", "-b", "master"],
        vec!["config", "user.name", "Fixture"],
        vec!["config", "user.email", "fixture@example.invalid"],
    ] {
        assert!(
            std::process::Command::new("git")
                .arg("-C")
                .arg(&root)
                .args(args)
                .output()
                .unwrap()
                .status
                .success()
        );
    }
    std::fs::write(root.join("recipe.bb"), "SUMMARY = \"Example recipe\"\n").unwrap();
    for args in [vec!["add", "."], vec!["commit", "-m", "Initial recipe"]] {
        assert!(
            std::process::Command::new("git")
                .arg("-C")
                .arg(&root)
                .args(args)
                .output()
                .unwrap()
                .status
                .success()
        );
    }
    std::fs::write(root.join("recipe.bb"), "SUMMARY = \"Updated recipe\"\n").unwrap();
    let mut supervisor = DaemonPtySupervisor::default();
    let id = supervisor
        .start_new(
            "GitUI · source".into(),
            PtyKind::Utility,
            root.display().to_string(),
            PtyCommand {
                program,
                arguments: Vec::new(),
                environment_profile_id: None,
            },
            TerminalDimensions {
                columns: 110,
                rows: 32,
            },
        )
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let screen = loop {
        assert!(Instant::now() < deadline, "GitUI did not render");
        while let Some(event) = supervisor.try_event() {
            if let DaemonPtyEvent::Lost { message, .. } = event {
                panic!("{message}");
            }
        }
        if let Ok(screen) = supervisor.snapshot(id, 0)
            && screen
                .cells
                .iter()
                .map(|cell| cell.contents.as_str())
                .collect::<String>()
                .contains("recipe.bb")
        {
            break screen;
        }
        tokio::time::sleep(Duration::from_millis(30)).await;
    };
    if let Ok(path) = std::env::var("YOCTUI_GITUI_CAPTURE") {
        std::fs::write(path, serde_json::to_vec_pretty(&screen).unwrap()).unwrap();
    }
    let client = PtyClientId([42; 16]);
    supervisor.attach(id, client).unwrap();
    let epoch = supervisor.take(id, client, 0).unwrap();
    supervisor
        .resize(
            id,
            client,
            epoch,
            PtyDimensions {
                columns: 100,
                rows: 28,
            },
        )
        .unwrap();
    supervisor.input(id, client, epoch, b"\t".to_vec()).unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    let resized = supervisor.snapshot(id, 0).unwrap();
    assert_eq!(
        (resized.dimensions.columns, resized.dimensions.rows),
        (100, 28)
    );
    supervisor.input(id, client, epoch, b"q".to_vec()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        assert!(Instant::now() < deadline, "GitUI did not quit");
        if let Some(DaemonPtyEvent::Exited { exit_code, .. }) = supervisor.try_event() {
            assert_eq!(exit_code, Some(0));
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    std::fs::remove_dir_all(root).unwrap();
}
