//! Regression tests grouped around task_identity_statistics_decode_without_recipe_or_task_inference.
use super::*;

#[test]
fn task_identity_statistics_decode_without_recipe_or_task_inference() {
    let event = serde_json::from_str::<super::Event>(
        r#"{"type":"task_stats","stats":{"completed":3,"total":10,"active":1,"failed":0}}"#,
    )
    .unwrap();
    assert!(matches!(
        super::BridgeBackend::event(event).unwrap(),
        super::BackendEvent::TaskStats(super::TaskStats {
            completed: 3,
            total: 10,
            active: 1,
            failed: 0
        })
    ));
}

#[tokio::test]
async fn devtool_metadata_fake_process_reports_workspace_and_dirty_git_state() {
    let root = fixture_script("devtool-workspace");
    let source = root.join("sources/busybox");
    fs::create_dir_all(&source).unwrap();
    let devtool = root.join("devtool");
    let git = root.join("git");
    fs::write(
        &devtool,
        format!(
            "#!/bin/sh\nprintf '%s\\n' 'busybox: {} (/layers/core/busybox_1.0.bb)'\n",
            source.display()
        ),
    )
    .unwrap();
    fs::write(
            &git,
            "#!/bin/sh\nprintf '%s\\n' '# branch.oid abc123' '# branch.head work' '1 .M N... 100644 100644 100644 abc abc file.c' '? new.txt' 'u UU N... 100644 100644 100644 100644 abc abc abc conflict.c'\n",
        )
        .unwrap();
    for script in [&devtool, &git] {
        let mut permissions = fs::metadata(script).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(script, permissions).unwrap();
    }
    let identity = RecipeIdentity {
        name: "busybox".into(),
        file: "/layers/core/busybox_1.0.bb".into(),
    };
    let compatibility = devtool_compatibility(&root, &devtool);
    let status = DevtoolInspector::with_programs(devtool, git)
        .inspect_with_compatibility(&root, identity.clone(), &compatibility, 1)
        .await;
    assert_eq!(status.identity, identity);
    assert_eq!(status.capability, DevtoolCapability::Available);
    assert_eq!(
        status.workspace,
        DevtoolWorkspace::Present {
            source_path: source,
            recipe_file: Some("/layers/core/busybox_1.0.bb".into()),
        }
    );
    assert_eq!(
        status.git,
        DevtoolGitState::Available {
            branch: Some("work".into()),
            head: Some("abc123".into()),
            modified: 1,
            untracked: 1,
            conflicted: 1,
        }
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn devtool_job_spec_builds_exact_shell_free_arguments_for_every_operation() {
    let cases = [
        (
            DevtoolOperation::Modify {
                recipe: "busybox".into(),
            },
            vec!["modify", "busybox"],
        ),
        (
            DevtoolOperation::UpdateRecipe {
                recipe: "busybox".into(),
            },
            vec!["update-recipe", "busybox"],
        ),
        (
            DevtoolOperation::Finish {
                recipe: "busybox".into(),
                destination: "/layers/meta-custom".into(),
            },
            vec!["finish", "busybox", "/layers/meta-custom"],
        ),
        (
            DevtoolOperation::DeployTarget {
                recipe: "busybox".into(),
                target: "root@192.0.2.1:/opt".into(),
            },
            vec!["deploy-target", "busybox", "root@192.0.2.1:/opt"],
        ),
        (
            DevtoolOperation::UndeployTarget {
                recipe: "busybox".into(),
                target: "root@192.0.2.1".into(),
            },
            vec!["undeploy-target", "busybox", "root@192.0.2.1"],
        ),
        (
            DevtoolOperation::Reset {
                recipe: "busybox".into(),
            },
            vec!["reset", "busybox"],
        ),
    ];
    for (operation, expected) in cases {
        let command = authorized_devtool_command("devtool".into(), &operation).unwrap();
        assert_eq!(command.executable(), Path::new("/test/bin/devtool"));
        assert_eq!(
            command.arguments(),
            expected.into_iter().map(OsString::from).collect::<Vec<_>>()
        );
    }
}

#[test]
fn devtool_job_spec_rejects_invalid_operations_before_process_construction() {
    assert!(matches!(
        authorized_devtool_command("devtool".into(), &DevtoolOperation::Reset {
            recipe: "--help".into(),
        }),
        Err(DevtoolCompatibilityError::InvalidRequest(message)) if message.contains("recipe")
    ));
    assert!(matches!(
        authorized_devtool_command("devtool".into(), &DevtoolOperation::DeployTarget {
            recipe: "busybox".into(),
            target: "root@host\n--help".into(),
        }),
        Err(DevtoolCompatibilityError::InvalidRequest(message)) if message.contains("target")
    ));
    assert!(matches!(
        authorized_devtool_command("devtool".into(), &DevtoolOperation::Finish {
            recipe: "busybox".into(),
            destination: "meta-custom".into(),
        }),
        Err(DevtoolCompatibilityError::InvalidRequest(message)) if message.contains("absolute")
    ));
}

#[test]
fn devtool_publish_update_uses_exact_shell_free_arguments() {
    let command = authorized_devtool_command(
        "devtool".into(),
        &DevtoolOperation::UpdateRecipe {
            recipe: "busybox".into(),
        },
    )
    .unwrap();
    assert_eq!(command.executable(), Path::new("/test/bin/devtool"));
    assert_eq!(
        command.arguments(),
        [OsString::from("update-recipe"), OsString::from("busybox")]
    );
}

#[test]
fn devtool_publish_finish_uses_exact_shell_free_arguments() {
    let command = authorized_devtool_command(
        "devtool".into(),
        &DevtoolOperation::Finish {
            recipe: "busybox".into(),
            destination: "/layers/meta-demo".into(),
        },
    )
    .unwrap();
    assert_eq!(command.executable(), Path::new("/test/bin/devtool"));
    assert_eq!(
        command.arguments(),
        [
            OsString::from("finish"),
            OsString::from("busybox"),
            OsString::from("/layers/meta-demo"),
        ]
    );
}

#[cfg(unix)]
#[test]
fn devtool_publish_finish_preserves_native_destination_bytes() {
    use std::os::unix::ffi::OsStringExt;

    let mut bytes = b"/layers/meta-".to_vec();
    bytes.push(0xfe);
    let command = authorized_devtool_command(
        "devtool".into(),
        &DevtoolOperation::Finish {
            recipe: "busybox".into(),
            destination: PathBuf::from(OsString::from_vec(bytes.clone())),
        },
    )
    .unwrap();
    assert_eq!(command.arguments()[2], OsString::from_vec(bytes));
}

#[test]
fn devtool_target_deploy_validates_before_exact_shell_free_arguments() {
    let operation = DevtoolOperation::DeployTarget {
        recipe: "busybox".into(),
        target: "root@192.0.2.1:/opt/demo".into(),
    };
    let command = authorized_devtool_command("devtool".into(), &operation).unwrap();
    assert_eq!(command.executable(), Path::new("/test/bin/devtool"));
    assert_eq!(
        command.arguments(),
        [
            OsString::from("deploy-target"),
            OsString::from("busybox"),
            OsString::from("root@192.0.2.1:/opt/demo"),
        ]
    );
    assert!(matches!(
        authorized_devtool_command("devtool".into(), &DevtoolOperation::DeployTarget {
            recipe: "busybox".into(),
            target: "--help".into(),
        }),
        Err(DevtoolCompatibilityError::InvalidRequest(message)) if message.contains("target")
    ));
}

#[test]
fn devtool_target_reset_uses_exact_shell_free_arguments() {
    let command = authorized_devtool_command(
        "devtool".into(),
        &DevtoolOperation::Reset {
            recipe: "busybox".into(),
        },
    )
    .unwrap();
    assert_eq!(command.executable(), Path::new("/test/bin/devtool"));
    assert_eq!(
        command.arguments(),
        [OsString::from("reset"), OsString::from("busybox")]
    );
}

#[cfg(unix)]
#[test]
fn devtool_job_spec_preserves_non_utf8_finish_destination() {
    use std::os::unix::ffi::OsStringExt;

    let mut bytes = b"/layers/meta-".to_vec();
    bytes.push(0xff);
    let destination = PathBuf::from(OsString::from_vec(bytes.clone()));
    let command = authorized_devtool_command(
        "devtool".into(),
        &DevtoolOperation::Finish {
            recipe: "busybox".into(),
            destination,
        },
    )
    .unwrap();
    assert_eq!(command.arguments()[2], OsString::from_vec(bytes));
}

#[cfg(unix)]
#[tokio::test]
async fn devtool_job_runner_streams_bounded_stdout_stderr_and_invalid_utf8() {
    let (script, command) = fake_devtool_command(
        "devtool-runner-output",
        "printf 'stdout line\\n'\nprintf 'stderr line\\n' >&2\nprintf '\\377bad\\n'\nhead -c 70000 /dev/zero | tr '\\000' x\nprintf '\\n'",
    );
    let mut runner = DevtoolJobRunner::new(std::env::temp_dir());
    runner.start(command).await.unwrap();
    assert_eq!(
        runner.next_event().await.unwrap(),
        DevtoolRunnerEvent::Started
    );
    let mut output = Vec::new();
    loop {
        match runner.next_event().await.unwrap() {
            DevtoolRunnerEvent::Output {
                stream,
                line,
                truncated,
            } => output.push((stream, line, truncated)),
            DevtoolRunnerEvent::Completed { exit_code } => {
                assert_eq!(exit_code, Some(0));
                break;
            }
            event => panic!("unexpected runner event: {event:?}"),
        }
    }
    assert!(output.iter().any(|(stream, line, _)| {
        *stream == DevtoolOutputStream::Stdout && line == "stdout line"
    }));
    assert!(output.iter().any(|(stream, line, _)| {
        *stream == DevtoolOutputStream::Stderr && line == "stderr line"
    }));
    assert!(output.iter().any(|(_, line, _)| line.contains('\u{fffd}')));
    let (_, truncated_line, truncated) = output
        .iter()
        .find(|(_, _, truncated)| *truncated)
        .expect("oversized output was not marked truncated");
    assert!(*truncated);
    assert!(truncated_line.len() <= MAX_DEVTOOL_LINE_BYTES);
    fs::remove_file(script).unwrap();
}

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

#[cfg(unix)]
#[tokio::test]
async fn devtool_job_runner_acknowledges_and_escalates_cancellation() {
    for (name, trap, expected_forced) in [
        ("devtool-runner-graceful", "trap 'exit 0' TERM", false),
        ("devtool-runner-forced", "trap '' TERM", true),
    ] {
        let (script, command) = fake_devtool_command(
            name,
            &format!("{trap}\nprintf 'ready\\n'\nwhile :; do :; done"),
        );
        let mut runner = DevtoolJobRunner::new(std::env::temp_dir())
            .with_cancellation_timeout(Duration::from_millis(250));
        runner.start(command).await.unwrap();
        assert_eq!(
            runner.next_event().await.unwrap(),
            DevtoolRunnerEvent::Started
        );
        loop {
            if matches!(
                runner.next_event().await.unwrap(),
                DevtoolRunnerEvent::Output { ref line, .. } if line == "ready"
            ) {
                break;
            }
        }
        assert!(runner.cancel().await.unwrap());
        assert!(!runner.cancel().await.unwrap());
        loop {
            if let DevtoolRunnerEvent::Cancelled { forced, .. } = runner.next_event().await.unwrap()
            {
                assert_eq!(forced, expected_forced);
                break;
            }
        }
        fs::remove_file(script).unwrap();
    }
}

#[cfg(unix)]
#[tokio::test]
async fn devtool_job_runner_reports_unexpected_event_channel_loss() {
    let (script, command) =
        fake_devtool_command("devtool-runner-channel-loss", "printf 'ready\\n'\nsleep 30");
    let mut runner = DevtoolJobRunner::new(std::env::temp_dir());
    runner.start(command).await.unwrap();
    assert_eq!(
        runner.next_event().await.unwrap(),
        DevtoolRunnerEvent::Started
    );
    runner.output = None;
    assert!(matches!(
        runner.next_event().await.unwrap(),
        DevtoolRunnerEvent::Lost { message } if message.contains("channel")
    ));
    assert!(!runner.is_active());
    fs::remove_file(script).unwrap();
}

#[tokio::test]
async fn devtool_metadata_distinguishes_missing_tool_and_workspace_directory() {
    let root = fixture_script("devtool-missing");
    fs::create_dir_all(&root).unwrap();
    let identity = RecipeIdentity {
        name: "busybox".into(),
        file: "/layers/core/busybox_1.0.bb".into(),
    };
    let missing_devtool = root.join("does-not-exist");
    let compatibility = devtool_compatibility(&root, &missing_devtool);
    let missing =
        DevtoolInspector::with_programs(missing_devtool, root.join("does-not-exist-either"))
            .inspect_with_compatibility(&root, identity.clone(), &compatibility, 1)
            .await;
    assert_eq!(missing.capability, DevtoolCapability::MissingExecutable);

    let devtool = root.join("devtool");
    let absent_source = root.join("sources/absent");
    fs::write(
        &devtool,
        format!(
            "#!/bin/sh\nprintf '%s\\n' 'busybox: {}'\n",
            absent_source.display()
        ),
    )
    .unwrap();
    let mut permissions = fs::metadata(&devtool).unwrap().permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&devtool, permissions).unwrap();
    let compatibility = devtool_compatibility(&root, &devtool);
    let status = DevtoolInspector::with_programs(devtool, root.join("git"))
        .inspect_with_compatibility(&root, identity, &compatibility, 1)
        .await;
    assert_eq!(
        status.workspace,
        DevtoolWorkspace::MissingDirectory {
            source_path: absent_source
        }
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn devtool_metadata_rejects_malformed_external_records() {
    assert_eq!(
        parse_devtool_status("busybox relative/path"),
        Err("busybox relative/path".into())
    );
    assert_eq!(
        parse_git_status("unexpected"),
        Err("unrecognized Git status record: unexpected".into())
    );
}

#[test]
fn devtool_metadata_ignores_bounded_bitbake_diagnostics() {
    assert_eq!(
        parse_devtool_status(
            "NOTE: Starting bitbake server...\nWARNING: using existing server\nbusybox: /workspace/sources/busybox (/layers/core/busybox_1.0.bb)\n"
        ),
        Ok(vec![(
            "busybox".into(),
            "/workspace/sources/busybox".into(),
            Some("/layers/core/busybox_1.0.bb".into()),
        )])
    );
}

#[test]
fn ansi_and_severity() {
    assert_eq!(strip_ansi("\x1b[31merror: bad\x1b[0m"), "error: bad");
    assert_eq!(
        classify_output("WARNING: x".into()).severity,
        Severity::Warning
    )
}
#[test]
fn typed_event_preserves_unknown_progress_and_ignores_future_events() {
    assert!(matches!(
        BridgeBackend::event(Event::TaskProgress {
            recipe: "busybox".into(),
            task: "do_compile".into(),
            progress: None,
        })
        .unwrap(),
        BackendEvent::TaskProgress { progress: None, .. }
    ));
    assert!(matches!(
        BridgeBackend::event(Event::Unknown).unwrap(),
        BackendEvent::Ignored
    ));
    assert!(matches!(
        BridgeBackend::event(Event::BridgeShutdown).unwrap(),
        BackendEvent::Disconnected
    ));
}

#[test]
fn dependency_graph_typed_event_converts_nodes_edges_and_limitations() {
    let root = DependencyNodeIdData {
        recipe: "image".into(),
        task: None,
    };
    let task = DependencyNodeIdData {
        recipe: "busybox".into(),
        task: Some("do_compile".into()),
    };
    let event = Event::DependencyGraph {
        data: DependencyGraphData {
            root: root.clone(),
            nodes: vec![DependencyNodeData {
                id: task.clone(),
                provider: Some("/layers/meta/busybox.bb".into()),
                log: Some("tmp/log.do_compile".into()),
            }],
            edges: vec![DependencyEdgeData {
                from: root,
                to: task.clone(),
                kind: DependencyEdgeKindData::Task,
            }],
            limitations: vec!["runtime unavailable".into()],
        },
    };
    let BackendEvent::DependencyGraph { graph, limitations } = BridgeBackend::event(event).unwrap()
    else {
        panic!("dependency graph event was not preserved");
    };
    assert_eq!(graph.root, DependencyNodeId::recipe("image"));
    assert_eq!(
        graph
            .nodes
            .iter()
            .find(|node| node.id == DependencyNodeId::task("busybox", "do_compile"))
            .and_then(|node| node.provider.as_deref()),
        Some(Path::new("/layers/meta/busybox.bb"))
    );
    assert_eq!(graph.edges[0].kind, DependencyEdgeKind::Task);
    assert_eq!(
        graph
            .nodes
            .iter()
            .find(|node| node.id == DependencyNodeId::task("busybox", "do_compile"))
            .and_then(|node| node.log.as_ref()),
        None
    );
    assert!(
        limitations
            .iter()
            .any(|value| value == "runtime unavailable")
    );
    assert!(
        limitations
            .iter()
            .any(|value| value.contains("non-absolute"))
    );
}

#[test]
fn typed_event_workspace_converts_wire_paths_and_metadata() {
    let event = Event::Workspace {
        data: yoctui_protocol::WorkspaceData {
            build_dir: Some("/build".into()),
            source_dir: Some("/poky".into()),
            variables: std::collections::HashMap::from([("MACHINE".into(), "qemux86-64".into())]),
            variable_provenance: std::collections::HashMap::new(),
            variable_provenance_chain: std::collections::HashMap::new(),
            bitbake_version: Some("2.19.0".into()),
            release: Some("6.0".into()),
            layers: vec![LayerData {
                name: "core".into(),
                path: "/poky/meta".into(),
                priority: Some(5),
            }],
            recipes: vec![RecipeData {
                name: "base-files".into(),
                version: None,
                layer: Some("core".into()),
                preferred_version: None,
                file: Some("/poky/meta/recipes-core/base-files/base-files.bb".into()),
                append_count: Some(0),
            }],
        },
    };
    let BackendEvent::Workspace(workspace) = BridgeBackend::event(event).unwrap() else {
        panic!("workspace event was not preserved");
    };
    assert_eq!(workspace.build_dir, Some(PathBuf::from("/build")));
    assert_eq!(workspace.layers[0].path, PathBuf::from("/poky/meta"));
    assert_eq!(workspace.recipes[0].name, "base-files");
}
#[test]
fn config_metadata_converts_scope_unexpanded_value_and_operations() {
    let event = Event::Variable {
        name: "MACHINE".into(),
        recipe: Some("base-files".into()),
        value: Some("qemux86-64".into()),
        provenance: Some("/build/conf/local.conf:12".into()),
        unexpanded_value: Some("${DEFAULT_MACHINE}".into()),
        operations: vec![yoctui_protocol::VariableOperationData {
            operation: "set".into(),
            file: Some("/build/conf/local.conf".into()),
            line: Some(12),
            value: Some("${DEFAULT_MACHINE}".into()),
        }],
        active_overrides: vec!["qemux86-64".into()],
    };
    let BackendEvent::Variable {
        name,
        recipe,
        value,
        unexpanded_value,
        operations,
        active_overrides,
        ..
    } = BridgeBackend::event(event).unwrap()
    else {
        panic!("variable detail event was not preserved");
    };
    assert_eq!(name, "MACHINE");
    assert_eq!(recipe.as_deref(), Some("base-files"));
    assert_eq!(value.as_deref(), Some("qemux86-64"));
    assert_eq!(unexpanded_value.as_deref(), Some("${DEFAULT_MACHINE}"));
    assert_eq!(
        operations[0].file,
        Some(PathBuf::from("/build/conf/local.conf"))
    );
    assert_eq!(operations[0].line, Some(12));
    assert_eq!(active_overrides, ["qemux86-64"]);
}
#[test]
fn recipe_metadata_converts_typed_statuses_and_paths() {
    let event = Event::RecipeMetadata {
        data: yoctui_protocol::RecipeMetadataData {
            recipe: "busybox".into(),
            workspace_status: Some(RecipeWorkspaceStatusData::Modified),
            build_status: Some(RecipeBuildStatusData::Running),
            tasks: Some(vec!["do_compile".into()]),
            sources: Some(vec!["/layers/meta/busybox.bb".into()]),
            patches: Some(vec!["file://fix.patch".into()]),
            packages: Some(vec!["busybox".into()]),
            history: None,
        },
    };
    let BackendEvent::RecipeMetadata(metadata) = BridgeBackend::event(event).unwrap() else {
        panic!("recipe metadata event was not preserved");
    };
    assert_eq!(
        metadata.workspace_status,
        Some(RecipeWorkspaceStatus::Modified)
    );
    assert_eq!(metadata.build_status, Some(RecipeBuildStatus::Running));
    assert_eq!(
        metadata.sources,
        Some(vec![PathBuf::from("/layers/meta/busybox.bb")])
    );
    assert_eq!(metadata.history, None);
}
#[test]
fn live_tasks_preserves_queue_statistics_and_task_details() {
    let queued = BridgeBackend::event(Event::TaskQueued {
        recipe: "busybox".into(),
        task: "do_compile".into(),
        worker: Some("worker-1".into()),
        stats: Some(TaskStatsData {
            completed: 3,
            total: 10,
            active: 2,
            failed: 1,
        }),
    })
    .unwrap();
    assert!(matches!(
        queued,
        BackendEvent::TaskQueued {
            stats: Some(TaskStats { total: 10, .. }),
            ..
        }
    ));
    let started = BridgeBackend::event(Event::TaskStarted {
        recipe: "busybox".into(),
        task: "do_compile".into(),
        pid: Some(42),
        worker: Some("worker-1".into()),
        log_path: Some("/tmp/log.do_compile".into()),
        stats: None,
    })
    .unwrap();
    assert!(matches!(
        started,
        BackendEvent::TaskStarted {
            pid: Some(42),
            log_path: Some(path),
            ..
        } if path.as_os_str() == "/tmp/log.do_compile"
    ));
}
#[test]
fn invalid_utf8_output_is_preserved_lossily() {
    assert_eq!(output_text(b"warning: \xff\n"), "warning: �");
}

#[tokio::test]
async fn oversized_process_line_is_truncated_and_stream_continues() {
    let (mut writer, reader) = tokio::io::duplex(MAX_PROCESS_LINE_BYTES + 2);
    let (sender, mut receiver) = tokio::sync::mpsc::channel(2);
    let reader_task = tokio::spawn(read_output(reader, sender));
    writer
        .write_all(&vec![b'x'; MAX_PROCESS_LINE_BYTES + 1])
        .await
        .unwrap();
    writer.write_all(b"\nnext line\n").await.unwrap();
    drop(writer);
    reader_task.await.unwrap();
    assert!(
        receiver
            .recv()
            .await
            .unwrap()
            .message
            .ends_with("[line truncated]")
    );
    assert_eq!(receiver.recv().await.unwrap().message, "next line");
}

#[tokio::test]
async fn process_backend_collects_both_output_streams() {
    let script = fixture_script("fake-bitbake");
    fs::write(
        &script,
        "#!/bin/sh\nprintf 'NOTE: stdout line\\n'\nprintf 'WARNING: stderr line\\n' >&2\n",
    )
    .unwrap();
    let mut permissions = fs::metadata(&script).unwrap().permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&script, permissions).unwrap();
    let mut backend = shell_backend(script.clone());
    backend
        .start_build(BuildRequest {
            targets: vec!["core-image-minimal".into()],
            task: None,
            force: false,
        })
        .await
        .unwrap();
    let mut messages = Vec::new();
    loop {
        match backend.next_event().await.unwrap() {
            BackendEvent::Log(entry) => messages.push(entry),
            BackendEvent::BuildCompleted { success, .. } => {
                assert!(success);
                break;
            }
            _ => {}
        }
    }
    fs::remove_file(script).unwrap();
    assert_eq!(messages.len(), 2);
    assert!(
        messages
            .iter()
            .any(|entry| entry.severity == Severity::Warning)
    );
}

#[test]
fn dependency_graph_dot_parser_normalizes_task_build_cycles_and_bounds() {
    let graph = br#"digraph depends {
"image.do_build" [label="image do_build"]
"busybox.do_build" [label="busybox do_build"]
"image.do_build" -> "busybox.do_build"
"image.do_build" -> "busybox.do_build"
"busybox.do_build" -> "image.do_build"
}
"#;
    let response = parse_task_dependency_dot("image", graph).unwrap();
    assert!(
        response
            .limitations
            .iter()
            .any(|value| value.contains("runtime"))
    );
    assert!(response.graph.edges.contains(&DependencyEdge {
        from: DependencyNodeId::recipe("image"),
        to: DependencyNodeId::recipe("busybox"),
        kind: DependencyEdgeKind::Build,
    }));
    assert!(response.graph.edges.contains(&DependencyEdge {
        from: DependencyNodeId::recipe("image"),
        to: DependencyNodeId::task("image", "do_build"),
        kind: DependencyEdgeKind::Task,
    }));
    assert!(response.graph.edges.contains(&DependencyEdge {
        from: DependencyNodeId::task("image", "do_build"),
        to: DependencyNodeId::task("busybox", "do_build"),
        kind: DependencyEdgeKind::Task,
    }));
    assert_eq!(
        parse_task_dependency_dot("image", b"not a dot graph")
            .unwrap_err()
            .to_string(),
        "bridge: dependency graph has an invalid header"
    );

    let mut bounded = String::from("digraph depends {\n");
    for index in 0..=MAX_DEPENDENCY_EDGES {
        bounded.push_str(&format!(
            "\"image.do_{index}\" -> \"dep-{index}.do_build\"\n"
        ));
    }
    bounded.push_str("}\n");
    let response = parse_task_dependency_dot("image", bounded.as_bytes()).unwrap();
    assert!(
        response
            .limitations
            .iter()
            .any(|value| value.contains("bounds dropped"))
    );
    assert!(response.graph.nodes.len() <= MAX_DEPENDENCY_NODES);
    assert!(response.graph.edges.len() <= MAX_DEPENDENCY_EDGES);
}
