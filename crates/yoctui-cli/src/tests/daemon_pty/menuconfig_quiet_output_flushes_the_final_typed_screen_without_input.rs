use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn quiet_output_flushes_the_final_typed_screen_without_input() {
    let mut supervisor = DaemonPtySupervisor::default();
    let id = supervisor
        .start_new(
            "screen flush fixture".into(),
            PtyKind::Utility,
            "/tmp".into(),
            PtyCommand {
                program: "/usr/bin/python3".into(),
                arguments: vec![
                    "-c".into(),
                    "import os,time; os.write(1,b'first'); time.sleep(.005); os.write(1,b' FINAL_FRAME'); time.sleep(.25)".into(),
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
    let flushed = loop {
        assert!(
            Instant::now() < deadline,
            "final quiet screen was not published"
        );
        if let Some(event) = supervisor.try_event() {
            match event {
                DaemonPtyEvent::Screen(screen) if screen.session_id.0 == id.0 => {
                    let contents = screen
                        .cells
                        .iter()
                        .map(|cell| cell.contents.as_str())
                        .collect::<String>();
                    if contents.contains("FINAL_FRAME") {
                        break screen;
                    }
                }
                DaemonPtyEvent::Lost { message, .. } => panic!("{message}"),
                _ => {}
            }
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    };

    assert_eq!(flushed.session_id.0, id.0);
    supervisor.terminate(id).unwrap();
}
