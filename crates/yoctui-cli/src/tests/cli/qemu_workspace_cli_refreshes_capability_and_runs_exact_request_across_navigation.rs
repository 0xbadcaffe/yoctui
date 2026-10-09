use super::*;

#[cfg(unix)]
#[tokio::test]
async fn qemu_workspace_cli_refreshes_capability_and_runs_exact_request_across_navigation() {
    let (directory, build_dir, mut app) =
        qemu_workspace_fixture("success", "printf '%s\\n' \"$@\"; exit 0");
    assert!(matches!(
        app.qemu_capability,
        QemuCapability::Available { .. }
    ));
    assert_eq!(
        qemu_launch_dialog_action(false, Input::Char('Q')),
        None,
        "modal Q input must not leak to the Images workspace"
    );
    let (id, request) = qemu_workspace_start_effect(&mut app);
    let mut operation = None;
    begin_qemu_job(
        &mut app,
        &mut operation,
        &build_dir,
        Duration::from_millis(100),
        id,
        request.clone(),
    )
    .await;
    app.screen = Screen::Logs;
    poll_qemu_until(&mut app, &mut operation, |_, operation| operation.is_none()).await;
    let session = app.qemu_session(id).unwrap();
    let job = app.background_jobs.get(session.background_job_id).unwrap();
    assert_eq!(job.status, yoctui_model::BackgroundJobStatus::Succeeded);
    assert_eq!(app.screen, Screen::Logs);
    assert!(
        job.output
            .iter()
            .any(|entry| entry.message == request.image.path.display().to_string())
    );
    assert!(
        job.output
            .iter()
            .any(|entry| entry.message == "qemuparams=-m 1024")
    );

    let effect = update(&mut app, Action::RefreshImageArtifactInventory).unwrap();
    let mut scan = None;
    begin_image_artifact_operation(&mut app, None, &mut scan, effect);
    assert!(matches!(app.qemu_capability, QemuCapability::Failed { .. }));
    fs::remove_dir_all(directory).unwrap();
}

#[cfg(unix)]
#[test]
fn qemu_workspace_uses_initialized_daemon_runqemu_identity() {
    let (directory, _, mut app) = qemu_workspace_fixture("authority", "exit 0");
    let executable = directory.join("runqemu").canonicalize().unwrap();
    app.workspace_compatibility
        .install(yoctui_model::DaemonCompatibilitySnapshot {
            snapshot: yoctui_model::CapabilitySnapshot {
                generation: 1,
                environment: yoctui_model::YoctoEnvironmentIdentity {
                    available_tools: yoctui_model::AuthoritativeValue::detected(
                        vec![yoctui_model::ToolIdentity {
                            id: "runqemu".into(),
                            executable: executable.clone(),
                            version: None,
                        }],
                        yoctui_model::IdentityAuthority::ExecutableProbe,
                    ),
                    ..yoctui_model::YoctoEnvironmentIdentity::default()
                },
                capabilities: Vec::new(),
            },
            implementations: std::collections::BTreeMap::new(),
        })
        .unwrap();

    let inspector = qemu_capability_inspector(&app);
    execute_qemu_capability_effect(&mut app, &inspector, Effect::InspectQemuCapability);
    assert!(matches!(
        &app.qemu_capability,
        QemuCapability::Available { executable: actual, .. } if actual == &executable
    ));
    fs::remove_dir_all(directory).unwrap();
}
