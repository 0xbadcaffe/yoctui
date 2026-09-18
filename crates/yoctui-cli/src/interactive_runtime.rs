//! Interactive runtime.
use super::*;

pub(crate) async fn tui(
    config: Config,
    targets: Vec<String>,
    mut session: Session,
    mut internal_tracing_capture: internal_tracing::InternalTracingCapture,
) -> Result<()> {
    let Config {
        backend: backend_kind,
        mut build_dir,
        mut build_dir_configured,
        log_entries,
        log_bytes,
        refresh,
        cancellation_timeout,
        color,
        color_forced_off,
        theme,
        animation_speed,
        reduced_motion,
        preferences,
        editor,
        session_path,
        ..
    } = config;
    // Yoctui resolves color itself (including --no-color and the persisted
    // Settings value), so Crossterm must not silently apply a second ambient
    // NO_COLOR policy that contradicts the visible setting.
    crossterm::style::force_color_output(true);
    let guard = TerminalGuard::enter()?;
    let mut terminal = Terminal::new(ratatui::backend::CrosstermBackend::new(io::stdout()))?;
    let mut app = if build_dir_configured {
        App::new(log_entries, log_bytes)
    } else {
        App::new_unconfigured(log_entries, log_bytes)
    };
    // The interactive application always opens on Overview / Dashboard in
    // Navigator. Model fixtures retain their explicit focus semantics.
    app.focus = yoctui_model::FocusTarget::Navigator;
    app.require_daemon = true;
    app.client_access_origin = client_access_origin();
    let _ = update(
        &mut app,
        Action::DetachedTerminalAvailabilityDetected(detached_terminal_availability()),
    );
    let _ = update(
        &mut app,
        Action::SshClientCapabilityDetected(ssh_client_capability()),
    );
    let _ = update(
        &mut app,
        Action::GitUiDetected(executable_on_initialized_path("gitui")),
    );
    if build_dir_configured {
        app.workspace.build_dir = Some(build_dir.clone());
    }
    app.backend = backend_kind.to_string();
    app.color_forced_off = color_forced_off;
    app.install_preferences(preferences)
        .map_err(anyhow::Error::msg)
        .context("could not install workbench preferences")?;
    app.color_enabled = color;
    debug_assert_eq!(app.theme, theme);
    debug_assert_eq!(app.animation_speed, animation_speed);
    debug_assert_eq!(app.reduced_motion, reduced_motion);
    install_session_raw_favorites(&session, &mut app)?;
    if let Some(layout) = session.pane_layout.clone()
        && app.preferences.remember_pane_sizes
        && layout.validate().is_ok()
    {
        app.pane_layout = layout;
    }
    #[cfg(unix)]
    let mut daemon_runtime = match client_runtime::InteractiveDaemonRuntime::connect(
        &mut app,
        client_runtime::INITIAL_DAEMON_ATTACH_TIMEOUT,
    ) {
        Ok(runtime) => Some(runtime),
        Err(error) => {
            tracing::debug!(%error, "daemon unavailable; continuing with local runtime");
            None
        }
    };
    #[cfg(unix)]
    let mut next_daemon_reconnect = Instant::now() + client_runtime::DAEMON_RECONNECT_INTERVAL;
    let daemon_attached = daemon_runtime.is_some();
    if daemon_attached && let Some(attached_build_dir) = app.workspace.build_dir.clone() {
        build_dir = attached_build_dir;
        build_dir_configured = true;
    }
    if build_dir_configured && let Some(root) = project_profile_root(&build_dir) {
        let action = match load_project_profile(&root) {
            Ok(Some(profile)) => Action::ProjectProfileLoaded(profile),
            Ok(None) => Action::ProjectProfileAbsent,
            Err(error) => Action::ProjectProfileLoadFailed(error.to_string()),
        };
        let _ = update(&mut app, action);
    }
    if build_dir_configured {
        app.screen = session.last_screen.unwrap_or(Screen::Dashboard);
    } else {
        app.screen = Screen::BuildEnvironment;
        app.focus = yoctui_model::FocusTarget::Navigator;
    }
    install_session_onboarding(&session, &mut app)?;
    app.logs.filter = session.log_filter;
    app.logs.recipe_filter = session.log_recipe_filter.clone();
    app.logs.task_filter = session.log_task_filter.clone();
    app.logs.build_filter = session.log_build_filter.clone();
    let session_build_dir = build_dir.clone();
    // Offline browsing must not spawn BitBake or refresh live metadata.
    let mut backend: Box<dyn BitBakeBackend> = Box::new(ProcessBackend::new(build_dir.clone()));
    if startup_metadata_authority(daemon_attached) == StartupMetadataAuthority::OfflineFiles {
        if build_dir_configured {
            if let Some(source) =
                project_profile_root(&build_dir).filter(|p| p.join("oe-init-build-env").is_file())
            {
                app.workspace.source_dir = Some(source.clone());
                app.build_environment = yoctui_model::BuildEnvironmentState::Configured(
                    yoctui_model::BuildEnvironmentProfile {
                        init_script: source.join("oe-init-build-env"),
                        source_dir: source,
                        build_dir: build_dir.clone(),
                    },
                );
            }
            app.notification = Some("Offline files and saved builds are available. Start the daemon in your initialized Yocto shell; connection retries automatically.".into());
        } else {
            app.notification = Some("Build environment: press e to configure paths or b to browse; F3 opens saved builds.".into());
        }
    }
    if !targets.is_empty() {
        app.build.target = targets.first().cloned()
    }
    let mut build_jobs = BuildJobCoordinator::default();
    let mut devtool_jobs = DevtoolJobCoordinator::default();
    let mut devtool_runner = None;
    let mut pending_devtool_modify = None;
    let mut pending_daemon_devtool_modify = None;
    let mut pending_devtool_update = None;
    let mut pending_devtool_finish = None;
    let mut pending_devtool_deploy = None;
    let mut pending_devtool_reset = None;
    let signature_adapter = SignatureAdapter::new(session_build_dir.clone());
    let mut signature_operation = None;
    let package_adapter = PackageDataAdapter::new(session_build_dir.clone());
    let mut package_operation = None;
    let image_artifact_adapter = app
        .workspace
        .variables
        .get("DEPLOY_DIR_IMAGE")
        .map(PathBuf::from)
        .map(ImageArtifactAdapter::new);
    let mut image_artifact_operation = None;
    let mut rootfs_composition_operation = None;
    let mut global_content_search_operation = None;
    let history_root = daemon_state_root()?;
    let mut history_load = None;
    app.saved_builds.reload_requested = true;
    let mut clone_operation = None;
    let mut source_git_poller = source_git::SourceGitPoller::default();
    let mut environment_operation = None;
    let sdk_artifact_adapter = app
        .workspace
        .variables
        .get("SDK_DEPLOY")
        .map(PathBuf::from)
        .map(SdkArtifactAdapter::new);
    let sdk_tool_adapter = match sdk_tool_adapter_for_workspace(&app, &session_build_dir) {
        Ok(adapter) => Some(adapter),
        Err(message) => {
            let _ = update(
                &mut app,
                Action::SdkToolCapabilityLoaded(SdkToolCapability::Failed { message }),
            );
            None
        }
    };
    let mut sdk_artifact_operation = None;
    let mut sdk_capability_operation = None;
    let mut sdk_operation = None;
    let mut pending_sdk_build = None;
    let qemu_inspector = QemuCapabilityInspector::default();
    let mut qemu_operation = None;
    let wic_inspector = wic_capability_inspector(&app);
    let mut wic_capability_operation = None;
    let wic_device_inspector = WicDeviceInspector::default();
    let mut wic_device_operation = None;
    let mut wic_operation = None;
    let initialized_paths = initialized_path_directories();
    let mut test_coordinator = TestCliCoordinator::new(
        session_build_dir.clone(),
        initialized_paths.clone(),
        ptest_capability(&app),
    );
    let mut pending_test_build = None;
    let mut security_coordinator =
        SecurityCliCoordinator::new(session_build_dir.clone(), initialized_paths.clone());
    let mut pending_security_build = None;
    let mut qa_coordinator =
        QaCliCoordinator::new(session_build_dir.clone(), initialized_paths.clone());
    let mut pending_qa_build = None;
    let maintenance_build_dir = if app.build_environment.connected() {
        session_build_dir.clone()
    } else {
        std::env::current_dir().unwrap_or_else(|_| std::env::temp_dir())
    };
    let mut maintenance_coordinator =
        MaintenanceCliCoordinator::new(&app, &maintenance_build_dir, initialized_paths)
            .map_err(anyhow::Error::msg)?;
    if app.screen == Screen::Packages
        && let Some(effect @ Effect::GetPackageInventory(_)) =
            compatibility_workspace_action(&mut app, Action::BeginPackageInventory)
    {
        begin_package_operation(&mut app, &package_adapter, &mut package_operation, effect);
    }
    if app.screen == Screen::Images
        && let Some(effect @ Effect::GetImageArtifacts(_)) =
            compatibility_workspace_action(&mut app, Action::BeginImageArtifactInventory)
    {
        begin_image_artifact_operation(
            &mut app,
            image_artifact_adapter.as_ref(),
            &mut image_artifact_operation,
            effect,
        );
    }
    if app.screen == Screen::Kernel
        && let Some(Effect::InspectKernel) =
            compatibility_workspace_action(&mut app, Action::InspectKernel)
    {
        inspect_kernel_workbench(&mut app, backend.as_mut()).await;
    }
    if app.screen == Screen::Firmware
        && let Some(Effect::InspectFirmware) =
            compatibility_workspace_action(&mut app, Action::InspectFirmware)
    {
        inspect_firmware_workbench(&mut app, backend.as_mut()).await;
    }
    if app.screen == Screen::Testing
        && let Some(effect) =
            compatibility_workspace_action(&mut app, Action::InspectTestCapability)
    {
        let _ = test_coordinator.handle_effect(&mut app, effect).await;
    }
    if app.screen == Screen::Security
        && let Some(effect) = compatibility_workspace_action(
            &mut app,
            Action::Security(SecurityAction::InspectCapability),
        )
    {
        let _ = security_coordinator.handle_effect(&mut app, effect).await;
    }
    if app.screen == Screen::Qa
        && let Some(effect) =
            compatibility_workspace_action(&mut app, Action::Qa(QaAction::InspectCapability))
    {
        let _ = qa_coordinator.handle_effect(&mut app, effect).await;
    }
    if app.screen == Screen::Maintenance
        && let Some(effect) = compatibility_workspace_action(
            &mut app,
            Action::Maintenance(yoctui_model::MaintenanceAction::InspectCapability),
        )
    {
        let _ = maintenance_coordinator
            .handle_effect(&mut app, effect)
            .await;
    }
    let mut telemetry_sampler = HostTelemetrySampler::default();
    let mut next_telemetry_sample = Instant::now();
    let mut telemetry_was_visible = client_telemetry_visible(&app);
    let mut next_animation_tick = Instant::now();
    let mut next_elapsed_refresh = Instant::now();
    let frame_interval = interactive_frame_interval(refresh);
    let mut render_scheduler = RenderScheduler::default();
    let mut environment_browser_io = environment_setup::EnvironmentBrowserIo::default();
    let render_measurement_started = Instant::now();
    let mut prefix_state = PrefixState::default();
    #[cfg(unix)]
    let mut termination = termination_receiver()?;
    loop {
        let (internal_records, ingress_dropped) = internal_tracing_capture.drain(256);
        render_scheduler.invalidate_if(
            environment_browser_io.poll(&mut app).await,
            RenderCause::State,
        );
        if ingress_dropped > 0 {
            let _ = update(&mut app, Action::InternalLogIngressDropped(ingress_dropped));
            render_scheduler.invalidate(RenderCause::State);
        }
        if !internal_records.is_empty() {
            render_scheduler.invalidate(RenderCause::State);
        }
        for record in internal_records {
            let _ = update(&mut app, Action::InternalLog(record));
        }
        #[cfg(unix)]
        if termination_requested(&mut termination) {
            break;
        }
        #[cfg(unix)]
        let daemon_poll_result = daemon_runtime
            .as_mut()
            .map(|runtime| runtime.poll(&mut app));
        #[cfg(unix)]
        let daemon_poll_error = match daemon_poll_result {
            Some(Ok(changed)) => {
                render_scheduler.invalidate_if(changed, RenderCause::State);
                None
            }
            Some(Err(error)) => Some(error),
            None => None,
        };
        #[cfg(unix)]
        if let Some(error) = daemon_poll_error {
            tracing::warn!(%error, "yoctui daemon client disconnected; reconnecting");
            daemon_runtime = None;
            app.daemon.status = yoctui_model::ClientReplicaStatus::Disconnected;
            yoctui_model::invalidate_workspace_compatibility(&mut app);
            let _ = update(
                &mut app,
                Action::BuildAuthorityLost {
                    message: error.to_string(),
                },
            );
            render_scheduler.invalidate(RenderCause::State);
            next_daemon_reconnect = Instant::now() + client_runtime::DAEMON_RECONNECT_INTERVAL;
        }
        #[cfg(unix)]
        if daemon_runtime.is_none() && Instant::now() >= next_daemon_reconnect {
            next_daemon_reconnect = Instant::now() + client_runtime::DAEMON_RECONNECT_INTERVAL;
            match client_runtime::InteractiveDaemonRuntime::connect(
                &mut app,
                Duration::from_millis(250),
            ) {
                Ok(runtime) => {
                    daemon_runtime = Some(runtime);
                    render_scheduler.invalidate(RenderCause::State);
                }
                Err(error) => tracing::debug!(%error, "daemon reattach not yet available"),
            }
        }
        if let Some(identity) = pending_daemon_devtool_modify.as_ref() {
            match daemon_devtool_modify_completion(&app, identity) {
                DaemonDevtoolModifyCompletion::Pending => {}
                DaemonDevtoolModifyCompletion::Succeeded => {
                    let identity = pending_daemon_devtool_modify
                        .take()
                        .expect("daemon Devtool identity was present");
                    complete_devtool_modify(&mut app, &session_build_dir, identity).await;
                    render_scheduler.invalidate(RenderCause::State);
                }
                DaemonDevtoolModifyCompletion::Failed => {
                    let identity = pending_daemon_devtool_modify
                        .take()
                        .expect("daemon Devtool identity was present");
                    app.notification = Some(format!(
                        "Devtool modify {} failed in the daemon; inspect Jobs and Logs.",
                        identity.name
                    ));
                    render_scheduler.invalidate(RenderCause::State);
                }
            }
        }
        if build_archive::poll_load(&mut app, &mut history_load, &history_root).await {
            render_scheduler.invalidate(RenderCause::State);
        }
        if source_git_poller.poll(&mut app).await {
            render_scheduler.invalidate(RenderCause::State);
        }
        let local_operation_active = history_load.is_some()
            || environment_operation.is_some()
            || clone_operation.is_some()
            || signature_operation.is_some()
            || package_operation.is_some()
            || image_artifact_operation.is_some()
            || rootfs_composition_operation.is_some()
            || global_content_search_operation.is_some()
            || sdk_artifact_operation.is_some()
            || sdk_capability_operation.is_some()
            || sdk_operation.is_some()
            || qemu_operation.is_some()
            || wic_capability_operation.is_some()
            || wic_device_operation.is_some()
            || wic_operation.is_some()
            || test_coordinator.session.is_some()
            || test_coordinator.import.is_some()
            || test_coordinator.result.is_some()
            || security_coordinator.capability.is_some()
            || security_coordinator.report.is_some()
            || security_coordinator.mapper.is_some()
            || qa_coordinator.capability.is_some()
            || qa_coordinator.layer_capability.is_some()
            || qa_coordinator.report.is_some()
            || qa_coordinator.layer.is_some()
            || maintenance_coordinator.operation_active();
        environment_operation::poll(&mut app, &mut backend, &mut environment_operation).await;
        for (activity, active) in [
            (
                yoctui_model::BackgroundActivity::Initializing,
                environment_operation.is_some(),
            ),
            (
                yoctui_model::BackgroundActivity::Cancelling,
                app.build.status == yoctui_model::BuildStatus::Cancelling,
            ),
            (
                yoctui_model::BackgroundActivity::Loading,
                local_operation_active
                    && clone_operation.is_none()
                    && environment_operation.is_none(),
            ),
        ] {
            compatibility_workspace_action(
                &mut app,
                Action::SetBackgroundActivity { activity, active },
            );
        }
        clone_operation::poll(&mut app, &mut clone_operation).await;
        poll_signature_operation(&mut app, &mut signature_operation).await;
        poll_package_operation(&mut app, &mut package_operation).await;
        poll_image_artifact_operation(
            &mut app,
            &mut image_artifact_operation,
            &qemu_inspector,
            &wic_inspector,
            &mut wic_capability_operation,
        )
        .await;
        poll_rootfs_composition_operation(&mut app, &mut rootfs_composition_operation).await;
        poll_global_content_search(&mut app, &mut global_content_search_operation).await;
        poll_sdk_artifact_operation(&mut app, &mut sdk_artifact_operation).await;
        poll_sdk_capability_operation(&mut app, &mut sdk_capability_operation).await;
        if let Some(completed) = poll_sdk_job(&mut app, &mut sdk_operation).await
            && matches!(completed, SdkOperation::Publish(_))
            && let Some(effect) = update(&mut app, Action::RefreshSdkArtifactInventory)
        {
            begin_sdk_artifact_operation(
                &mut app,
                sdk_artifact_adapter.as_ref(),
                &mut sdk_artifact_operation,
                effect,
            );
        }
        poll_wic_capability_operation(&mut app, &wic_inspector, &mut wic_capability_operation)
            .await;
        poll_wic_device_operation(&mut app, &mut wic_device_operation).await;
        poll_qemu_job(&mut app, &mut qemu_operation).await;
        poll_wic_job(&mut app, &mut wic_operation).await;
        test_coordinator.poll(&mut app).await;
        security_coordinator.poll(&mut app).await;
        qa_coordinator.poll(&mut app).await;
        maintenance_coordinator.poll(&mut app).await;
        render_scheduler.invalidate_if(local_operation_active, RenderCause::Presentation);
        let telemetry_now = Instant::now();
        let telemetry_visible = client_telemetry_visible(&app);
        if telemetry_visible != telemetry_was_visible {
            next_telemetry_sample = telemetry_now;
            telemetry_was_visible = telemetry_visible;
        }
        if telemetry_now >= next_telemetry_sample {
            let telemetry = telemetry_sampler.sample(&session_build_dir);
            let telemetry_changed = telemetry != app.host_telemetry;
            let _ = update(&mut app, Action::HostTelemetryUpdated(telemetry));
            next_telemetry_sample = telemetry_now + client_telemetry_interval(&app);
            render_scheduler.invalidate_if(
                telemetry_visible && telemetry_changed,
                RenderCause::Telemetry,
            );
        }
        let presentation_now = Instant::now();
        let visible_animation = has_visible_indeterminate_activity(&app);
        if visible_animation && presentation_now >= next_animation_tick {
            let _ = update(&mut app, Action::Tick);
            next_animation_tick = presentation_now + animation_interval(&app);
            render_scheduler.invalidate(RenderCause::Presentation);
        } else if !visible_animation {
            // Returning to an animated workspace shows activity immediately,
            // without accumulating hidden animation phases.
            next_animation_tick = presentation_now;
        }
        let live_elapsed = has_live_elapsed_time(&app);
        if live_elapsed && presentation_now >= next_elapsed_refresh {
            next_elapsed_refresh = presentation_now + ELAPSED_REFRESH_INTERVAL;
            render_scheduler.invalidate(RenderCause::Presentation);
        } else if !live_elapsed {
            next_elapsed_refresh = presentation_now;
        }
        if guard.take_full_redraw_request() {
            terminal.clear()?;
            render_scheduler.invalidate(RenderCause::Resize);
        }
        if app.screen == Screen::TerminalSessions
            && app.selected_terminal_is_menuconfig()
            && app.terminal.mode == yoctui_model::TerminalWorkbenchMode::Live
            && let Some(runtime) = daemon_runtime.as_mut()
        {
            let size = terminal.size()?;
            if let Some(dimensions) =
                yoctui_app::terminal_workspace_dimensions(&app, size.width, size.height)
                && runtime.resize_selected_terminal(&app, dimensions)?
            {
                render_scheduler.invalidate(RenderCause::Resize);
            }
        }
        if render_scheduler.take_frame_with_interval(ordinary_frame_interval(&app)) {
            terminal.draw(|f| render(f, &app))?;
        }
        if event::poll(frame_interval)? {
            let terminal_event = event::read()?;
            render_scheduler.invalidate(RenderCause::Input);
            if let Event::Paste(text) = terminal_event {
                if matches!(app.active_dialog(), Some(Dialog::EnvironmentSetup(_))) {
                    let _ = update(
                        &mut app,
                        Action::EnvironmentSetup(yoctui_model::EnvironmentSetupAction::Insert(
                            text,
                        )),
                    );
                    continue;
                }
                if app.screen == Screen::TerminalSessions
                    && app.active_dialog().is_none()
                    && !app.menu.is_open()
                    && !app.command_palette_open
                {
                    let _ =
                        compatibility_workspace_action(&mut app, Action::TerminalStagePaste(text));
                    continue;
                }
                if matches!(
                    app.active_dialog(),
                    Some(Dialog::RecipeEditor(editor))
                        if editor.focus == yoctui_model::RecipeEditorFocus::Document
                            && editor.document.editing
                ) {
                    let _ = compatibility_workspace_action(
                        &mut app,
                        Action::EditRecipeEditor(yoctui_model::PopupEditorCommand::PasteText {
                            text,
                            source: yoctui_model::TextAreaPasteSource::BracketedPaste,
                        }),
                    );
                    continue;
                }
                if active_popup_accepts_paste(&app) {
                    let _ = compatibility_workspace_action(
                        &mut app,
                        Action::EditActivePopup(yoctui_model::PopupEditorCommand::PasteText {
                            text,
                            source: yoctui_model::TextAreaPasteSource::BracketedPaste,
                        }),
                    );
                    continue;
                }
                if matches!(
                    app.active_dialog(),
                    Some(Dialog::Security(yoctui_model::SecurityDialog::Import { editor, .. }))
                        if editor.editing
                ) {
                    for character in text.chars().filter(|character| !character.is_control()) {
                        let _ = compatibility_workspace_action(
                            &mut app,
                            Action::EditActivePopup(yoctui_model::PopupEditorCommand::Insert(
                                character,
                            )),
                        );
                    }
                    continue;
                }
                if matches!(
                    app.active_dialog(),
                    Some(Dialog::Qa(yoctui_model::QaDialog::Import { editor, .. }))
                        if editor.editing
                ) {
                    for character in text.chars().filter(|character| !character.is_control()) {
                        let _ = compatibility_workspace_action(
                            &mut app,
                            Action::EditActivePopup(yoctui_model::PopupEditorCommand::Insert(
                                character,
                            )),
                        );
                    }
                    continue;
                }
                if matches!(
                    app.active_dialog(),
                    Some(Dialog::Maintenance(dialog))
                        if matches!(
                            dialog.as_ref(),
                            yoctui_model::MaintenanceDialog::ReadinessToml { editor, .. }
                                | yoctui_model::MaintenanceDialog::CleanupToml { editor, .. }
                                | yoctui_model::MaintenanceDialog::PrServiceToml { editor, .. }
                                | yoctui_model::MaintenanceDialog::LockedCacheToml { editor, .. }
                                | yoctui_model::MaintenanceDialog::BuildHistoryToml { editor, .. }
                                | yoctui_model::MaintenanceDialog::GitArchiveToml { editor, .. }
                                if editor.editing
                        )
                ) {
                    for character in text.chars().filter(|character| !character.is_control()) {
                        let _ = compatibility_workspace_action(
                            &mut app,
                            Action::EditActivePopup(yoctui_model::PopupEditorCommand::Insert(
                                character,
                            )),
                        );
                    }
                    continue;
                }
                let popup_action = match app.active_dialog() {
                    Some(Dialog::BuildEnvironmentEditor(editor)) if editor.editing => {
                        Some(Action::AppendBuildEnvironmentEditor as fn(char) -> Action)
                    }
                    Some(Dialog::BuildEnvironmentCloneEditor(editor)) if editor.editing => {
                        Some(Action::AppendBuildEnvironmentCloneEditor as fn(char) -> Action)
                    }
                    Some(Dialog::ConfigEdit { editor, .. }) if editor.editing => {
                        Some(Action::AppendConfigEdit as fn(char) -> Action)
                    }
                    Some(Dialog::BbmaskEdit(editor)) if editor.editing => {
                        Some(Action::AppendBbmask as fn(char) -> Action)
                    }
                    Some(Dialog::BuildTarget { editor, .. }) if editor.editing => {
                        Some(Action::AppendBuildTarget as fn(char) -> Action)
                    }
                    Some(Dialog::WicCreateTomlEditor { editor, .. }) if editor.editing => {
                        Some(Action::AppendWicCreateTomlEditor as fn(char) -> Action)
                    }
                    Some(Dialog::SdkPublishTomlEditor(editor)) if editor.editing => {
                        Some(Action::AppendSdkPublishTomlEditor as fn(char) -> Action)
                    }
                    Some(Dialog::SdkNativeTomlEditor(editor)) if editor.editing => {
                        Some(Action::AppendSdkNativeTomlEditor as fn(char) -> Action)
                    }
                    Some(Dialog::TestLaunchTomlEditor { editor, .. }) if editor.editing => {
                        Some(Action::AppendTestLaunchTomlEditor as fn(char) -> Action)
                    }
                    Some(Dialog::TestResultImportTomlEditor { editor, .. }) if editor.editing => {
                        Some(Action::AppendTestResultImportTomlEditor as fn(char) -> Action)
                    }
                    Some(Dialog::TestComparisonTomlEditor { editor, .. }) if editor.editing => {
                        Some(Action::AppendTestComparisonTomlEditor as fn(char) -> Action)
                    }
                    Some(Dialog::TestJunitTomlEditor { editor, .. }) if editor.editing => {
                        Some(Action::AppendTestJunitTomlEditor as fn(char) -> Action)
                    }
                    _ => None,
                };
                if let Some(action) = popup_action {
                    for character in text.chars().filter(|character| !character.is_control()) {
                        let _ = compatibility_workspace_action(&mut app, action(character));
                    }
                } else if app.screen == Screen::BuildEnvironment
                    && app
                        .build_environment_draft
                        .as_ref()
                        .is_some_and(|draft| draft.editing)
                {
                    for character in text.chars() {
                        let _ = compatibility_workspace_action(
                            &mut app,
                            Action::AppendBuildEnvironmentField(character),
                        );
                    }
                }
                continue;
            }
            if terminal_event_requires_full_redraw(&terminal_event) {
                terminal.clear()?;
                continue;
            }
            if let Event::Mouse(mouse) = terminal_event {
                let kind = mouse_kind_from_event(mouse.kind);
                let terminal_size = terminal.size()?;
                if let Some(kind) = kind
                    && let Some(action) = mouse_action_for_app(
                        MouseInput {
                            kind,
                            column: mouse.column,
                            row: mouse.row,
                        },
                        &app,
                        terminal_size.width,
                        terminal_size.height,
                    )
                {
                    match compatibility_workspace_action(&mut app, action) {
                        Some(Effect::InspectKernel) => {
                            inspect_kernel_workbench(&mut app, backend.as_mut()).await;
                        }
                        Some(Effect::InspectFirmware) => {
                            inspect_firmware_workbench(&mut app, backend.as_mut()).await;
                        }
                        Some(
                            effect @ (Effect::GetPackageInventory(_) | Effect::GetPackageDetail(_)),
                        ) => {
                            begin_package_operation(
                                &mut app,
                                &package_adapter,
                                &mut package_operation,
                                effect,
                            );
                        }
                        Some(effect @ Effect::GetImageArtifacts(_)) => {
                            begin_image_artifact_operation(
                                &mut app,
                                image_artifact_adapter.as_ref(),
                                &mut image_artifact_operation,
                                effect,
                            );
                        }
                        Some(effect @ Effect::GetRootfsComposition(_)) => {
                            begin_rootfs_composition_operation(
                                backend.as_mut(),
                                &mut app,
                                &session_build_dir,
                                &mut rootfs_composition_operation,
                                effect,
                                daemon_attached,
                            )
                            .await;
                        }
                        Some(Effect::LoadLayerBrowserDirectory {
                            layer,
                            root,
                            directory,
                        }) => {
                            load_layer_browser_directory(&mut app, layer, root, directory).await;
                        }
                        Some(Effect::LoadLayerBrowserPreview(path)) => {
                            load_layer_browser_preview(&mut app, path).await;
                        }
                        Some(Effect::OpenInEditor(path)) => {
                            open_in_editor(&guard, &mut app, path, editor.as_deref()).await;
                        }
                        Some(effect @ Effect::InspectSdkTools) => {
                            begin_sdk_capability_operation(
                                &mut app,
                                sdk_tool_adapter.as_ref(),
                                &mut sdk_capability_operation,
                                effect,
                            );
                        }
                        _ => {}
                    }
                }
                continue;
            }
            if let Event::Key(k) = terminal_event {
                let Some(mut input) = input_from_key(k) else {
                    continue;
                };
                if input == Input::Esc
                    && app.screen == Screen::BuildEnvironment
                    && app.active_dialog().is_none()
                    && !app.menu.is_open()
                    && clone_operation.is_some()
                {
                    clone_operation::cancel(&mut app, &mut clone_operation);
                    render_scheduler.invalidate(RenderCause::State);
                    continue;
                }
                if let Some(Dialog::EnvironmentSetup(setup)) = app.active_dialog() {
                    if let Some(action) = yoctui_app::environment_setup_action(setup, input)
                        && let Some(effect) = compatibility_workspace_action(&mut app, action)
                    {
                        environment_browser_io.submit(effect);
                    }
                    continue;
                }
                if app.active_dialog().is_none() && !app.menu.is_open() && !app.onboarding.open {
                    match prefix_state.feed(input, Instant::now()) {
                        PrefixEvent::Awaiting => {
                            app.notification = Some(
                                "Prefix Ctrl+B: t terminals | c create | n/p session | %/\" split | z zoom | [ copy | / search | d detach | : palette | ? help"
                                    .into(),
                            );
                            continue;
                        }
                        PrefixEvent::Command(command) => {
                            if matches!(
                                command,
                                PrefixCommand::CreateSession | PrefixCommand::TakeControl
                            ) {
                                if let Some(runtime) = daemon_runtime.as_mut() {
                                    if let Err(error) = runtime.route_prefix(&app, command) {
                                        app.notification =
                                            Some(format!("Prefix command failed: {error}"));
                                    }
                                } else {
                                    app.notification =
                                        Some("Daemon is unavailable for terminal sessions.".into());
                                }
                            }
                            if command == PrefixCommand::Detach
                                && let Some(runtime) = daemon_runtime.as_mut()
                                && let Err(error) = runtime.detach_terminal(&app)
                            {
                                app.notification = Some(format!("Terminal detach failed: {error}"));
                            }
                            if command == PrefixCommand::OpenTerminalSessions {
                                let _ = compatibility_workspace_action(
                                    &mut app,
                                    Action::Open(Screen::TerminalSessions),
                                );
                            }
                            let terminal_action = match command {
                                PrefixCommand::CopyMode => Some(Action::TerminalEnterCopyMode),
                                PrefixCommand::Search => Some(Action::TerminalBeginSearch),
                                PrefixCommand::Rename => Some(Action::TerminalBeginRename),
                                PrefixCommand::ReleaseControl => {
                                    Some(Action::TerminalReleaseControl)
                                }
                                PrefixCommand::Kill => Some(Action::TerminalBeginKill),
                                PrefixCommand::Zoom => Some(Action::TogglePaneZoom),
                                _ => None,
                            };
                            if let Some(action) = terminal_action
                                && let Some(effect @ Effect::Terminal(_)) =
                                    compatibility_workspace_action(&mut app, action)
                            {
                                let _ =
                                    submit_daemon_effect(&mut daemon_runtime, &mut app, &effect);
                            }
                            if command == PrefixCommand::NextSession {
                                let _ = compatibility_workspace_action(
                                    &mut app,
                                    Action::SelectPtySession { delta: 1 },
                                );
                            } else if command == PrefixCommand::PreviousSession {
                                let _ = compatibility_workspace_action(
                                    &mut app,
                                    Action::SelectPtySession { delta: -1 },
                                );
                            }
                            if command == PrefixCommand::SplitHorizontal
                                || command == PrefixCommand::SplitVertical
                            {
                                let axis = if command == PrefixCommand::SplitHorizontal {
                                    yoctui_model::SplitAxis::Horizontal
                                } else {
                                    yoctui_model::SplitAxis::Vertical
                                };
                                match app.pane_layout.split(app.pane_layout.focused, axis) {
                                    Ok(_) => app.notification = Some("Terminal pane split".into()),
                                    Err(error) => {
                                        app.notification =
                                            Some(format!("Pane split failed: {error}"))
                                    }
                                }
                            }
                            if command == PrefixCommand::ClosePane {
                                match app.pane_layout.close(app.pane_layout.focused) {
                                    Ok(_) => app.notification = Some("Terminal pane closed".into()),
                                    Err(error) => {
                                        app.notification =
                                            Some(format!("Pane close failed: {error}"))
                                    }
                                }
                            }
                            app.notification = Some(match command {
                                PrefixCommand::CommandPalette => {
                                    let _ = compatibility_workspace_action(
                                        &mut app,
                                        Action::OpenCommandPalette,
                                    );
                                    "Command palette opened".into()
                                }
                                PrefixCommand::Help => {
                                    let action = if app.screen == Screen::TerminalSessions {
                                        Action::TerminalToggleHelp
                                    } else {
                                        Action::Open(Screen::Help)
                                    };
                                    let _ = compatibility_workspace_action(&mut app, action);
                                    "Help opened".into()
                                }
                                PrefixCommand::CreateSession => {
                                    "Create terminal session requested".into()
                                }
                                PrefixCommand::NextSession => "Next terminal session".into(),
                                PrefixCommand::PreviousSession => {
                                    "Previous terminal session".into()
                                }
                                PrefixCommand::SplitHorizontal => {
                                    "Horizontal split requested".into()
                                }
                                PrefixCommand::SplitVertical => "Vertical split requested".into(),
                                PrefixCommand::ClosePane => "Terminal pane close requested".into(),
                                PrefixCommand::Detach => "Detached from terminal session".into(),
                                PrefixCommand::TakeControl => "PTY writer control requested".into(),
                                PrefixCommand::OpenTerminalSessions => {
                                    "Terminal Sessions opened".into()
                                }
                                PrefixCommand::CopyMode => "Terminal copy mode opened".into(),
                                PrefixCommand::Search => "Terminal search opened".into(),
                                PrefixCommand::Rename => "Terminal rename opened".into(),
                                PrefixCommand::ReleaseControl => {
                                    "Terminal writer release requested".into()
                                }
                                PrefixCommand::Kill => "Terminal kill confirmation opened".into(),
                                PrefixCommand::Zoom => "Terminal pane zoom toggled".into(),
                            });
                            continue;
                        }
                        PrefixEvent::Literal(next) => input = next,
                    }
                }
                let mut replayed_context_action = false;
                if app.menu.is_open() {
                    match menu_action(&app, input) {
                        Some(MenuInputResult::Reduce(action)) => {
                            let _ = compatibility_workspace_action(&mut app, *action);
                            continue;
                        }
                        Some(MenuInputResult::ActivateCommand(command)) => {
                            let _ = compatibility_workspace_action(&mut app, Action::CloseMenu);
                            let action = yoctui_model::command_action(&app, command);
                            let _ = compatibility_workspace_action(&mut app, action);
                            continue;
                        }
                        Some(MenuInputResult::ActivateContext(replay)) => {
                            let _ = compatibility_workspace_action(&mut app, Action::CloseMenu);
                            input = replay;
                            replayed_context_action = true;
                        }
                        None => continue,
                    }
                }
                if app.onboarding.open {
                    let effect = onboarding_action(&app, input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                    match effect {
                        Some(Effect::PersistOnboarding) => {
                            let result =
                                persist_onboarding(session_path.as_deref(), &mut session, &app);
                            let action = match result {
                                Ok(()) => Action::OnboardingPersisted,
                                Err(error) => {
                                    Action::OnboardingPersistenceFailed(error.to_string())
                                }
                            };
                            let _ = compatibility_workspace_action(&mut app, action);
                        }
                        Some(effect @ Effect::GetImageArtifacts(_)) => {
                            begin_image_artifact_operation(
                                &mut app,
                                image_artifact_adapter.as_ref(),
                                &mut image_artifact_operation,
                                effect,
                            );
                        }
                        _ => {}
                    }
                    continue;
                }
                if let Some(action) = yoctui_app::saved_build_workspace_action(&app, input) {
                    let _ = compatibility_workspace_action(&mut app, action);
                    continue;
                }
                if let Some(action) = notification_popup_action(&app, input) {
                    let _ = compatibility_workspace_action(&mut app, action);
                    continue;
                }
                if let Some(action) =
                    direct_menu_shortcut_action(&app, input, replayed_context_action)
                {
                    let _ = compatibility_workspace_action(&mut app, action);
                    continue;
                }
                if app.command_palette_open {
                    let global_search_edit = app.command_palette_mode
                        == yoctui_model::CommandPaletteMode::GlobalRegexSearch
                        && matches!(input, Input::Backspace | Input::CtrlU | Input::Char(_));
                    let global_search_close = app.command_palette_mode
                        == yoctui_model::CommandPaletteMode::GlobalRegexSearch
                        && input == Input::Esc;
                    let effect = match input {
                        Input::Up => compatibility_workspace_action(
                            &mut app,
                            Action::SelectCommandPalette { delta: -1 },
                        ),
                        Input::Down => compatibility_workspace_action(
                            &mut app,
                            Action::SelectCommandPalette { delta: 1 },
                        ),
                        Input::Enter => {
                            compatibility_workspace_action(&mut app, Action::ActivateCommandPalette)
                        }
                        Input::Esc => {
                            compatibility_workspace_action(&mut app, Action::CloseCommandPalette)
                        }
                        Input::Backspace => compatibility_workspace_action(
                            &mut app,
                            Action::BackspaceCommandPaletteQuery,
                        ),
                        Input::CtrlU => compatibility_workspace_action(
                            &mut app,
                            Action::ClearCommandPaletteQuery,
                        ),
                        Input::Char(character) => compatibility_workspace_action(
                            &mut app,
                            Action::AppendCommandPaletteQuery(character),
                        ),
                        _ => None,
                    };
                    if let Some(effect @ Effect::GetImageArtifacts(_)) = effect {
                        begin_image_artifact_operation(
                            &mut app,
                            image_artifact_adapter.as_ref(),
                            &mut image_artifact_operation,
                            effect,
                        );
                    } else if let Some(effect @ Effect::GetRootfsComposition(_)) = effect {
                        begin_rootfs_composition_operation(
                            backend.as_mut(),
                            &mut app,
                            &session_build_dir,
                            &mut rootfs_composition_operation,
                            effect,
                            daemon_attached,
                        )
                        .await;
                    } else if let Some(
                        effect @ (Effect::GetPackageInventory(_) | Effect::GetPackageDetail(_)),
                    ) = effect
                    {
                        begin_package_operation(
                            &mut app,
                            &package_adapter,
                            &mut package_operation,
                            effect,
                        );
                    } else if let Some(effect @ Effect::InspectSdkTools) = effect {
                        begin_sdk_capability_operation(
                            &mut app,
                            sdk_tool_adapter.as_ref(),
                            &mut sdk_capability_operation,
                            effect,
                        );
                    } else if let Some(
                        effect @ (Effect::InspectTestCapability
                        | Effect::InspectResultToolCapability),
                    ) = effect
                    {
                        let _ = test_coordinator.handle_effect(&mut app, effect).await;
                    } else if let Some(effect @ Effect::Security(_)) = effect {
                        let _ = route_independent_security_effect(
                            &guard,
                            &mut app,
                            &mut security_coordinator,
                            effect,
                            editor.as_deref(),
                        )
                        .await;
                    } else if let Some(effect @ Effect::Qa(_)) = effect {
                        let _ = route_independent_qa_effect(
                            &guard,
                            &mut app,
                            &mut qa_coordinator,
                            effect,
                            editor.as_deref(),
                        )
                        .await;
                    } else if let Some(effect @ Effect::Maintenance(_)) = effect {
                        let _ = route_independent_maintenance_effect(
                            &guard,
                            &mut app,
                            &mut maintenance_coordinator,
                            effect,
                            editor.as_deref(),
                        )
                        .await;
                    } else if let Some(Effect::OpenInEditor(path)) = effect {
                        open_in_editor(&guard, &mut app, path, editor.as_deref()).await;
                    }
                    if global_search_edit {
                        begin_global_content_search(
                            &mut app,
                            &session_build_dir,
                            &mut global_content_search_operation,
                        );
                    } else if global_search_close
                        && let Some(operation) = global_content_search_operation.take()
                    {
                        operation.cancellation.cancel();
                    }
                } else if let Some(action) = global_search_action(&app, input) {
                    let _ = compatibility_workspace_action(&mut app, action);
                } else if app.keymap_preferences_ui.open {
                    let effect = keymap_preferences_action(&app, input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                    match effect {
                        Some(Effect::PersistSettings) => {
                            let result = persist_settings(
                                session_path.as_deref(),
                                &mut session,
                                &app,
                                !color_forced_off,
                            );
                            let action = match result {
                                Ok(()) => Action::SettingsPersisted,
                                Err(error) => Action::SettingsPersistenceFailed(error.to_string()),
                            };
                            let _ = compatibility_workspace_action(&mut app, action);
                        }
                        Some(Effect::CopyToClipboard(report)) => {
                            copy_to_clipboard(&mut app, report).await;
                        }
                        _ => {}
                    }
                } else if let Some(Dialog::BuildEnvironmentCloneEditor(editor)) =
                    app.active_dialog().cloned()
                {
                    let action = match input {
                        Input::Enter => Some(Action::ReviewBuildEnvironmentClone),
                        Input::Esc if !editor.editing => Some(Action::CancelBuildEnvironmentClone),
                        Input::Char('q') if !editor.editing => {
                            Some(Action::CancelBuildEnvironmentClone)
                        }
                        input => popup_editor_action(editor.editing, input),
                    };
                    if let Some(Effect::CopyToClipboard(content)) =
                        action.and_then(|action| compatibility_workspace_action(&mut app, action))
                    {
                        copy_to_clipboard(&mut app, content).await;
                    }
                } else if matches!(
                    app.active_dialog(),
                    Some(Dialog::BuildEnvironmentCloneReview(_))
                ) {
                    let action = match input {
                        Input::Enter => Some(Action::ConfirmBuildEnvironmentClone),
                        Input::Esc => Some(Action::CancelBuildEnvironmentClone),
                        _ => None,
                    };
                    if let Some(action) = action
                        && let Some(Effect::CloneBuildEnvironment(plan)) =
                            compatibility_workspace_action(&mut app, action)
                    {
                        clone_operation::start(&mut app, &mut clone_operation, plan);
                    }
                } else if let Some(Dialog::BuildEnvironmentEditor(editor)) =
                    app.active_dialog().cloned()
                {
                    let action = match input {
                        Input::Enter => Some(Action::ApplyBuildEnvironmentEditor),
                        Input::Char('q') | Input::Esc if !editor.editing => {
                            Some(Action::CloseBuildEnvironmentEditor)
                        }
                        input => popup_editor_action(editor.editing, input),
                    };
                    if let Some(Effect::CopyToClipboard(content)) =
                        action.and_then(|action| compatibility_workspace_action(&mut app, action))
                    {
                        copy_to_clipboard(&mut app, content).await;
                    }
                } else if matches!(app.active_dialog(), Some(Dialog::ThemePicker { .. })) {
                    let action = match input {
                        Input::Up | Input::Char('k') => Some(Action::SelectTheme { delta: -1 }),
                        Input::Down | Input::Char('j') => Some(Action::SelectTheme { delta: 1 }),
                        Input::Enter => Some(Action::ApplySelectedTheme),
                        Input::Esc => Some(Action::CloseThemePicker),
                        _ => None,
                    };
                    if let Some(action) = action
                        && let Some(Effect::PersistSettings) =
                            compatibility_workspace_action(&mut app, action)
                    {
                        let result = persist_settings(
                            session_path.as_deref(),
                            &mut session,
                            &app,
                            !color_forced_off,
                        );
                        let persistence_action = match result {
                            Ok(()) => Action::SettingsPersisted,
                            Err(error) => Action::SettingsPersistenceFailed(error.to_string()),
                        };
                        let _ = compatibility_workspace_action(&mut app, persistence_action);
                    }
                } else if let Some(Dialog::Maintenance(dialog)) = app.active_dialog().cloned() {
                    let effect = maintenance_dialog_action(&dialog, input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                    if let Some(effect) = effect {
                        if let Effect::CopyToClipboard(content) = effect {
                            copy_to_clipboard(&mut app, content).await;
                            continue;
                        }
                        let _ = route_independent_maintenance_effect(
                            &guard,
                            &mut app,
                            &mut maintenance_coordinator,
                            effect,
                            editor.as_deref(),
                        )
                        .await;
                    }
                } else if let Some(Dialog::Security(dialog)) = app.active_dialog().cloned() {
                    let effect = security_dialog_action(&dialog, input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                    if let Some(effect) = effect {
                        if let Effect::CopyToClipboard(content) = effect {
                            copy_to_clipboard(&mut app, content).await;
                            continue;
                        }
                        let routed = route_independent_security_effect(
                            &guard,
                            &mut app,
                            &mut security_coordinator,
                            effect.clone(),
                            editor.as_deref(),
                        )
                        .await;
                        if !routed {
                            match effect {
                                Effect::Security(SecurityEffect::StartBuild { id, request }) => {
                                    if begin_security_build(
                                        &mut backend,
                                        &mut app,
                                        &mut build_jobs,
                                        id,
                                        request,
                                    )
                                    .await
                                    {
                                        pending_security_build = Some(id);
                                    }
                                }
                                Effect::Security(SecurityEffect::CancelSession(id))
                                    if pending_security_build == Some(id) =>
                                {
                                    if let Some(action) = build_jobs.request_cancellation() {
                                        let _ = compatibility_workspace_action(&mut app, action);
                                    }
                                    if let Err(error) = backend.cancel_build().await {
                                        let _ = compatibility_workspace_action(
                                            &mut app,
                                            Action::Security(SecurityAction::RejectCancellation {
                                                id,
                                                message: error.to_string(),
                                            }),
                                        );
                                        for action in build_jobs.cancellation_failed(
                                            error.to_string(),
                                            SystemTime::now(),
                                        ) {
                                            let _ =
                                                compatibility_workspace_action(&mut app, action);
                                        }
                                    }
                                }
                                Effect::Security(SecurityEffect::CancelSession(id)) => {
                                    let _ = compatibility_workspace_action(
                                        &mut app,
                                        Action::Security(SecurityAction::RejectCancellation {
                                            id,
                                            message: "the CLI does not own this Security operation"
                                                .into(),
                                        }),
                                    );
                                }
                                _ => {}
                            }
                        }
                    }
                } else if let Some(Dialog::Qa(dialog)) = app.active_dialog().cloned() {
                    let effect = qa_dialog_action(&dialog, input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                    if let Some(effect) = effect {
                        if let Effect::CopyToClipboard(content) = effect {
                            copy_to_clipboard(&mut app, content).await;
                            continue;
                        }
                        let routed = route_independent_qa_effect(
                            &guard,
                            &mut app,
                            &mut qa_coordinator,
                            effect.clone(),
                            editor.as_deref(),
                        )
                        .await;
                        if !routed {
                            match effect {
                                Effect::Qa(QaEffect::StartBuild { session, request }) => {
                                    if begin_qa_build(
                                        &mut backend,
                                        &mut app,
                                        &mut build_jobs,
                                        session,
                                        request,
                                    )
                                    .await
                                    {
                                        pending_qa_build = Some(session);
                                    }
                                }
                                Effect::Qa(QaEffect::CancelBuild { session, .. })
                                    if pending_qa_build == Some(session) =>
                                {
                                    if let Some(action) = build_jobs.request_cancellation() {
                                        let _ = compatibility_workspace_action(&mut app, action);
                                    }
                                    if let Err(error) = backend.cancel_build().await {
                                        let _ = compatibility_workspace_action(
                                            &mut app,
                                            Action::Qa(QaAction::RejectCancellation {
                                                session,
                                                message: error.to_string(),
                                            }),
                                        );
                                        for action in build_jobs.cancellation_failed(
                                            error.to_string(),
                                            SystemTime::now(),
                                        ) {
                                            let _ =
                                                compatibility_workspace_action(&mut app, action);
                                        }
                                    }
                                }
                                Effect::Qa(QaEffect::CancelBuild { session, .. }) => {
                                    let _ = compatibility_workspace_action(
                                        &mut app,
                                        Action::Qa(QaAction::RejectCancellation {
                                            session,
                                            message: "the CLI does not own this QA managed build"
                                                .into(),
                                        }),
                                    );
                                }
                                _ => {}
                            }
                        }
                    }
                } else if matches!(app.active_dialog(), Some(Dialog::SdkBuildConfirmation(_))) {
                    let effect = sdk_build_confirmation_action(input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                    if let Some(Effect::Start(request)) = effect {
                        let tracked = sdk_build_is_populate(&request);
                        if begin_runtime_build(
                            &mut daemon_runtime,
                            &mut backend,
                            &mut app,
                            &mut build_jobs,
                            request.clone(),
                        )
                        .await
                            && tracked
                        {
                            pending_sdk_build = Some(request);
                        }
                    }
                } else if let Some(Dialog::SdkPublishTomlEditor(editor)) =
                    app.active_dialog().cloned()
                {
                    let action = match input {
                        Input::Enter => Some(Action::PreviewSdkPublish),
                        Input::Char('q') | Input::Esc if !editor.editing => {
                            Some(Action::CancelSdkPublish)
                        }
                        input => popup_editor_action(editor.editing, input),
                    };
                    if let Some(Effect::CopyToClipboard(content)) =
                        action.and_then(|action| compatibility_workspace_action(&mut app, action))
                    {
                        copy_to_clipboard(&mut app, content).await;
                    }
                } else if matches!(app.active_dialog(), Some(Dialog::SdkPublish(_))) {
                    let _ = sdk_publish_dialog_action(input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                } else if matches!(app.active_dialog(), Some(Dialog::SdkPublishConfirmation(_))) {
                    let effect = sdk_publish_confirmation_action(input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                    if let Some(effect @ Effect::StartSdkSession { .. }) = effect {
                        if submit_daemon_effect(&mut daemon_runtime, &mut app, &effect).is_some() {
                            continue;
                        }
                        let Effect::StartSdkSession { id, operation } = effect else {
                            unreachable!()
                        };
                        begin_sdk_job(
                            &mut app,
                            &mut sdk_operation,
                            sdk_tool_adapter.as_ref(),
                            cancellation_timeout,
                            SDK_TOOL_OPERATION_TIMEOUT,
                            id,
                            operation,
                        );
                    }
                } else if let Some(Dialog::SdkNativeTomlEditor(editor)) =
                    app.active_dialog().cloned()
                {
                    let action = match input {
                        Input::Enter => Some(Action::PreviewSdkNative),
                        Input::Char('q') | Input::Esc if !editor.editing => {
                            Some(Action::CancelSdkNative)
                        }
                        input => popup_editor_action(editor.editing, input),
                    };
                    if let Some(Effect::CopyToClipboard(content)) =
                        action.and_then(|action| compatibility_workspace_action(&mut app, action))
                    {
                        copy_to_clipboard(&mut app, content).await;
                    }
                } else if let Some(Dialog::SdkNative(dialog)) = app.active_dialog() {
                    let editing = dialog.editing;
                    let _ = sdk_native_dialog_action(editing, input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                } else if matches!(app.active_dialog(), Some(Dialog::SdkNativeConfirmation(_))) {
                    let effect = sdk_native_confirmation_action(input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                    if let Some(effect @ Effect::StartSdkSession { .. }) = effect {
                        if submit_daemon_effect(&mut daemon_runtime, &mut app, &effect).is_some() {
                            continue;
                        }
                        let Effect::StartSdkSession { id, operation } = effect else {
                            unreachable!()
                        };
                        begin_sdk_job(
                            &mut app,
                            &mut sdk_operation,
                            sdk_tool_adapter.as_ref(),
                            cancellation_timeout,
                            SDK_TOOL_OPERATION_TIMEOUT,
                            id,
                            operation,
                        );
                    }
                } else if matches!(
                    app.active_dialog(),
                    Some(Dialog::SdkCancellationConfirmation(_))
                ) {
                    let effect = sdk_cancellation_confirmation_action(input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                    if let Some(effect @ Effect::CancelSdkSession(_)) = effect {
                        if submit_daemon_effect(&mut daemon_runtime, &mut app, &effect).is_some() {
                            continue;
                        }
                        let Effect::CancelSdkSession(id) = effect else {
                            unreachable!()
                        };
                        begin_sdk_cancellation(&mut app, &mut sdk_operation, id);
                    }
                } else if let Some(Dialog::TestLaunchTomlEditor { editor, .. }) =
                    app.active_dialog().cloned()
                {
                    let action = match input {
                        Input::Enter => Some(Action::PreviewTestLaunch),
                        Input::Char('q') | Input::Esc if !editor.editing => {
                            Some(Action::CancelTestLaunch)
                        }
                        input => popup_editor_action(editor.editing, input),
                    };
                    if let Some(Effect::CopyToClipboard(content)) =
                        action.and_then(|action| compatibility_workspace_action(&mut app, action))
                    {
                        copy_to_clipboard(&mut app, content).await;
                    }
                } else if let Some(Dialog::TestLaunch(dialog)) = app.active_dialog() {
                    let editing = dialog.editing;
                    let _ = test_launch_dialog_action(editing, input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                } else if matches!(app.active_dialog(), Some(Dialog::TestLaunchConfirmation(_))) {
                    let effect = test_launch_confirmation_action(input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                    match effect {
                        Some(effect @ Effect::StartTestSession { .. }) => {
                            if submit_daemon_effect(&mut daemon_runtime, &mut app, &effect)
                                .is_none()
                            {
                                let _ = test_coordinator.handle_effect(&mut app, effect).await;
                            }
                        }
                        Some(Effect::StartTestBuildSession {
                            id,
                            family: _,
                            request,
                        }) => {
                            if begin_test_build(
                                &mut backend,
                                &mut app,
                                &mut build_jobs,
                                id,
                                request,
                            )
                            .await
                            {
                                pending_test_build = Some(id);
                            }
                        }
                        _ => {}
                    }
                } else if matches!(
                    app.active_dialog(),
                    Some(Dialog::TestCancellationConfirmation(_))
                ) {
                    let effect = test_cancellation_confirmation_action(input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                    if let Some(effect @ Effect::CancelTestSession(id)) = effect
                        && submit_daemon_effect(&mut daemon_runtime, &mut app, &effect).is_none()
                        && !test_coordinator.handle_effect(&mut app, effect).await
                        && pending_test_build == Some(id)
                    {
                        if let Some(action) = build_jobs.request_cancellation() {
                            let _ = compatibility_workspace_action(&mut app, action);
                        }
                        if let Err(error) = backend.cancel_build().await {
                            let _ = compatibility_workspace_action(
                                &mut app,
                                Action::RejectTestSessionCancellation {
                                    id,
                                    message: error.to_string(),
                                },
                            );
                            for action in
                                build_jobs.cancellation_failed(error.to_string(), SystemTime::now())
                            {
                                let _ = compatibility_workspace_action(&mut app, action);
                            }
                        }
                    }
                } else if let Some(Dialog::TestResultImportTomlEditor { editor, .. }) =
                    app.active_dialog().cloned()
                {
                    let action = match input {
                        Input::Enter => Some(Action::ConfirmTestResultImport),
                        Input::Char('q') | Input::Esc if !editor.editing => {
                            Some(Action::CancelTestResultImport)
                        }
                        input => popup_editor_action(editor.editing, input),
                    };
                    if let Some(effect) =
                        action.and_then(|action| compatibility_workspace_action(&mut app, action))
                    {
                        match effect {
                            Effect::CopyToClipboard(content) => {
                                copy_to_clipboard(&mut app, content).await;
                            }
                            effect => {
                                let _ = test_coordinator.handle_effect(&mut app, effect).await;
                            }
                        }
                    }
                } else if matches!(app.active_dialog(), Some(Dialog::TestResultImport(_))) {
                    let effect = test_result_import_dialog_action(input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                    if let Some(effect) = effect {
                        let _ = test_coordinator.handle_effect(&mut app, effect).await;
                    }
                } else if let Some(Dialog::TestComparisonTomlEditor { editor, .. }) =
                    app.active_dialog().cloned()
                {
                    let action = match input {
                        Input::Enter => Some(Action::PreviewTestComparison),
                        Input::Char('q') | Input::Esc if !editor.editing => {
                            Some(Action::CancelTestComparison)
                        }
                        input => popup_editor_action(editor.editing, input),
                    };
                    if let Some(Effect::CopyToClipboard(content)) =
                        action.and_then(|action| compatibility_workspace_action(&mut app, action))
                    {
                        copy_to_clipboard(&mut app, content).await;
                    }
                } else if matches!(app.active_dialog(), Some(Dialog::TestComparison(_))) {
                    let _ = test_comparison_dialog_action(input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                } else if matches!(
                    app.active_dialog(),
                    Some(Dialog::TestComparisonConfirmation(_))
                ) {
                    let effect = test_comparison_confirmation_action(input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                    if let Some(effect) = effect {
                        let _ = test_coordinator.handle_effect(&mut app, effect).await;
                    }
                } else if let Some(Dialog::TestJunitTomlEditor { editor, .. }) =
                    app.active_dialog().cloned()
                {
                    let action = if editor.editing {
                        match input {
                            Input::Esc => Some(Action::ToggleTestJunitTomlEditor),
                            Input::Enter => Some(Action::PreviewTestJunitExport),
                            Input::Backspace => Some(Action::BackspaceTestJunitTomlEditor),
                            Input::Left => Some(Action::MoveTestJunitTomlEditorLeft),
                            Input::Right => Some(Action::MoveTestJunitTomlEditorRight),
                            Input::Up => Some(Action::MoveTestJunitTomlEditorUp),
                            Input::Down => Some(Action::MoveTestJunitTomlEditorDown),
                            Input::Home => Some(Action::MoveTestJunitTomlEditorHome),
                            Input::End => Some(Action::MoveTestJunitTomlEditorEnd),
                            Input::CtrlC => Some(Action::CopyTestJunitTomlEditor),
                            Input::CtrlV => Some(Action::PasteTestJunitTomlEditor),
                            Input::Char(character) => {
                                Some(Action::AppendTestJunitTomlEditor(character))
                            }
                            _ => None,
                        }
                    } else {
                        match input {
                            Input::Char('i') => Some(Action::ToggleTestJunitTomlEditor),
                            Input::Char('e') => Some(Action::SelectTestJunitDestination),
                            Input::Left | Input::Char('h') => {
                                Some(Action::MoveTestJunitTomlEditorLeft)
                            }
                            Input::Right | Input::Char('l') => {
                                Some(Action::MoveTestJunitTomlEditorRight)
                            }
                            Input::Up | Input::Char('k') => Some(Action::MoveTestJunitTomlEditorUp),
                            Input::Down | Input::Char('j') => {
                                Some(Action::MoveTestJunitTomlEditorDown)
                            }
                            Input::Home => Some(Action::MoveTestJunitTomlEditorHome),
                            Input::End => Some(Action::MoveTestJunitTomlEditorEnd),
                            Input::CtrlC => Some(Action::CopyTestJunitTomlEditor),
                            Input::Char('q') | Input::Esc => Some(Action::CancelTestJunitExport),
                            Input::Enter => Some(Action::PreviewTestJunitExport),
                            _ => None,
                        }
                    };
                    if let Some(effect) =
                        action.and_then(|action| compatibility_workspace_action(&mut app, action))
                    {
                        match effect {
                            Effect::CopyToClipboard(content) => {
                                copy_to_clipboard(&mut app, content).await;
                            }
                            effect => {
                                let _ = test_coordinator.handle_effect(&mut app, effect).await;
                            }
                        }
                    }
                } else if matches!(app.active_dialog(), Some(Dialog::TestJunitExport(_))) {
                    let effect = test_junit_dialog_action(input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                    if let Some(effect) = effect {
                        let _ = test_coordinator.handle_effect(&mut app, effect).await;
                    }
                } else if matches!(
                    app.active_dialog(),
                    Some(Dialog::TestJunitExportConfirmation(_))
                ) {
                    let effect = test_junit_confirmation_action(input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                    if let Some(effect) = effect {
                        let _ = test_coordinator.handle_effect(&mut app, effect).await;
                    }
                } else if let Some(Dialog::WicCreateTomlEditor { editor, .. }) =
                    app.active_dialog().cloned()
                {
                    let action = match input {
                        Input::Enter => Some(Action::PreviewWicCreate),
                        Input::Char('q') | Input::Esc if !editor.editing => {
                            Some(Action::CancelWicCreate)
                        }
                        input => popup_editor_action(editor.editing, input),
                    };
                    if let Some(Effect::CopyToClipboard(content)) =
                        action.and_then(|action| compatibility_workspace_action(&mut app, action))
                    {
                        copy_to_clipboard(&mut app, content).await;
                    }
                } else if matches!(app.active_dialog(), Some(Dialog::WicCreate(_))) {
                    let editing = app.active_dialog().is_some_and(
                        |dialog| matches!(dialog, Dialog::WicCreate(state) if state.editing),
                    );
                    let _ = wic_create_dialog_action(editing, input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                } else if matches!(app.active_dialog(), Some(Dialog::WicCreateConfirmation(_))) {
                    let effect = wic_create_confirmation_action(input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                    if let Some(effect @ Effect::StartWicSession { .. }) = effect {
                        if submit_daemon_effect(&mut daemon_runtime, &mut app, &effect).is_some() {
                            continue;
                        }
                        let Effect::StartWicSession { id, operation } = effect else {
                            unreachable!()
                        };
                        begin_wic_job(
                            &mut app,
                            &mut wic_operation,
                            &wic_device_inspector,
                            &session_build_dir,
                            cancellation_timeout,
                            id,
                            operation,
                        )
                        .await;
                    }
                } else if matches!(app.active_dialog(), Some(Dialog::WicDevicePicker(_))) {
                    let _ = wic_device_picker_action(input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                } else if matches!(app.active_dialog(), Some(Dialog::WicWritePhrase(_))) {
                    let _ = wic_write_phrase_action(input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                } else if matches!(app.active_dialog(), Some(Dialog::WicWriteConfirmation(_))) {
                    let effect = wic_write_confirmation_action(input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                    if let Some(Effect::StartWicSession { id, operation }) = effect {
                        begin_wic_job(
                            &mut app,
                            &mut wic_operation,
                            &wic_device_inspector,
                            &session_build_dir,
                            cancellation_timeout,
                            id,
                            operation,
                        )
                        .await;
                    }
                } else if let Some(Dialog::WicCancellationConfirmation {
                    id,
                    incomplete_device_warning,
                }) = app.active_dialog().cloned()
                {
                    let effect =
                        wic_cancellation_confirmation_action(id, incomplete_device_warning, input)
                            .and_then(|action| compatibility_workspace_action(&mut app, action));
                    if let Some(effect @ Effect::CancelWicSession(_)) = effect {
                        if submit_daemon_effect(&mut daemon_runtime, &mut app, &effect).is_some() {
                            continue;
                        }
                        let Effect::CancelWicSession(id) = effect else {
                            unreachable!()
                        };
                        begin_wic_cancellation(&mut app, &mut wic_operation, id);
                    }
                } else if let Some(Dialog::ImageConsole(dialog)) = app.active_dialog().cloned() {
                    let effect = image_console_dialog_action(&dialog, input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                    if let Some(effect @ Effect::Terminal(_)) = effect {
                        let _ = submit_daemon_effect(&mut daemon_runtime, &mut app, &effect);
                    }
                } else if matches!(app.active_dialog(), Some(Dialog::QemuLaunch(_))) {
                    let editing = app.active_dialog().is_some_and(
                        |dialog| matches!(dialog, Dialog::QemuLaunch(state) if state.editing),
                    );
                    let _ = qemu_launch_dialog_action(editing, input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                } else if matches!(app.active_dialog(), Some(Dialog::QemuLaunchConfirmation(_))) {
                    let effect = qemu_launch_confirmation_action(input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                    if let Some(effect @ Effect::Terminal(_)) = &effect {
                        let _ = submit_daemon_effect(&mut daemon_runtime, &mut app, effect);
                    }
                    if let Some(effect @ Effect::StartQemuSession { .. }) = effect {
                        if submit_daemon_effect(&mut daemon_runtime, &mut app, &effect).is_some() {
                            continue;
                        }
                        let Effect::StartQemuSession { id, request } = effect else {
                            unreachable!()
                        };
                        begin_qemu_job(
                            &mut app,
                            &mut qemu_operation,
                            &session_build_dir,
                            cancellation_timeout,
                            id,
                            request,
                        )
                        .await;
                    }
                } else if matches!(
                    app.active_dialog(),
                    Some(Dialog::QemuCancellationConfirmation(_))
                ) {
                    let effect = qemu_cancellation_confirmation_action(input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                    if let Some(effect @ Effect::CancelQemuSession(_)) = effect {
                        if submit_daemon_effect(&mut daemon_runtime, &mut app, &effect).is_some() {
                            continue;
                        }
                        let Effect::CancelQemuSession(id) = effect else {
                            unreachable!()
                        };
                        begin_qemu_cancellation(&mut app, &mut qemu_operation, id);
                    }
                } else if matches!(app.active_dialog(), Some(Dialog::DtcCompile(_))) {
                    let _ = dtc_compile_dialog_action(input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                } else if matches!(app.active_dialog(), Some(Dialog::TerminalLaunch(_))) {
                    let effect = terminal_launch_dialog_action(input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                    match effect {
                        Some(effect @ Effect::Terminal(_)) => {
                            if submit_daemon_effect(&mut daemon_runtime, &mut app, &effect)
                                .is_none()
                            {
                                if let Effect::Terminal(yoctui_model::TerminalEffect::Create {
                                    kind: yoctui_model::TerminalCreationKind::GitUi,
                                    program,
                                    cwd,
                                    arguments,
                                    ..
                                }) = effect
                                {
                                    if let Err(error) = guard.suspend() {
                                        app.notification =
                                            Some(format!("Cannot open GitUI: {error}"));
                                    } else {
                                        let result = tokio::task::spawn_blocking(move || {
                                            std::process::Command::new(program)
                                                .args(arguments)
                                                .current_dir(cwd)
                                                .status()
                                        })
                                        .await;
                                        let restored = guard.resume();
                                        app.notification = Some(match (result, restored) {
                                            (_, Err(error)) => {
                                                format!("Cannot restore terminal: {error}")
                                            }
                                            (Ok(Ok(status)), Ok(())) if status.success() => {
                                                "GitUI closed; source status will refresh.".into()
                                            }
                                            (result, _) => format!("GitUI finished: {result:?}"),
                                        });
                                    }
                                } else {
                                    app.notification = Some("Embedded terminal unavailable: connect to the daemon or choose a detached terminal.".into());
                                }
                            }
                        }
                        Some(Effect::LaunchDetachedTerminal(request)) => {
                            match launch_detached_terminal(&request) {
                                Ok(()) => {
                                    app.notification = Some(format!(
                                        "Detached terminal started for {}.",
                                        request.name
                                    ));
                                }
                                Err(error) => {
                                    app.notification =
                                        Some(format!("Could not start detached terminal: {error}"));
                                    let _ = update(
                                        &mut app,
                                        Action::DetachedTerminalAvailabilityDetected(
                                            detached_terminal_availability(),
                                        ),
                                    );
                                }
                            }
                        }
                        _ => {}
                    }
                } else if matches!(app.active_dialog(), Some(Dialog::RecipeEditor(_))) {
                    let editor = app.active_dialog().and_then(|dialog| match dialog {
                        Dialog::RecipeEditor(editor) => Some(editor.clone()),
                        _ => None,
                    });
                    let effect = editor
                        .as_ref()
                        .and_then(|editor| recipe_editor_action(editor, input))
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                    match effect {
                        Some(Effect::LoadRecipeEditorFile(path)) => {
                            load_recipe_editor_file(&mut app, path).await;
                        }
                        Some(Effect::SaveRecipeEditorFile {
                            root,
                            path,
                            content,
                            expected,
                        }) => {
                            save_recipe_editor_file(&mut app, root, path, content, expected).await;
                        }
                        Some(Effect::OpenInEditor(path)) => {
                            open_in_editor(&guard, &mut app, path.clone(), Some("vim")).await;
                            if let Ok(content) = fs::read_to_string(path) {
                                let _ = compatibility_workspace_action(
                                    &mut app,
                                    Action::LoadRecipeEditorExternalContent(content),
                                );
                            }
                        }
                        Some(Effect::Start(request)) => {
                            begin_runtime_build(
                                &mut daemon_runtime,
                                &mut backend,
                                &mut app,
                                &mut build_jobs,
                                request,
                            )
                            .await;
                        }
                        Some(Effect::CopyToClipboard(content)) => {
                            copy_to_clipboard(&mut app, content).await;
                        }
                        _ => {}
                    }
                } else if matches!(
                    app.active_dialog(),
                    Some(Dialog::DevtoolModifyConfirmation(_))
                ) {
                    let effect = devtool_modify_confirmation_action(input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                    if let Some(Effect::DevtoolModify(identity)) = effect {
                        if submit_daemon_effect(
                            &mut daemon_runtime,
                            &mut app,
                            &Effect::DevtoolModify(identity.clone()),
                        )
                        .is_some()
                        {
                            pending_daemon_devtool_modify = Some(identity);
                            continue;
                        }
                        let recipe = identity.name.clone();
                        if begin_devtool_job(
                            &mut app,
                            &mut devtool_jobs,
                            &mut devtool_runner,
                            &session_build_dir,
                            cancellation_timeout,
                            None,
                            DevtoolOperation::Modify { recipe },
                        )
                        .await
                        {
                            pending_devtool_modify = Some(identity);
                        }
                    }
                } else if app.screen == Screen::Signatures
                    && app.focus == yoctui_model::FocusTarget::Workspace
                    && signature_workspace_action(input).is_some()
                    && app.active_dialog().is_none()
                    && app.notification.is_none()
                {
                    let effect = signature_workspace_action(input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                    match effect {
                        Some(
                            effect @ (Effect::GetSignatureDump(_) | Effect::CompareSignatures(_)),
                        ) => begin_signature_operation(
                            &mut app,
                            &signature_adapter,
                            &mut signature_operation,
                            effect,
                        ),
                        Some(Effect::CancelSignatureOperation) => {
                            if let Some(operation) = signature_operation.as_ref() {
                                if operation.cancellation.cancel() {
                                    app.notification =
                                        Some("Signature cancellation requested.".into());
                                }
                            } else {
                                app.notification =
                                    Some("No signature operation is running.".into());
                            }
                        }
                        Some(Effect::OpenInEditor(path)) => {
                            open_in_editor(&guard, &mut app, path, editor.as_deref()).await;
                        }
                        _ => {
                            if matches!(input, Input::Char('q') | Input::CtrlC) {
                                let _ = compatibility_workspace_action(&mut app, Action::Quit);
                            } else if input == Input::Char('?') {
                                let _ = compatibility_workspace_action(
                                    &mut app,
                                    Action::Open(Screen::Help),
                                );
                            }
                        }
                    }
                } else if let Some(action) = pane_focus_route(&app, input) {
                    let effect = compatibility_workspace_action(&mut app, action);
                    if let Some(
                        effect @ (Effect::GetPackageInventory(_) | Effect::GetPackageDetail(_)),
                    ) = effect
                    {
                        begin_package_operation(
                            &mut app,
                            &package_adapter,
                            &mut package_operation,
                            effect,
                        );
                    } else if let Some(effect @ Effect::GetImageArtifacts(_)) = effect {
                        begin_image_artifact_operation(
                            &mut app,
                            image_artifact_adapter.as_ref(),
                            &mut image_artifact_operation,
                            effect,
                        );
                    } else if let Some(effect @ Effect::GetRootfsComposition(_)) = effect {
                        begin_rootfs_composition_operation(
                            backend.as_mut(),
                            &mut app,
                            &session_build_dir,
                            &mut rootfs_composition_operation,
                            effect,
                            daemon_attached,
                        )
                        .await;
                    } else if let Some(effect @ Effect::InspectSdkTools) = effect {
                        begin_sdk_capability_operation(
                            &mut app,
                            sdk_tool_adapter.as_ref(),
                            &mut sdk_capability_operation,
                            effect,
                        );
                    } else if let Some(Effect::InspectKernel) = effect {
                        inspect_kernel_workbench(&mut app, backend.as_mut()).await;
                    } else if let Some(Effect::InspectFirmware) = effect {
                        inspect_firmware_workbench(&mut app, backend.as_mut()).await;
                    } else if let Some(effect @ Effect::Security(_)) = effect {
                        let _ = route_independent_security_effect(
                            &guard,
                            &mut app,
                            &mut security_coordinator,
                            effect,
                            editor.as_deref(),
                        )
                        .await;
                    } else if let Some(effect @ Effect::Maintenance(_)) = effect {
                        let _ = route_independent_maintenance_effect(
                            &guard,
                            &mut app,
                            &mut maintenance_coordinator,
                            effect,
                            editor.as_deref(),
                        )
                        .await;
                    }
                } else if matches!(
                    app.active_dialog(),
                    Some(Dialog::BuildCancellationConfirmation)
                ) {
                    let _ = build_cancellation_confirmation_action(input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                } else if matches!(app.active_dialog(), Some(Dialog::QuitConfirmation)) {
                    let _ = quit_confirmation_action(input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                } else if app.layer_browser.is_some()
                    && !app.metadata_searching
                    && app.focus != yoctui_model::FocusTarget::Dialog
                {
                    let preview_focused = app
                        .layer_browser
                        .as_ref()
                        .is_some_and(|browser| browser.preview_focused);
                    let effect = match (preview_focused, input) {
                        (true, Input::Up) => compatibility_workspace_action(
                            &mut app,
                            Action::ScrollLayerBrowserPreview { delta: -1 },
                        ),
                        (true, Input::Down) => compatibility_workspace_action(
                            &mut app,
                            Action::ScrollLayerBrowserPreview { delta: 1 },
                        ),
                        (true, Input::PageUp) => compatibility_workspace_action(
                            &mut app,
                            Action::ScrollLayerBrowserPreview { delta: -10 },
                        ),
                        (true, Input::PageDown) => compatibility_workspace_action(
                            &mut app,
                            Action::ScrollLayerBrowserPreview { delta: 10 },
                        ),
                        (true, Input::Left) => {
                            compatibility_workspace_action(&mut app, Action::FocusLayerBrowserTree)
                        }
                        (_, input) => match input {
                            Input::Tab => compatibility_workspace_action(
                                &mut app,
                                Action::CycleFocus { backwards: false },
                            ),
                            Input::BackTab => compatibility_workspace_action(
                                &mut app,
                                Action::CycleFocus { backwards: true },
                            ),
                            Input::Up => compatibility_workspace_action(
                                &mut app,
                                Action::SelectLayerBrowserEntry { delta: -1 },
                            ),
                            Input::Down => compatibility_workspace_action(
                                &mut app,
                                Action::SelectLayerBrowserEntry { delta: 1 },
                            ),
                            Input::PageUp => compatibility_workspace_action(
                                &mut app,
                                Action::SelectLayerBrowserEntry { delta: -10 },
                            ),
                            Input::PageDown => compatibility_workspace_action(
                                &mut app,
                                Action::SelectLayerBrowserEntry { delta: 10 },
                            ),
                            Input::Enter => {
                                compatibility_workspace_action(&mut app, Action::LayerBrowserEnter)
                            }
                            Input::Right | Input::Char('l') => {
                                compatibility_workspace_action(&mut app, Action::LayerBrowserExpand)
                            }
                            Input::Esc => {
                                compatibility_workspace_action(&mut app, Action::CloseLayerBrowser)
                            }
                            Input::Left | Input::Char('h') => {
                                compatibility_workspace_action(&mut app, Action::LayerBrowserUp)
                            }
                            Input::Char('r') => compatibility_workspace_action(
                                &mut app,
                                Action::RefreshLayerBrowser,
                            ),
                            Input::Char('e') => compatibility_workspace_action(
                                &mut app,
                                Action::EditSelectedLayerBrowserFile,
                            ),
                            Input::Char('.') => compatibility_workspace_action(
                                &mut app,
                                Action::ToggleLayerBrowserHidden,
                            ),
                            Input::Char('/') => compatibility_workspace_action(
                                &mut app,
                                Action::BeginMetadataSearch,
                            ),
                            Input::Char('i') => compatibility_workspace_action(
                                &mut app,
                                Action::SetLayerInspectorMode(LayerInspectorMode::Metadata),
                            ),
                            Input::Char('[') => compatibility_workspace_action(
                                &mut app,
                                Action::ScrollLayerBrowserPreview { delta: -10 },
                            ),
                            Input::Char(']') => compatibility_workspace_action(
                                &mut app,
                                Action::ScrollLayerBrowserPreview { delta: 10 },
                            ),
                            Input::Char('m') => compatibility_workspace_action(
                                &mut app,
                                Action::SetLayerInspectorMode(LayerInspectorMode::Metadata),
                            ),
                            Input::Char('d') => compatibility_workspace_action(
                                &mut app,
                                Action::SetLayerInspectorMode(LayerInspectorMode::Dependencies),
                            ),
                            _ => None,
                        },
                    };
                    match effect {
                        Some(Effect::LoadLayerBrowserDirectory {
                            layer,
                            root,
                            directory,
                        }) => load_layer_browser_directory(&mut app, layer, root, directory).await,
                        Some(Effect::LoadLayerBrowserPreview(path)) => {
                            load_layer_browser_preview(&mut app, path).await
                        }
                        Some(Effect::OpenLayerBrowserEditor { layer, root, file }) => {
                            if let Some(Effect::LoadRecipeEditorFile(path)) =
                                compatibility_workspace_action(
                                    &mut app,
                                    Action::OpenRecipeEditor {
                                        recipe: format!("Layer: {layer}"),
                                        root,
                                        files: vec![file],
                                    },
                                )
                            {
                                load_recipe_editor_file(&mut app, path).await;
                            }
                        }
                        _ => {}
                    }
                } else if matches!(
                    app.active_dialog(),
                    Some(Dialog::DevtoolResetConfirmation(_))
                ) {
                    let effect = devtool_reset_confirmation_action(input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                    if let Some(Effect::DevtoolReset(plan)) = effect {
                        if submit_daemon_effect(
                            &mut daemon_runtime,
                            &mut app,
                            &Effect::DevtoolReset(plan.clone()),
                        )
                        .is_some()
                        {
                            continue;
                        }
                        let operation = plan.operation();
                        if begin_devtool_job(
                            &mut app,
                            &mut devtool_jobs,
                            &mut devtool_runner,
                            &session_build_dir,
                            cancellation_timeout,
                            None,
                            operation,
                        )
                        .await
                        {
                            pending_devtool_reset = Some(plan.identity);
                        }
                    }
                } else if matches!(
                    app.active_dialog(),
                    Some(Dialog::DevtoolUpdateConfirmation(_))
                ) {
                    let effect = devtool_update_confirmation_action(input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                    if let Some(Effect::DevtoolUpdateRecipe(identity)) = effect {
                        if submit_daemon_effect(
                            &mut daemon_runtime,
                            &mut app,
                            &Effect::DevtoolUpdateRecipe(identity.clone()),
                        )
                        .is_some()
                        {
                            continue;
                        }
                        let recipe = identity.name.clone();
                        if begin_devtool_job(
                            &mut app,
                            &mut devtool_jobs,
                            &mut devtool_runner,
                            &session_build_dir,
                            cancellation_timeout,
                            None,
                            DevtoolOperation::UpdateRecipe { recipe },
                        )
                        .await
                        {
                            pending_devtool_update = Some(identity);
                        }
                    }
                } else if matches!(
                    app.active_dialog(),
                    Some(Dialog::DevtoolFinishConfirmation(_))
                ) {
                    let effect = devtool_finish_confirmation_action(input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                    if let Some(Effect::DevtoolFinish(plan)) = effect {
                        if submit_daemon_effect(
                            &mut daemon_runtime,
                            &mut app,
                            &Effect::DevtoolFinish(plan.clone()),
                        )
                        .is_some()
                        {
                            continue;
                        }
                        let request = plan.request();
                        if begin_devtool_job(
                            &mut app,
                            &mut devtool_jobs,
                            &mut devtool_runner,
                            &session_build_dir,
                            cancellation_timeout,
                            None,
                            request.into(),
                        )
                        .await
                        {
                            pending_devtool_finish = Some(plan.identity);
                        }
                    }
                } else if matches!(app.active_dialog(), Some(Dialog::DevtoolFinishPicker(_))) {
                    let _ = devtool_finish_picker_action(input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                } else if matches!(
                    app.active_dialog(),
                    Some(Dialog::DevtoolDeployConfirmation(_))
                ) {
                    let effect = devtool_deploy_confirmation_action(input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                    if let Some(Effect::DevtoolDeploy(plan)) = effect {
                        if submit_daemon_effect(
                            &mut daemon_runtime,
                            &mut app,
                            &Effect::DevtoolDeploy(plan.clone()),
                        )
                        .is_some()
                        {
                            continue;
                        }
                        let request = plan.request();
                        if begin_devtool_job(
                            &mut app,
                            &mut devtool_jobs,
                            &mut devtool_runner,
                            &session_build_dir,
                            cancellation_timeout,
                            None,
                            request.into(),
                        )
                        .await
                        {
                            pending_devtool_deploy = Some(plan.identity);
                        }
                    }
                } else if matches!(app.active_dialog(), Some(Dialog::DevtoolDeploy(_))) {
                    let _ = devtool_deploy_dialog_action(input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                } else if matches!(app.active_dialog(), Some(Dialog::BbmaskConfirmation(_))) {
                    let effect = match input {
                        Input::Enter => {
                            compatibility_workspace_action(&mut app, Action::ConfirmBbmaskWrite)
                        }
                        Input::Esc => {
                            compatibility_workspace_action(&mut app, Action::CancelBbmaskWrite)
                        }
                        _ => None,
                    };
                    if let Some(Effect::WriteBbmask(value)) = effect {
                        match write_bbmask(&session_build_dir, value).await {
                            Ok(()) => {
                                refresh_workspace(
                                    &mut backend,
                                    &mut app,
                                    "BBMASK saved and workspace metadata refreshed.",
                                )
                                .await
                            }
                            Err(error) => {
                                app.notification = Some(format!("Could not save BBMASK: {error}"))
                            }
                        }
                    }
                } else if let Some(Dialog::BbmaskEdit(editor)) = app.active_dialog().cloned() {
                    let action = match input {
                        Input::Enter => Some(Action::PreviewBbmaskEdit),
                        Input::Char('q') | Input::Esc if !editor.editing => {
                            Some(Action::CancelBbmaskEdit)
                        }
                        input => popup_editor_action(editor.editing, input),
                    };
                    if let Some(Effect::CopyToClipboard(content)) =
                        action.and_then(|action| compatibility_workspace_action(&mut app, action))
                    {
                        copy_to_clipboard(&mut app, content).await;
                    }
                } else if matches!(app.active_dialog(), Some(Dialog::BuildCompletion)) {
                    let action = if input == Input::Enter
                        && app.build.status == BuildStatus::Failed
                        && app.build.errors > 0
                    {
                        Action::OpenBuildCompletionErrors
                    } else {
                        Action::DismissBuildCompletion
                    };
                    let _ = compatibility_workspace_action(&mut app, action);
                } else if matches!(app.active_dialog(), Some(Dialog::ImagePicker(_))) {
                    let _ = match input {
                        Input::Up => compatibility_workspace_action(
                            &mut app,
                            Action::SelectImage { delta: -1 },
                        ),
                        Input::Down => compatibility_workspace_action(
                            &mut app,
                            Action::SelectImage { delta: 1 },
                        ),
                        Input::Enter => {
                            compatibility_workspace_action(&mut app, Action::ConfirmImagePicker)
                        }
                        Input::Esc => {
                            compatibility_workspace_action(&mut app, Action::CancelImagePicker)
                        }
                        _ => None,
                    };
                } else if matches!(app.active_dialog(), Some(Dialog::SignatureTaskPicker(_))) {
                    let effect = signature_task_picker_action(input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                    if let Some(effect @ Effect::GetSignatureDump(_)) = effect {
                        begin_signature_operation(
                            &mut app,
                            &signature_adapter,
                            &mut signature_operation,
                            effect,
                        );
                    }
                } else if matches!(app.active_dialog(), Some(Dialog::RecipeTaskPicker(_))) {
                    let _ = match input {
                        Input::Up => compatibility_workspace_action(
                            &mut app,
                            Action::SelectRecipeTask { delta: -1 },
                        ),
                        Input::Down => compatibility_workspace_action(
                            &mut app,
                            Action::SelectRecipeTask { delta: 1 },
                        ),
                        Input::Enter => compatibility_workspace_action(
                            &mut app,
                            Action::PreviewSelectedRecipeTask,
                        ),
                        Input::Esc => {
                            compatibility_workspace_action(&mut app, Action::CancelRecipeTaskPicker)
                        }
                        _ => None,
                    };
                } else if matches!(app.active_dialog(), Some(Dialog::RecipeTaskLogPicker(_))) {
                    let effect = match input {
                        Input::Up => compatibility_workspace_action(
                            &mut app,
                            Action::SelectRecipeTaskLog { delta: -1 },
                        ),
                        Input::Down => compatibility_workspace_action(
                            &mut app,
                            Action::SelectRecipeTaskLog { delta: 1 },
                        ),
                        Input::Enter => compatibility_workspace_action(
                            &mut app,
                            Action::OpenSelectedRecipeTaskLog,
                        ),
                        Input::Esc => compatibility_workspace_action(
                            &mut app,
                            Action::CancelRecipeTaskLogPicker,
                        ),
                        _ => None,
                    };
                    if let Some(Effect::OpenInEditor(path)) = effect {
                        open_in_editor(&guard, &mut app, path, editor.as_deref()).await;
                    }
                } else if matches!(app.active_dialog(), Some(Dialog::RecipePatchPicker(_))) {
                    let effect = match input {
                        Input::Up => compatibility_workspace_action(
                            &mut app,
                            Action::SelectRecipePatch { delta: -1 },
                        ),
                        Input::Down => compatibility_workspace_action(
                            &mut app,
                            Action::SelectRecipePatch { delta: 1 },
                        ),
                        Input::Enter => compatibility_workspace_action(
                            &mut app,
                            Action::OpenSelectedRecipePatch,
                        ),
                        Input::Esc => compatibility_workspace_action(
                            &mut app,
                            Action::CancelRecipePatchPicker,
                        ),
                        _ => None,
                    };
                    if let Some(Effect::OpenInEditor(path)) = effect {
                        open_in_editor(&guard, &mut app, path, editor.as_deref()).await;
                    }
                } else if matches!(app.active_dialog(), Some(Dialog::ConfigSourcePicker(_))) {
                    let effect = config_source_picker_action(input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                    if let Some(Effect::OpenInEditor(path)) = effect {
                        open_in_editor(&guard, &mut app, path, editor.as_deref()).await;
                    }
                } else if matches!(app.active_dialog(), Some(Dialog::ConfigScopePicker(_))) {
                    let effect = config_scope_picker_action(input)
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                    if let Some(Effect::GetVariable(identity)) = effect {
                        load_config_variable(&mut app, backend.as_mut(), identity).await;
                    }
                } else if matches!(app.active_dialog(), Some(Dialog::ConfigComparison(_))) {
                    if let Some(action) = config_compare_dialog_action(input) {
                        let _ = compatibility_workspace_action(&mut app, action);
                    }
                } else if let Some(Dialog::ConfigEdit { editor, .. }) = app.active_dialog().cloned()
                {
                    let action = match input {
                        Input::Enter => Some(Action::PreviewConfigEdit),
                        Input::Char('q') | Input::Esc if !editor.editing => {
                            Some(Action::CancelConfigEdit)
                        }
                        input => popup_editor_action(editor.editing, input),
                    };
                    if let Some(Effect::CopyToClipboard(content)) =
                        action.and_then(|action| compatibility_workspace_action(&mut app, action))
                    {
                        copy_to_clipboard(&mut app, content).await;
                    }
                } else if matches!(app.active_dialog(), Some(Dialog::ConfigEditConfirmation(_))) {
                    if let Some(action) = config_edit_confirmation_action(input)
                        && let Some(Effect::WriteConfigAssignment(request)) =
                            compatibility_workspace_action(&mut app, action)
                    {
                        execute_config_edit_write(
                            backend.as_mut(),
                            &mut app,
                            &session_build_dir,
                            request,
                        )
                        .await;
                    }
                } else if matches!(app.active_dialog(), Some(Dialog::RecipeTaskConfirmation(_))) {
                    let effect = match input {
                        Input::Enter => {
                            compatibility_workspace_action(&mut app, Action::ConfirmRecipeTask)
                        }
                        Input::Esc => {
                            compatibility_workspace_action(&mut app, Action::CancelRecipeTask)
                        }
                        _ => None,
                    };
                    if let Some(Effect::Start(request)) = effect {
                        begin_runtime_build(
                            &mut daemon_runtime,
                            &mut backend,
                            &mut app,
                            &mut build_jobs,
                            request,
                        )
                        .await;
                    }
                } else if matches!(app.active_dialog(), Some(Dialog::BuildOptions)) {
                    let effect = match input {
                        Input::Char('b') => compatibility_workspace_action(
                            &mut app,
                            Action::BeginBuildTargetTask(None),
                        ),
                        Input::Char('c') => compatibility_workspace_action(
                            &mut app,
                            Action::BeginBuildTargetTask(Some("clean".into())),
                        ),
                        Input::Char('m') => compatibility_workspace_action(
                            &mut app,
                            Action::BeginBuildTargetTask(Some("menuconfig".into())),
                        ),
                        Input::Char('e') => {
                            compatibility_workspace_action(&mut app, Action::BeginBuildTargetEdit)
                        }
                        Input::Esc => {
                            compatibility_workspace_action(&mut app, Action::CloseBuildOptions)
                        }
                        _ => None,
                    };
                    if let Some(Effect::Start(request)) = effect {
                        begin_runtime_build(
                            &mut daemon_runtime,
                            &mut backend,
                            &mut app,
                            &mut build_jobs,
                            request,
                        )
                        .await;
                    }
                } else if let Some(Dialog::BuildTarget { editor, .. }) =
                    app.active_dialog().cloned()
                {
                    let action = match input {
                        Input::Enter => Some(Action::ConfirmBuildTarget),
                        Input::Char('q') | Input::Esc if !editor.editing => {
                            Some(Action::CancelBuildTargetEdit)
                        }
                        input => popup_editor_action(editor.editing, input),
                    };
                    let effect =
                        action.and_then(|action| compatibility_workspace_action(&mut app, action));
                    match effect {
                        Some(Effect::Start(request)) => {
                            begin_runtime_build(
                                &mut daemon_runtime,
                                &mut backend,
                                &mut app,
                                &mut build_jobs,
                                request,
                            )
                            .await;
                        }
                        Some(Effect::CopyToClipboard(content)) => {
                            copy_to_clipboard(&mut app, content).await;
                        }
                        _ => {}
                    }
                } else if app.screen == Screen::TerminalSessions {
                    let terminal_action = if replayed_context_action {
                        yoctui_app::terminal_context_action(input)
                    } else {
                        yoctui_app::terminal_workspace_action(&app, input)
                    };
                    if let Some(action) = terminal_action {
                        match compatibility_workspace_action(&mut app, action) {
                            Some(effect @ Effect::Terminal(_)) => {
                                let _ =
                                    submit_daemon_effect(&mut daemon_runtime, &mut app, &effect);
                            }
                            Some(Effect::CopyToClipboard(content)) => {
                                copy_to_clipboard(&mut app, content).await;
                            }
                            _ => {}
                        }
                    } else if app.selected_terminal_is_writer() {
                        if let (Some(bytes), Some(session), Some(details)) = (
                            terminal_input_bytes(input),
                            app.selected_terminal_session(),
                            app.selected_terminal_details(),
                        ) {
                            let effect = Effect::Terminal(yoctui_model::TerminalEffect::Input {
                                session_id: session.id,
                                writer_epoch: details.writer_epoch,
                                bytes,
                            });
                            let _ = submit_daemon_effect(&mut daemon_runtime, &mut app, &effect);
                        }
                    } else {
                        app.notification = Some(
                            "Terminal is read-only; press o or Ctrl+B o to take writer control."
                                .into(),
                        );
                    }
                } else if let Some(action) = notification_input_action(
                    app.notification.is_some(),
                    app.build.status == BuildStatus::Failed
                        && app.logs.diagnostics().next().is_some(),
                    app.screen == Screen::Settings && app.settings_dirty,
                    input,
                ) {
                    let _ = compatibility_workspace_action(&mut app, action);
                } else if matches!(app.screen, Screen::Kernel | Screen::Firmware)
                    && let Some(action) = match app.screen {
                        Screen::Kernel => platform_workspace_action(input),
                        Screen::Firmware => firmware_workspace_action(input),
                        _ => None,
                    }
                {
                    match compatibility_workspace_action(&mut app, action) {
                        Some(Effect::InspectKernel) => {
                            inspect_kernel_workbench(&mut app, backend.as_mut()).await;
                        }
                        Some(Effect::InspectFirmware) => {
                            inspect_firmware_workbench(&mut app, backend.as_mut()).await;
                        }
                        Some(Effect::OpenWorkspaceEditor { label, root }) => {
                            open_workspace_editor(&mut app, label, root).await;
                        }
                        Some(Effect::OpenLayerBrowserEditor { layer, root, file }) => {
                            if let Some(Effect::LoadRecipeEditorFile(path)) =
                                compatibility_workspace_action(
                                    &mut app,
                                    Action::OpenRecipeEditor {
                                        recipe: layer,
                                        root,
                                        files: vec![file],
                                    },
                                )
                            {
                                load_recipe_editor_file(&mut app, path).await;
                            }
                        }
                        _ => {}
                    }
                } else if app.screen == Screen::Insights
                    && let Some(action) = overview_workspace_action(input)
                {
                    let _ = compatibility_workspace_action(&mut app, action);
                } else if collection_scroll_delta(input).is_some()
                    && let Some(action) = workspace_collection_action(&app, input)
                {
                    let _ = compatibility_workspace_action(&mut app, action);
                } else if app.screen == Screen::Packages
                    && package_workspace_action(app.package_searching, input).is_some()
                {
                    let action = package_workspace_action(app.package_searching, input)
                        .expect("Packages action was checked");
                    match compatibility_workspace_action(&mut app, action) {
                        Some(
                            effect @ (Effect::GetPackageInventory(_) | Effect::GetPackageDetail(_)),
                        ) => begin_package_operation(
                            &mut app,
                            &package_adapter,
                            &mut package_operation,
                            effect,
                        ),
                        Some(Effect::CancelPackageOperation) => {
                            if let Some(operation) = package_operation.as_ref() {
                                if operation.cancellation.cancel() {
                                    app.notification =
                                        Some("Package-data cancellation requested.".into());
                                }
                            } else {
                                app.notification =
                                    Some("No package-data operation is running.".into());
                            }
                        }
                        Some(Effect::OpenInEditor(path)) => {
                            open_in_editor(&guard, &mut app, path, editor.as_deref()).await;
                        }
                        _ => {}
                    }
                } else if app.screen == Screen::Images
                    && images_workspace_action_for_view(
                        app.image_artifact_searching,
                        app.images_view,
                        input,
                    )
                    .is_some()
                {
                    let action = images_workspace_action_for_view(
                        app.image_artifact_searching,
                        app.images_view,
                        input,
                    )
                    .expect("Images action was checked");
                    match compatibility_workspace_action(&mut app, action) {
                        Some(effect @ Effect::GetImageArtifacts(_)) => {
                            begin_image_artifact_operation(
                                &mut app,
                                image_artifact_adapter.as_ref(),
                                &mut image_artifact_operation,
                                effect,
                            )
                        }
                        Some(effect @ Effect::GetRootfsComposition(_)) => {
                            begin_rootfs_composition_operation(
                                backend.as_mut(),
                                &mut app,
                                &session_build_dir,
                                &mut rootfs_composition_operation,
                                effect,
                                daemon_attached,
                            )
                            .await
                        }
                        Some(effect @ Effect::GetWicDevices(_)) => begin_wic_device_operation(
                            &wic_device_inspector,
                            &mut wic_device_operation,
                            effect,
                        ),
                        Some(Effect::CancelImageArtifactOperation) => {
                            if let Some(operation) = image_artifact_operation.as_ref() {
                                if operation.cancellation.cancel() {
                                    app.notification =
                                        Some("Image artifact cancellation requested.".into());
                                }
                            } else {
                                app.notification =
                                    Some("No image artifact operation is running.".into());
                            }
                        }
                        Some(Effect::OpenInEditor(path)) => {
                            open_in_editor(&guard, &mut app, path, editor.as_deref()).await;
                        }
                        Some(Effect::OpenLayerBrowserEditor { layer, root, file }) => {
                            if let Some(Effect::LoadRecipeEditorFile(path)) =
                                compatibility_workspace_action(
                                    &mut app,
                                    Action::OpenRecipeEditor {
                                        recipe: layer,
                                        root,
                                        files: vec![file],
                                    },
                                )
                            {
                                load_recipe_editor_file(&mut app, path).await;
                            }
                        }
                        Some(Effect::LoadLayerBrowserDirectory {
                            layer,
                            root,
                            directory,
                        }) => {
                            load_layer_browser_directory(&mut app, layer, root, directory).await;
                        }
                        _ => {}
                    }
                } else if app.screen == Screen::Sdk
                    && sdk_workspace_action(app.sdk_artifact_searching, input).is_some()
                {
                    let action = sdk_workspace_action(app.sdk_artifact_searching, input)
                        .expect("SDK action was checked");
                    match compatibility_workspace_action(&mut app, action) {
                        Some(effect @ Effect::GetSdkArtifacts(_)) => begin_sdk_artifact_operation(
                            &mut app,
                            sdk_artifact_adapter.as_ref(),
                            &mut sdk_artifact_operation,
                            effect,
                        ),
                        Some(Effect::CancelSdkArtifactOperation) => {
                            if let Some(operation) = sdk_artifact_operation.as_ref() {
                                if operation.cancellation.cancel() {
                                    app.notification =
                                        Some("SDK artifact cancellation requested.".into());
                                }
                            } else {
                                app.notification = Some("No SDK artifact scan is running.".into());
                            }
                        }
                        Some(effect @ Effect::InspectSdkTools) => begin_sdk_capability_operation(
                            &mut app,
                            sdk_tool_adapter.as_ref(),
                            &mut sdk_capability_operation,
                            effect,
                        ),
                        Some(Effect::OpenInEditor(path)) => {
                            open_in_editor(&guard, &mut app, path, editor.as_deref()).await;
                        }
                        _ => {}
                    }
                } else if app.screen == Screen::Testing
                    && testing_screen_action(&app, input).is_some()
                {
                    let action =
                        testing_screen_action(&app, input).expect("Testing action was checked");
                    match compatibility_workspace_action(&mut app, action) {
                        Some(effect @ Effect::ImportTestResults(_))
                        | Some(effect @ Effect::CompareTestResults(_)) => {
                            if submit_daemon_effect(&mut daemon_runtime, &mut app, &effect)
                                .is_none()
                            {
                                let _ = test_coordinator.handle_effect(&mut app, effect).await;
                            }
                        }
                        Some(effect @ Effect::InspectTestJunitDestination { .. })
                        | Some(effect @ Effect::ExportTestJunit(_))
                        | Some(effect @ Effect::InspectTestCapability)
                        | Some(effect @ Effect::InspectResultToolCapability) => {
                            let _ = test_coordinator.handle_effect(&mut app, effect).await;
                        }
                        Some(Effect::OpenInEditor(path)) => {
                            open_in_editor(&guard, &mut app, path, editor.as_deref()).await;
                        }
                        _ => {}
                    }
                } else if app.screen == Screen::Security
                    && security_workspace_action(
                        app.security.view,
                        app.security.drilled,
                        app.security.searching,
                        input,
                    )
                    .is_some()
                {
                    let action = security_workspace_action(
                        app.security.view,
                        app.security.drilled,
                        app.security.searching,
                        input,
                    )
                    .expect("Security action was checked");
                    if let Some(effect) = compatibility_workspace_action(&mut app, action) {
                        let _ = route_independent_security_effect(
                            &guard,
                            &mut app,
                            &mut security_coordinator,
                            effect,
                            editor.as_deref(),
                        )
                        .await;
                    }
                } else if app.screen == Screen::Qa
                    && qa_workspace_action(app.qa.view, app.qa.drilled, app.qa.searching, input)
                        .is_some()
                {
                    let action =
                        qa_workspace_action(app.qa.view, app.qa.drilled, app.qa.searching, input)
                            .expect("QA action was checked");
                    if let Some(effect) = compatibility_workspace_action(&mut app, action) {
                        let _ = route_independent_qa_effect(
                            &guard,
                            &mut app,
                            &mut qa_coordinator,
                            effect,
                            editor.as_deref(),
                        )
                        .await;
                    }
                } else if app.screen == Screen::Maintenance
                    && maintenance_workspace_action(
                        app.maintenance.view,
                        maintenance_row_count(&app),
                        input,
                    )
                    .is_some()
                {
                    let action = maintenance_workspace_action(
                        app.maintenance.view,
                        maintenance_row_count(&app),
                        input,
                    )
                    .expect("Maintenance action was checked");
                    if let Some(effect) = compatibility_workspace_action(&mut app, action) {
                        let _ = route_independent_maintenance_effect(
                            &guard,
                            &mut app,
                            &mut maintenance_coordinator,
                            effect,
                            editor.as_deref(),
                        )
                        .await;
                    }
                } else if app.screen == Screen::BuildEnvironment
                    && app
                        .build_environment_draft
                        .as_ref()
                        .is_some_and(|draft| draft.editing)
                {
                    let action = match input {
                        Input::Up => Some(Action::SelectBuildEnvironmentField { delta: -1 }),
                        Input::Down => Some(Action::SelectBuildEnvironmentField { delta: 1 }),
                        Input::Enter => Some(Action::ApplyBuildEnvironmentProfile),
                        Input::Esc => Some(Action::CancelBuildEnvironmentEdit),
                        Input::Backspace => Some(Action::BackspaceBuildEnvironmentField),
                        Input::Char(c) => Some(Action::AppendBuildEnvironmentField(c)),
                        _ => None,
                    };
                    if let Some(action) = action {
                        let _ = compatibility_workspace_action(&mut app, action);
                    }
                } else if app.screen == Screen::BuildEnvironment
                    && build_environment_action(input).is_some()
                {
                    let action = build_environment_action(input)
                        .expect("build environment action was checked");
                    if matches!(action, Action::EnvironmentSetup(_)) {
                        if let Some(effect) = compatibility_workspace_action(&mut app, action) {
                            environment_browser_io.submit(effect);
                        }
                        continue;
                    }
                    if let Some(Effect::VerifyBuildEnvironment {
                        profile,
                        generation,
                    }) = compatibility_workspace_action(&mut app, action)
                    {
                        environment_operation::start(
                            &mut environment_operation,
                            profile,
                            generation,
                            backend_kind.clone(),
                            cancellation_timeout,
                        );
                    }
                } else if app.screen == Screen::Settings && settings_action(input).is_some() {
                    if app.settings_selection == 0 && matches!(input, Input::Enter | Input::Right) {
                        let _ = compatibility_workspace_action(&mut app, Action::OpenThemePicker);
                        continue;
                    }
                    let action = settings_action(input).expect("settings action was checked");
                    match compatibility_workspace_action(&mut app, action) {
                        Some(Effect::PersistSettings) => {
                            let result = persist_settings(
                                session_path.as_deref(),
                                &mut session,
                                &app,
                                !color_forced_off,
                            );
                            let persistence_action = match result {
                                Ok(()) => Action::SettingsPersisted,
                                Err(error) => Action::SettingsPersistenceFailed(error.to_string()),
                            };
                            let _ = compatibility_workspace_action(&mut app, persistence_action);
                        }
                        Some(Effect::VerifyBuildEnvironment {
                            profile,
                            generation,
                        }) => environment_operation::start(
                            &mut environment_operation,
                            profile,
                            generation,
                            backend_kind.clone(),
                            cancellation_timeout,
                        ),
                        _ => {}
                    }
                } else if app.screen == Screen::Tasks
                    && tasks_action(app.task_filter_editing, input).is_some()
                {
                    let action = tasks_action(app.task_filter_editing, input)
                        .expect("Tasks action was checked");
                    let _ = compatibility_workspace_action(&mut app, action);
                } else if app.screen == Screen::Logs && log_workspace_action(&app, input).is_some()
                {
                    let action = log_workspace_action(&app, input)
                        .expect("Logs workspace action was checked");
                    match compatibility_workspace_action(&mut app, action) {
                        Some(Effect::OpenInEditor(path)) => {
                            open_in_editor(&guard, &mut app, path, editor.as_deref()).await;
                        }
                        Some(Effect::CopyToClipboard(content)) => {
                            copy_to_clipboard(&mut app, content).await;
                        }
                        _ => {}
                    }
                } else if app.screen == Screen::Errors && errors_action(input).is_some() {
                    let action = errors_action(input).expect("Errors action was checked");
                    if let Some(Effect::OpenInEditor(path)) =
                        compatibility_workspace_action(&mut app, action)
                    {
                        open_in_editor(&guard, &mut app, path, editor.as_deref()).await;
                    }
                } else if app.screen == Screen::Dependencies
                    && dependency_workspace_action(app.dependency_graph_searching, input).is_some()
                {
                    let action = dependency_workspace_action(app.dependency_graph_searching, input)
                        .expect("Dependency action was checked");
                    match compatibility_workspace_action(&mut app, action) {
                        Some(Effect::GetDependencies(recipe)) => {
                            load_dependency_graph(&mut app, backend.as_mut(), recipe).await;
                        }
                        Some(Effect::OpenInEditor(path)) => {
                            open_in_editor(&guard, &mut app, path, editor.as_deref()).await;
                        }
                        _ => {}
                    }
                } else if app.metadata_searching {
                    match input {
                        Input::Char(character) => {
                            let _ = compatibility_workspace_action(
                                &mut app,
                                Action::AppendMetadataQuery(character),
                            );
                        }
                        Input::Enter | Input::Esc => {
                            let _ = compatibility_workspace_action(
                                &mut app,
                                Action::FinishMetadataSearch,
                            );
                        }
                        Input::Backspace => {
                            let _ = compatibility_workspace_action(
                                &mut app,
                                Action::BackspaceMetadataQuery,
                            );
                        }
                        _ => {}
                    }
                } else if input == Input::Char('!') {
                    open_yocto_shell(&guard, &mut app).await;
                } else if input == Input::Char('i') {
                    let images = app
                        .workspace
                        .recipes
                        .iter()
                        .map(|recipe| recipe.name.as_str())
                        .filter(|name| name.contains("image"))
                        .map(str::to_owned)
                        .collect();
                    let _ =
                        compatibility_workspace_action(&mut app, Action::OpenImagePicker(images));
                } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('b') {
                    let _ =
                        compatibility_workspace_action(&mut app, Action::BeginSelectedRecipeBuild);
                } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('f') {
                    let _ = compatibility_workspace_action(
                        &mut app,
                        Action::BeginSelectedRecipeForceTask,
                    );
                } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('v') {
                    let _ = compatibility_workspace_action(
                        &mut app,
                        Action::BeginSelectedRecipeDevshell,
                    );
                } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('K') {
                    let _ = compatibility_workspace_action(
                        &mut app,
                        Action::BeginSelectedRecipeDiffconfig,
                    );
                } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('z') {
                    let _ = compatibility_workspace_action(
                        &mut app,
                        Action::BeginSelectedRecipeDiffsigs,
                    );
                } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('Z') {
                    let _ = compatibility_workspace_action(
                        &mut app,
                        Action::BeginSelectedRecipeSignatures,
                    );
                } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('V') {
                    let _ = compatibility_workspace_action(
                        &mut app,
                        Action::BeginSelectedRecipeCveCheck,
                    );
                } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('X') {
                    let _ =
                        compatibility_workspace_action(&mut app, Action::BeginSelectedRecipeSpdx);
                } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('e') {
                    if let Some(Effect::OpenInEditor(path)) =
                        compatibility_workspace_action(&mut app, Action::OpenSelectedRecipeProvider)
                    {
                        open_in_editor(&guard, &mut app, path, editor.as_deref()).await;
                    }
                } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('o') {
                    if let Some(Effect::OpenInEditor(path)) =
                        compatibility_workspace_action(&mut app, Action::BeginSelectedRecipeTaskLog)
                    {
                        open_in_editor(&guard, &mut app, path, editor.as_deref()).await;
                    }
                } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('p') {
                    if let Some(Effect::OpenInEditor(path)) = compatibility_workspace_action(
                        &mut app,
                        Action::BeginSelectedRecipePatchReview,
                    ) {
                        open_in_editor(&guard, &mut app, path, editor.as_deref()).await;
                    }
                } else if app.screen == yoctui_model::Screen::Dashboard
                    && let Some(action) = dashboard_workspace_action(input)
                {
                    let _ = compatibility_workspace_action(&mut app, action);
                } else if app.screen == yoctui_model::Screen::Dashboard
                    && collection_scroll_delta(input).is_some()
                {
                    let delta = collection_scroll_delta(input).expect("scroll key was checked");
                    let _ = compatibility_workspace_action(
                        &mut app,
                        Action::ScrollBuildTasks { delta },
                    );
                } else if app.screen == yoctui_model::Screen::BuildHistory
                    && collection_scroll_delta(input).is_some()
                {
                    let delta = collection_scroll_delta(input).expect("scroll key was checked");
                    let _ = compatibility_workspace_action(
                        &mut app,
                        Action::SelectBuildHistory { delta },
                    );
                } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('d') {
                    let root = match compatibility_workspace_action(
                        &mut app,
                        Action::BeginSelectedRecipeDevtoolModify,
                    ) {
                        Some(Effect::OpenWorkspaceEditor { label, root }) => Some((label, root)),
                        _ => None,
                    };
                    if let Some((recipe, root)) = root {
                        open_workspace_editor(&mut app, recipe, root).await;
                    }
                } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('t') {
                    inspect_selected_devtool(&mut app, &session_build_dir).await;
                } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('D') {
                    let _ = compatibility_workspace_action(
                        &mut app,
                        Action::BeginSelectedRecipeDevtoolReset,
                    );
                } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('u') {
                    let _ = compatibility_workspace_action(
                        &mut app,
                        Action::BeginSelectedRecipeDevtoolUpdateRecipe,
                    );
                } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('F') {
                    let _ = compatibility_workspace_action(
                        &mut app,
                        Action::BeginSelectedRecipeDevtoolFinish,
                    );
                } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('P') {
                    let _ = compatibility_workspace_action(
                        &mut app,
                        Action::BeginSelectedRecipeDevtoolDeploy,
                    );
                } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('A') {
                    if let Some(Effect::GetDependencies(recipe)) = compatibility_workspace_action(
                        &mut app,
                        Action::BeginSelectedRecipeDependencies,
                    ) {
                        load_dependency_graph(&mut app, backend.as_mut(), recipe).await;
                    }
                } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Enter {
                    if let Some(Effect::GetRecipeMetadata(recipe)) = compatibility_workspace_action(
                        &mut app,
                        Action::BeginSelectedRecipeMetadata,
                    ) {
                        match backend.get_recipe_metadata(recipe.clone()).await {
                            Ok(metadata) => {
                                let _ = compatibility_workspace_action(
                                    &mut app,
                                    Action::RecipeMetadataLoaded(metadata),
                                );
                            }
                            Err(error) => {
                                let _ = compatibility_workspace_action(
                                    &mut app,
                                    Action::RecipeMetadataFailed {
                                        recipe,
                                        message: error.to_string(),
                                    },
                                );
                            }
                        }
                    }
                    inspect_selected_devtool(&mut app, &session_build_dir).await;
                } else if input == Input::Char('b') {
                    let _ =
                        compatibility_workspace_action(&mut app, Action::BeginCurrentImageBuild);
                } else if app.screen == yoctui_model::Screen::Recipes
                    && collection_scroll_delta(input).is_some()
                {
                    let delta = collection_scroll_delta(input).expect("scroll key was checked");
                    let _ =
                        compatibility_workspace_action(&mut app, Action::SelectRecipe { delta });
                } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('C') {
                    let _ =
                        compatibility_workspace_action(&mut app, Action::BeginSelectedRecipeClean);
                } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('M') {
                    let _ = compatibility_workspace_action(
                        &mut app,
                        Action::BeginSelectedRecipeMenuConfig,
                    );
                } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('S') {
                    let _ = compatibility_workspace_action(
                        &mut app,
                        Action::BeginSelectedRecipeCleanState,
                    );
                } else if app.screen == yoctui_model::Screen::Layers
                    && collection_scroll_delta(input).is_some()
                {
                    let delta = collection_scroll_delta(input).expect("scroll key was checked");
                    let _ = compatibility_workspace_action(&mut app, Action::SelectLayer { delta });
                } else if let Some(action) = layer_list_open_action(&app, input) {
                    if let Some(Effect::LoadLayerBrowserDirectory {
                        layer,
                        root,
                        directory,
                    }) = compatibility_workspace_action(&mut app, action)
                    {
                        load_layer_browser_directory(&mut app, layer, root, directory).await;
                    }
                } else if app.screen == yoctui_model::Screen::Layers && input == Input::Char('o') {
                    if let Some(Effect::OpenInEditor(path)) =
                        compatibility_workspace_action(&mut app, Action::OpenSelectedLayer)
                    {
                        open_in_editor(&guard, &mut app, path, editor.as_deref()).await;
                    }
                } else if app.screen == yoctui_model::Screen::Layers && input == Input::Char('e') {
                    if let Some(Effect::OpenWorkspaceEditor { label, root }) =
                        compatibility_workspace_action(
                            &mut app,
                            Action::BeginSelectedLayerWorkspaceEditor,
                        )
                    {
                        open_workspace_editor(&mut app, label, root).await;
                    }
                } else if app.screen == yoctui_model::Screen::Layers && input == Input::Char('R') {
                    if matches!(
                        compatibility_workspace_action(&mut app, Action::BeginLayerRelationships),
                        Some(Effect::GetLayerRelationships)
                    ) {
                        match backend.get_layer_relationships().await {
                            Ok(layers) => {
                                let _ = compatibility_workspace_action(
                                    &mut app,
                                    Action::LayerRelationshipsLoaded(LayerRelationships {
                                        layers: layers
                                            .into_iter()
                                            .map(|layer| LayerRelationship {
                                                name: layer.name,
                                                priority: layer.priority,
                                                compatible: layer.compatible,
                                                depends: layer.depends,
                                                overlays: layer.overlays,
                                                appends: layer.appends,
                                            })
                                            .collect(),
                                    }),
                                );
                            }
                            Err(error) => {
                                let _ = compatibility_workspace_action(
                                    &mut app,
                                    Action::Failure(AppError::new(
                                        "Layers",
                                        error.to_string(),
                                        "use a bridge connected to a BitBake server that supports get_layer_relationships",
                                    )),
                                );
                            }
                        }
                    }
                } else if app.screen == yoctui_model::Screen::RawMode
                    && (app.focus == yoctui_model::FocusTarget::Workspace
                        || matches!(
                            app.raw_mode.view,
                            yoctui_model::RawModeView::Form | yoctui_model::RawModeView::Preview
                        ))
                {
                    if let Some(action) = raw_mode_input(&app, input) {
                        let effect =
                            compatibility_workspace_action(&mut app, Action::RawMode(action));
                        if matches!(effect.as_ref(), Some(Effect::PersistSettings)) {
                            let favorites = app.raw_mode.favorites.clone();
                            if let Err(error) = persist_raw_favorites(
                                session_path.as_deref(),
                                &mut session,
                                &favorites,
                            ) {
                                app.notification =
                                    Some(format!("Raw favorites could not be saved: {error}"));
                            }
                        }
                        if let Some(
                            effect @ (Effect::StartRaw(_)
                            | Effect::CancelRaw(_)
                            | Effect::SetRawAttachment { .. }),
                        ) = effect
                        {
                            #[cfg(unix)]
                            if let Some(runtime) = daemon_runtime.as_mut() {
                                match runtime.route_effect(&app, &effect) {
                                    Ok(client_runtime::RuntimeEffectRoute::Daemon(_)) => {}
                                    Ok(client_runtime::RuntimeEffectRoute::ClientLocal) => {
                                        app.notification = Some(
                                            "Raw execution was not routed to the daemon.".into(),
                                        );
                                    }
                                    Err(error) => {
                                        app.notification =
                                            Some(format!("Raw execution was not sent: {error}"));
                                    }
                                }
                            } else {
                                app.notification = Some(
                                    "Raw execution requires an attached Yoctui daemon.".into(),
                                );
                            }
                        }
                    }
                } else if app.screen == yoctui_model::Screen::Compatibility {
                    if let Some(action) =
                        compatibility_ui_inspector_action(app.compatibility_ui.searching, input)
                    {
                        let _ = compatibility_workspace_action(&mut app, action);
                    }
                } else if app.screen == yoctui_model::Screen::Configuration
                    && collection_scroll_delta(input).is_some()
                {
                    if let Some(action) = config_workspace_action(false, input) {
                        let _ = compatibility_workspace_action(&mut app, action);
                    }
                } else if app.screen == yoctui_model::Screen::Configuration && input == Input::Enter
                {
                    inspect_selected_config_variable(&mut app, backend.as_mut()).await;
                } else if app.screen == yoctui_model::Screen::Configuration
                    && matches!(
                        input,
                        Input::Char('s') | Input::Char('c') | Input::Char('E')
                    )
                {
                    if let Some(action) = config_workspace_action(false, input) {
                        let _ = compatibility_workspace_action(&mut app, action);
                    }
                } else if app.screen == yoctui_model::Screen::Configuration
                    && matches!(input, Input::Char('C') | Input::Char('U'))
                {
                    if let Some(Effect::CopyToClipboard(content)) =
                        config_copy_effect(&mut app, input)
                    {
                        copy_to_clipboard(&mut app, content).await;
                    }
                } else if app.screen == yoctui_model::Screen::Configuration
                    && input == Input::Char('o')
                {
                    if let Some(Effect::OpenInEditor(path)) =
                        compatibility_workspace_action(&mut app, Action::OpenSelectedConfigSource)
                    {
                        open_in_editor(&guard, &mut app, path, editor.as_deref()).await;
                    }
                } else if app.screen == yoctui_model::Screen::Bbmask && input == Input::Char('e') {
                    let _ = compatibility_workspace_action(&mut app, Action::BeginBbmaskEdit);
                } else if matches!(
                    app.screen,
                    yoctui_model::Screen::Recipes
                        | yoctui_model::Screen::Layers
                        | yoctui_model::Screen::Configuration
                ) && input == Input::Char('/')
                {
                    let _ = compatibility_workspace_action(&mut app, Action::BeginMetadataSearch);
                } else if app.logs.searching {
                    match input {
                        Input::Char(character) => {
                            let _ = compatibility_workspace_action(
                                &mut app,
                                Action::AppendLogQuery(character),
                            );
                        }
                        Input::Enter | Input::Esc => {
                            let _ =
                                compatibility_workspace_action(&mut app, Action::FinishLogSearch);
                        }
                        Input::Backspace => {
                            let _ =
                                compatibility_workspace_action(&mut app, Action::BackspaceLogQuery);
                        }
                        _ => {}
                    }
                } else if let Some(action) = keymap_action_for_app(&mut app, input).action() {
                    if matches!(action, Action::Cancel) {
                        if devtool_jobs.active_job_id().is_some() {
                            if let Some(job_action) = devtool_jobs.request_cancellation() {
                                let _ = compatibility_workspace_action(&mut app, job_action);
                            }
                            let cancellation = if let Some(runner) = devtool_runner.as_mut() {
                                runner.cancel().await.map(|_| ())
                            } else {
                                Err(yoctui_bitbake::DevtoolRunnerError::NotRunning)
                            };
                            if let Err(error) = cancellation {
                                for action in devtool_jobs
                                    .cancellation_failed(error.to_string(), SystemTime::now())
                                {
                                    let _ = compatibility_workspace_action(&mut app, action);
                                }
                            }
                        } else if let Some(effect @ Effect::Cancel) =
                            compatibility_workspace_action(&mut app, action)
                        {
                            #[cfg(unix)]
                            if let Some(runtime) = daemon_runtime.as_mut() {
                                match runtime.route_effect(&app, &effect) {
                                    Ok(client_runtime::RuntimeEffectRoute::Daemon(_)) => continue,
                                    Ok(client_runtime::RuntimeEffectRoute::ClientLocal) => {}
                                    Err(error) => {
                                        app.notification = Some(format!(
                                            "Daemon cancellation was not sent: {error}"
                                        ));
                                        continue;
                                    }
                                }
                            }
                            if let Some(job_action) = build_jobs.request_cancellation() {
                                let _ = compatibility_workspace_action(&mut app, job_action);
                            }
                            if let Err(error) = backend.cancel_build().await {
                                for action in build_jobs
                                    .cancellation_failed(error.to_string(), SystemTime::now())
                                {
                                    let _ = compatibility_workspace_action(&mut app, action);
                                }
                            }
                        }
                    } else {
                        if let Some(effect) = compatibility_workspace_action(&mut app, action) {
                            if local_workspace_effect_route(&effect)
                                == LocalWorkspaceEffectRoute::ImageArtifacts
                            {
                                begin_image_artifact_operation(
                                    &mut app,
                                    image_artifact_adapter.as_ref(),
                                    &mut image_artifact_operation,
                                    effect,
                                );
                            } else if local_workspace_effect_route(&effect)
                                == LocalWorkspaceEffectRoute::RootfsComposition
                            {
                                begin_rootfs_composition_operation(
                                    backend.as_mut(),
                                    &mut app,
                                    &session_build_dir,
                                    &mut rootfs_composition_operation,
                                    effect,
                                    daemon_attached,
                                )
                                .await;
                            } else if !route_independent_security_effect(
                                &guard,
                                &mut app,
                                &mut security_coordinator,
                                effect.clone(),
                                editor.as_deref(),
                            )
                            .await
                                && !route_independent_qa_effect(
                                    &guard,
                                    &mut app,
                                    &mut qa_coordinator,
                                    effect.clone(),
                                    editor.as_deref(),
                                )
                                .await
                                && !route_independent_maintenance_effect(
                                    &guard,
                                    &mut app,
                                    &mut maintenance_coordinator,
                                    effect.clone(),
                                    editor.as_deref(),
                                )
                                .await
                            {
                                let _ = test_coordinator.handle_effect(&mut app, effect).await;
                            }
                        }
                    }
                }
            }
        }
        let devtool_was_active = devtool_runner.is_some();
        let completed_devtool =
            poll_devtool_job(&mut app, &mut devtool_jobs, &mut devtool_runner).await;
        render_scheduler.invalidate_if(devtool_was_active, RenderCause::Presentation);
        match completed_devtool {
            Some(DevtoolOperation::Modify { recipe })
                if pending_devtool_modify
                    .as_ref()
                    .is_some_and(|identity| identity.name == recipe) =>
            {
                if let Some(identity) = pending_devtool_modify.take() {
                    complete_devtool_modify(&mut app, &session_build_dir, identity).await;
                }
            }
            Some(DevtoolOperation::UpdateRecipe { recipe })
                if pending_devtool_update
                    .as_ref()
                    .is_some_and(|identity| identity.name == recipe) =>
            {
                if let Some(identity) = pending_devtool_update.take() {
                    complete_devtool_update(&mut app, &session_build_dir, identity).await;
                }
            }
            Some(DevtoolOperation::Finish { recipe, .. })
                if pending_devtool_finish
                    .as_ref()
                    .is_some_and(|identity| identity.name == recipe) =>
            {
                if let Some(identity) = pending_devtool_finish.take() {
                    complete_devtool_finish(&mut app, &session_build_dir, identity).await;
                }
            }
            Some(DevtoolOperation::DeployTarget { recipe, .. })
                if pending_devtool_deploy
                    .as_ref()
                    .is_some_and(|identity| identity.name == recipe) =>
            {
                if let Some(identity) = pending_devtool_deploy.take() {
                    complete_devtool_deploy(&mut app, &session_build_dir, identity).await;
                }
            }
            Some(DevtoolOperation::Reset { recipe })
                if pending_devtool_reset
                    .as_ref()
                    .is_some_and(|identity| identity.name == recipe) =>
            {
                if let Some(identity) = pending_devtool_reset.take() {
                    complete_devtool_reset(&mut app, &session_build_dir, identity).await;
                }
            }
            _ if devtool_jobs.active_operation().is_none() => {
                pending_devtool_modify = None;
                pending_devtool_update = None;
                pending_devtool_finish = None;
                pending_devtool_deploy = None;
                pending_devtool_reset = None;
            }
            _ => {}
        }
        if build_jobs.active_job_id().is_some() {
            match tokio::time::timeout(Duration::from_millis(1), backend.next_event()).await {
                Ok(Ok(event)) => {
                    render_scheduler.invalidate(RenderCause::State);
                    let test_terminal = matches!(
                        event,
                        BackendEvent::BuildCompleted { .. }
                            | BackendEvent::CommandFailed { .. }
                            | BackendEvent::Disconnected
                    );
                    let security_terminal = test_terminal;
                    let qa_terminal = test_terminal;
                    if let Some(id) = pending_test_build
                        && let Some(action) = test_build_action_for_event(&app, id, &event)
                    {
                        let _ = compatibility_workspace_action(&mut app, action);
                    }
                    let security_followup = pending_security_build
                        .and_then(|id| security_build_action_for_event(&app, id, &event))
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                    let qa_followup = pending_qa_build
                        .and_then(|id| qa_build_action_for_event(&app, id, &event))
                        .and_then(|action| compatibility_workspace_action(&mut app, action));
                    let sdk_refresh =
                        sdk_refresh_after_build_event(&mut app, &mut pending_sdk_build, &event);
                    for action in build_jobs.actions_for_backend_event(event, SystemTime::now()) {
                        let _ = compatibility_workspace_action(&mut app, action);
                    }
                    if test_terminal {
                        pending_test_build = None;
                    }
                    if security_terminal {
                        pending_security_build = None;
                    }
                    if qa_terminal {
                        pending_qa_build = None;
                    }
                    if let Some(effect) = security_followup {
                        let _ = security_coordinator.handle_effect(&mut app, effect).await;
                    }
                    if let Some(effect) = qa_followup {
                        let _ = qa_coordinator.handle_effect(&mut app, effect).await;
                    }
                    if let Some(effect) = sdk_refresh {
                        begin_sdk_artifact_operation(
                            &mut app,
                            sdk_artifact_adapter.as_ref(),
                            &mut sdk_artifact_operation,
                            effect,
                        );
                    }
                }
                Ok(Err(error)) => {
                    render_scheduler.invalidate(RenderCause::State);
                    if let Some(id) = pending_test_build.take() {
                        let _ = update(
                            &mut app,
                            Action::LoseTestSession {
                                id,
                                message: error.to_string(),
                                finished_at: SystemTime::now(),
                            },
                        );
                    }
                    if let Some(id) = pending_security_build.take() {
                        let _ = update(
                            &mut app,
                            Action::Security(SecurityAction::LoseSession {
                                id,
                                message: error.to_string(),
                                finished_at: SystemTime::now(),
                            }),
                        );
                    }
                    if let Some(id) = pending_qa_build.take() {
                        let _ = update(
                            &mut app,
                            Action::Qa(QaAction::LoseSession {
                                session: id,
                                message: error.to_string(),
                                finished_at: SystemTime::now(),
                            }),
                        );
                    }
                    pending_sdk_build = None;
                    for action in build_jobs.backend_lost(error.to_string(), SystemTime::now()) {
                        let _ = compatibility_workspace_action(&mut app, action);
                    }
                }
                Err(_) => {}
            }
        }
        if app.should_quit {
            break;
        }
    }
    let render_metrics = render_scheduler.metrics();
    if let Some(path) = std::env::var_os("YOCTUI_PERFORMANCE_METRICS_PATH") {
        let elapsed_seconds = render_measurement_started.elapsed().as_secs_f64();
        let report = serde_json::json!({
            "schema": "yoctui.performance.render.v1",
            "elapsed_seconds": elapsed_seconds,
            "requests": render_metrics.requests,
            "frames": render_metrics.frames,
            "coalesced": render_metrics.coalesced,
            "skipped_checks": render_metrics.skipped_checks,
            "frames_per_second": render_metrics.frames as f64 / elapsed_seconds.max(f64::EPSILON),
        });
        if let Err(error) = std::fs::write(path, format!("{report}\n")) {
            tracing::warn!(%error, "could not write requested render performance metrics");
        }
    }
    tracing::debug!(
        requests = render_metrics.requests,
        frames = render_metrics.frames,
        coalesced = render_metrics.coalesced,
        skipped_checks = render_metrics.skipped_checks,
        "interactive render scheduler stopped"
    );
    #[cfg(unix)]
    if let Some(runtime) = daemon_runtime.take()
        && let Err(error) = runtime.detach(&mut app)
    {
        tracing::warn!(%error, "daemon detach failed during client shutdown");
    }
    if let Some(operation) = signature_operation.take() {
        operation.cancellation.cancel();
        let _ = operation.handle.await;
    }
    if let Some(operation) = image_artifact_operation.take() {
        operation.cancellation.cancel();
        let _ = operation.handle.await;
    }
    if let Some(operation) = sdk_artifact_operation.take() {
        operation.cancellation.cancel();
        let _ = operation.handle.await;
    }
    if let Some(operation) = sdk_capability_operation.take() {
        operation.handle.abort();
        let _ = operation.handle.await;
    }
    if let Some(mut operation) = sdk_operation.take() {
        if let Some(handle) = operation.starting.take() {
            handle.abort();
            let _ = handle.await;
        } else if let Some(handle) = operation.timeout_wait.take() {
            handle.abort();
            let _ = handle.await;
        } else if let Some(handle) = operation.cancellation.take() {
            handle.abort();
            let _ = handle.await;
        } else if let Some(mut runner) = operation.runner.take() {
            let _ = runner.cancel().await;
        }
    }
    if let Some(mut operation) = qemu_operation.take() {
        if let Some(handle) = operation.cancellation.take() {
            handle.abort();
            let _ = handle.await;
        } else if let Some(mut runner) = operation.runner.take() {
            let _ = runner.cancel().await;
        }
    }
    if let Some(operation) = wic_capability_operation.take() {
        operation.handle.abort();
        let _ = operation.handle.await;
    }
    if let Some(mut operation) = wic_operation.take() {
        if let Some(handle) = operation.cancellation.take() {
            handle.abort();
            let _ = handle.await;
        } else if let Some(mut runner) = operation.runner.take() {
            let _ = runner.cancel().await;
        }
    }
    if let Some(mut operation) = test_coordinator.session.take() {
        let _ = operation.runner.cancel().await;
    }
    if let Some(mut operation) = test_coordinator.result.take() {
        let _ = operation.runner.cancel().await;
    }
    if let Some(operation) = test_coordinator.import.take() {
        operation.handle.abort();
        let _ = operation.handle.await;
    }
    if let Some(operation) = security_coordinator.capability.take() {
        operation.handle.abort();
        let _ = operation.handle.await;
    }
    if let Some(operation) = security_coordinator.report.take() {
        operation.cancellation.cancel();
        operation.handle.abort();
        let _ = operation.handle.await;
    }
    if let Some(mut operation) = security_coordinator.mapper.take() {
        let _ = operation.runner.cancel(operation.id).await;
    }
    maintenance_coordinator.shutdown().await;
    backend.shutdown().await?;
    let raw_favorites = app.raw_mode.favorites.clone();
    let mut preferences = app.effective_preferences();
    if color_forced_off {
        preferences.color_enabled = app.preferences.color_enabled;
    }
    session.last_target = app.build.target;
    session.last_screen = Some(app.screen);
    session.log_filter = app.logs.filter;
    session.log_recipe_filter = app.logs.recipe_filter;
    session.log_task_filter = app.logs.task_filter;
    session.log_build_filter = app.logs.build_filter;
    session.pane_layout = preferences
        .remember_pane_sizes
        .then(|| app.pane_layout.clone());
    session.preferences = Some(preferences);
    session.log_wrap = None;
    session.log_follow = None;
    session.theme = None;
    session.animation_speed = None;
    session.reduced_motion = None;
    session.color_enabled = None;
    session.keymap = yoctui_model::KeymapPreferences::default();
    session.onboarding = Some(app.onboarding.progress.clone());
    session.recent_build_dirs = std::iter::once(session_build_dir)
        .chain(session.recent_build_dirs)
        .fold(Vec::new(), |mut directories, directory| {
            if !directories.contains(&directory) && directories.len() < 10 {
                directories.push(directory);
            }
            directories
        });
    persist_raw_favorites(session_path.as_deref(), &mut session, &raw_favorites)?;
    Ok(())
}
