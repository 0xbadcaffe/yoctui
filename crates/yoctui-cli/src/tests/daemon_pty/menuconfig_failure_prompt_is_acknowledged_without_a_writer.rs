use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn menuconfig_failure_prompt_is_acknowledged_without_a_writer() {
    let mut supervisor = DaemonPtySupervisor::default();
    let id = supervisor
        .start_new(
            "failed menuconfig fixture".into(),
            PtyKind::Menuconfig,
            "/tmp".into(),
            PtyCommand {
                program: "/usr/bin/python3".into(),
                arguments: vec![
                    "-c".into(),
                    "import os,sys,time; os.write(1,b'ncurses missing\\nPress any '); time.sleep(.02); os.write(1,b'key to continue...'); sys.stdin.buffer.read(1); sys.exit(23)".into(),
                ],
                environment_profile_id: None,
            },
            TerminalDimensions {
                columns: 80,
                rows: 8,
            },
        )
        .unwrap();

    let deadline = Instant::now() + Duration::from_secs(2);
    let exit_code = loop {
        assert!(
            Instant::now() < deadline,
            "failed menuconfig remained blocked on its acknowledgement prompt"
        );
        if let Some(event) = supervisor.try_event() {
            match event {
                DaemonPtyEvent::Exited {
                    session_id,
                    exit_code,
                    screen,
                } if session_id == id => {
                    let contents = screen
                        .expect("failed menuconfig retains its diagnostic screen")
                        .cells
                        .iter()
                        .map(|cell| cell.contents.as_str())
                        .collect::<String>();
                    assert!(contents.contains("ncurses missing"));
                    assert!(contents.contains("Press any key to continue..."));
                    break exit_code;
                }
                DaemonPtyEvent::Lost { message, .. } => panic!("{message}"),
                _ => {}
            }
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    };

    assert_eq!(exit_code, Some(23));
}
