use super::*;

#[test]
fn rootfs_client_ipc_negotiates_retries_only_stale_and_checks_reply_identity() {
    use yoctui_protocol::{
        daemon::*,
        daemon_ipc::{DaemonListener, runtime_paths_for},
    };
    for mode in [
        "normal",
        "old",
        "wrong-instance",
        "wrong-reply",
        "stale",
        "disconnect",
    ] {
        let (build, query, compatibility, _) = fixture();
        fs::create_dir(build.join("runtime")).unwrap();
        fs::set_permissions(build.join("runtime"), fs::Permissions::from_mode(0o700)).unwrap();
        let paths = runtime_paths_for(build.join("runtime"), unsafe { libc::geteuid() }).unwrap();
        let listener = DaemonListener::bind(&paths).unwrap();
        let worker_query = query.clone();
        let mut state = DaemonGlobalState::new(
            DaemonModelInstanceId(query.daemon_instance_id.0),
            1,
            "fixture".into(),
            DaemonStateLimits::default(),
        )
        .unwrap();
        state.compatibility = Some(compatibility);
        let snapshot = yoctui_app::daemon_protocol_snapshot(&state);
        let server = std::thread::spawn(move || {
            let mut connection = listener.accept(Duration::from_secs(3)).unwrap();
            connection
                .set_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            assert!(matches!(
                connection.receive::<ClientMessage>().unwrap(),
                ClientMessage::Hello(_)
            ));
            let mut instance = worker_query.daemon_instance_id;
            if mode == "wrong-instance" {
                instance.0[15] ^= 1;
            }
            connection
                .send(&ServerMessage::Hello(DaemonHello {
                    selected_version: ProtocolVersion::CURRENT,
                    daemon_instance_id: instance,
                    boot_id: "fixture".into(),
                    capabilities: if mode == "old" {
                        vec![]
                    } else {
                        vec![Capability::RootfsSources]
                    },
                    limits: ProtocolLimits {
                        maximum_frame_bytes: MAX_FRAME_BYTES as u32,
                        maximum_snapshot_bytes: MAX_FRAME_BYTES as u32,
                        maximum_pending_requests: 8,
                        maximum_queue_depth: 16,
                        maximum_terminal_rows: 512,
                        maximum_terminal_columns: 512,
                        maximum_clients: 32,
                        maximum_pty_sessions: 64,
                        maximum_scrollback_lines: 100000,
                        maximum_utility_output_bytes: 4 * 1024 * 1024,
                    },
                }))
                .unwrap();
            if matches!(mode, "old" | "wrong-instance") {
                return;
            }
            let mut commands = 0;
            loop {
                match connection.receive::<ClientMessage>().unwrap() {
                    ClientMessage::Attach { .. } => connection
                        .send(&ServerMessage::Attached {
                            snapshot: snapshot.clone(),
                            replayed_through: snapshot.sequence,
                        })
                        .unwrap(),
                    ClientMessage::Command(request) => {
                        commands += 1;
                        assert_eq!(request.expected_generation, Some(snapshot.generation));
                        assert_eq!(
                            request.command,
                            DaemonCommand::InspectRootfsSources {
                                query: worker_query.clone()
                            }
                        );
                        if mode == "disconnect" {
                            return;
                        }
                        let outcome = if mode == "stale" && commands == 1 {
                            CommandOutcome::Rejected {
                                code: ProtocolErrorCode::StaleGeneration,
                                message: "refresh".into(),
                                current_generation: snapshot.generation,
                            }
                        } else {
                            let mut query = worker_query.clone();
                            if mode == "wrong-reply" {
                                query.request.generation += 1;
                            }
                            CommandOutcome::RootfsSources {
                                sources: Box::new(RootfsSourcesData {
                                    query,
                                    image_manifest: None,
                                    pkgdata_dir: None,
                                    image_rootfs: Some("/build/retained-rootfs".into()),
                                }),
                            }
                        };
                        connection
                            .send(&ServerMessage::CommandResult(CommandResult {
                                request_id: request.request_id,
                                outcome,
                            }))
                            .unwrap();
                        if mode != "stale" || commands == 2 {
                            break;
                        }
                    }
                    other => panic!("unexpected client message: {other:?}"),
                }
            }
            assert_eq!(commands, if mode == "stale" { 2 } else { 1 });
        });
        let result = request_sources_at(
            &query,
            &build,
            &yoctui_bitbake::RootfsCompositionCancellation::default(),
            &paths,
        );
        if matches!(mode, "normal" | "stale") {
            assert_eq!(result.unwrap().query, query);
        } else {
            assert!(result.is_err(), "{mode}");
        }
        server.join().unwrap();
        fs::remove_dir_all(build).unwrap();
    }
}
