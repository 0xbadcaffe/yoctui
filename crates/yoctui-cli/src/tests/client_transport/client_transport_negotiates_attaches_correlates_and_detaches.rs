use super::*;

#[test]
fn client_transport_negotiates_attaches_correlates_and_detaches() {
    let root = std::env::temp_dir().join(format!(
        "yoctui-client-transport-{}-{}",
        std::process::id(),
        std::time::SystemTime::UNIX_EPOCH
            .elapsed()
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    let paths = runtime_paths_for(root.clone(), unsafe { libc::geteuid() }).unwrap();
    let server_paths = paths.clone();
    let instance = DaemonInstanceId([9; 16]);
    let server = thread::spawn(move || {
        let listener = DaemonListener::bind(&server_paths).unwrap();
        let mut connection = listener.accept(Duration::from_secs(2)).unwrap();
        assert!(matches!(
            connection.receive::<ClientMessage>().unwrap(),
            ClientMessage::Hello(_)
        ));
        connection
            .send(&ServerMessage::Hello(hello(instance)))
            .unwrap();
        assert!(matches!(
            connection.receive::<ClientMessage>().unwrap(),
            ClientMessage::Attach { .. }
        ));
        connection
            .send(&ServerMessage::Attached {
                snapshot: snapshot(instance),
                replayed_through: 0,
            })
            .unwrap();
        assert!(matches!(
            connection.receive::<ClientMessage>().unwrap(),
            ClientMessage::Subscribe { .. }
        ));
        assert!(matches!(
            connection.receive::<ClientMessage>().unwrap(),
            ClientMessage::Unsubscribe { .. }
        ));
        let ClientMessage::Command(request) = connection.receive::<ClientMessage>().unwrap() else {
            panic!("expected command");
        };
        connection
            .send(&ServerMessage::Ping {
                nonce: 44,
                deadline_unix_ms: 0,
            })
            .unwrap();
        assert_eq!(
            connection.receive::<ClientMessage>().unwrap(),
            ClientMessage::Pong { nonce: 44 }
        );
        connection
            .send(&ServerMessage::CommandResult(CommandResult {
                request_id: request.request_id,
                outcome: CommandOutcome::Completed,
            }))
            .unwrap();
        assert_eq!(
            connection.receive::<ClientMessage>().unwrap(),
            ClientMessage::Detach
        );
        connection.send(&ServerMessage::Detaching).unwrap();

        let mut connection = listener.accept(Duration::from_secs(2)).unwrap();
        assert!(matches!(
            connection.receive::<ClientMessage>().unwrap(),
            ClientMessage::Hello(_)
        ));
        connection
            .send(&ServerMessage::Hello(hello(instance)))
            .unwrap();
        assert!(matches!(
            connection.receive::<ClientMessage>().unwrap(),
            ClientMessage::Attach {
                resume: Some(ResumeCursor {
                    last_sequence: 0,
                    ..
                }),
                ..
            }
        ));
        connection
            .send(&ServerMessage::Attached {
                snapshot: snapshot(instance),
                replayed_through: 0,
            })
            .unwrap();
        assert_eq!(
            connection.receive::<ClientMessage>().unwrap(),
            ClientMessage::Detach
        );
        connection.send(&ServerMessage::Detaching).unwrap();
    });
    while !paths.socket.exists() {
        thread::yield_now();
    }
    let mut client = DaemonClientTransport::connect_at(
        &paths,
        ClientId([7; 16]),
        "terminal-one".into(),
        Duration::from_secs(2),
    )
    .unwrap();
    assert_eq!(client.hello().daemon_instance_id, instance);
    let attached = client
        .attach(
            None,
            Subscription {
                state: true,
                jobs: true,
                logs: true,
                pty_sessions: Vec::new(),
            },
            None,
        )
        .unwrap();
    assert_eq!(attached.snapshot.daemon_instance_id, instance);
    let subscription = Subscription {
        state: false,
        jobs: false,
        logs: true,
        pty_sessions: Vec::new(),
    };
    client.subscribe(subscription.clone()).unwrap();
    client.unsubscribe(subscription).unwrap();
    client
        .command(CommandRequest {
            request_id: RequestId(12),
            expected_generation: Some(0),
            command: DaemonCommand::PrepareShutdown,
        })
        .unwrap();
    assert!(matches!(
        client.receive().unwrap(),
        ClientServerEvent::CommandResult(CommandResult {
            request_id: RequestId(12),
            ..
        })
    ));
    client.detach().unwrap();
    assert_eq!(client.state(), ClientTransportState::Disconnected);
    let mut client = client.reconnect_at(&paths, Duration::from_secs(2)).unwrap();
    client
        .attach(
            None,
            Subscription {
                state: true,
                jobs: true,
                logs: false,
                pty_sessions: Vec::new(),
            },
            Some(ResumeCursor {
                daemon_instance_id: instance,
                last_sequence: 0,
            }),
        )
        .unwrap();
    client.detach().unwrap();
    let _default_connect = DaemonClientTransport::connect;
    let _default_reconnect = DaemonClientTransport::reconnect;
    server.join().unwrap();
    fs::remove_dir_all(root).unwrap();
}
