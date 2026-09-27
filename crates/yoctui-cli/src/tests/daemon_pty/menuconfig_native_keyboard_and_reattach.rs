use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires a built native mconf; set YOCTUI_MENUCONFIG_PROGRAM"]
async fn menuconfig_native_keyboard_and_reattach() {
    let program = std::env::var("YOCTUI_MENUCONFIG_PROGRAM").unwrap();
    let root = std::env::temp_dir().join(format!("yoctui-menuconfig-keys-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("Kconfig"), "mainmenu \"Yoctui input check\"\nmenu \"First submenu\"\nconfig FIRST\n bool \"First option\"\nendmenu\nmenu \"Second submenu\"\nconfig SECOND\n bool \"Second option\"\nendmenu\n").unwrap();
    let mut supervisor = DaemonPtySupervisor::default();
    let id = supervisor
        .start_new(
            "kernel menuconfig".into(),
            PtyKind::Menuconfig,
            root.display().to_string(),
            PtyCommand {
                program,
                arguments: vec!["Kconfig".into()],
                environment_profile_id: None,
            },
            TerminalDimensions {
                columns: 110,
                rows: 32,
            },
        )
        .unwrap();
    let screen = wait_for(&mut supervisor, id, "Second submenu").await;
    assert!(
        screen.application_cursor,
        "ncurses must report application cursor mode"
    );
    let client = PtyClientId([42; 16]);
    supervisor.attach(id, client).unwrap();
    let epoch = supervisor.take(id, client, 0).unwrap();
    for key in [KeyCode::Down, KeyCode::Enter] {
        send(&mut supervisor, id, client, epoch, key);
        tokio::time::sleep(Duration::from_millis(60)).await;
    }
    wait_for(&mut supervisor, id, "Second option").await;
    send(&mut supervisor, id, client, epoch, KeyCode::Char('y'));
    wait_for(&mut supervisor, id, "[*]").await;
    send(&mut supervisor, id, client, epoch, KeyCode::Char('n'));
    wait_for(&mut supervisor, id, "[ ]").await;
    supervisor.detach(id, client).unwrap();
    let next_client = PtyClientId([43; 16]);
    supervisor.attach(id, next_client).unwrap();
    let epoch = supervisor.take(id, next_client, epoch + 1).unwrap();
    send(&mut supervisor, id, next_client, epoch, KeyCode::Char('/'));
    wait_for(&mut supervisor, id, "CONFIG_").await;
    for c in "SECOND".chars() {
        send(&mut supervisor, id, next_client, epoch, KeyCode::Char(c));
    }
    send(&mut supervisor, id, next_client, epoch, KeyCode::Enter);
    wait_for(&mut supervisor, id, "Symbol: SECOND").await;
    supervisor.terminate(id).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

fn send(
    supervisor: &mut DaemonPtySupervisor,
    id: PtySessionId,
    client: PtyClientId,
    epoch: u64,
    key: KeyCode,
) {
    let screen = supervisor.snapshot(id, 0).unwrap();
    let bytes = crate::input_routing::terminal_key_bytes(
        KeyEvent::new(key, KeyModifiers::NONE),
        screen.application_cursor,
    )
    .unwrap();
    supervisor.input(id, client, epoch, bytes).unwrap();
}

async fn wait_for(
    supervisor: &mut DaemonPtySupervisor,
    id: PtySessionId,
    text: &str,
) -> yoctui_protocol::daemon::PtyScreenSnapshot {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        while let Some(event) = supervisor.try_event() {
            if let DaemonPtyEvent::Lost { message, .. } = event {
                panic!("{message}");
            }
        }
        if let Ok(screen) = supervisor.snapshot(id, 0) {
            let contents = screen
                .cells
                .iter()
                .map(|cell| cell.contents.as_str())
                .collect::<String>();
            if contents.contains(text) {
                return screen;
            }
            assert!(Instant::now() < deadline, "missing {text:?}: {contents}");
        }
        assert!(Instant::now() < deadline, "menuconfig did not start");
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}
