//! Interactive runtime.
use super::*;

mod build_dialogs;
mod command_dialogs;
mod dependency_workspace;
mod devtool_status_operation;
mod editor_dialogs;
mod extended_devtool_dialogs;
mod input;
mod jobs;
mod key_input;
mod metadata_backend;
#[cfg(test)]
pub(crate) use metadata_backend::metadata_backend_start_required;

pub(crate) fn begin_startup_platform_inspection(app: &mut App) -> Option<Screen> {
    match app.screen {
        Screen::Kernel => matches!(
            compatibility_workspace_action(app, Action::InspectKernel),
            Some(Effect::InspectKernel)
        )
        .then_some(Screen::Kernel),
        Screen::Firmware => matches!(
            compatibility_workspace_action(app, Action::InspectFirmware),
            Some(Effect::InspectFirmware)
        )
        .then_some(Screen::Firmware),
        _ => None,
    }
}
mod metadata_workspaces;
mod mouse_input;
mod paste_input;
mod platform_inspection_operation;
mod polling;
mod primary_workspaces;
mod recipe_inspection_operation;
mod remaining_workspaces;
mod runtime_loop;
mod sdk_test_dialogs;
mod shutdown;
mod state;
use key_input::KeyRouteOutcome;
use state::InteractiveRuntime;

pub(crate) async fn tui(
    config: Config,
    targets: Vec<String>,
    session: Session,
    internal_tracing_capture: internal_tracing::InternalTracingCapture,
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
        editor: editor_command,
        session_path,
        ..
    } = config;
    // Yoctui resolves color itself (including --no-color and the persisted
    // Settings value), so Crossterm must not silently apply a second ambient
    // NO_COLOR policy that contradicts the visible setting.
    crossterm::style::force_color_output(true);
    let guard = TerminalGuard::enter()?;
    let terminal = Terminal::new(ratatui::backend::CrosstermBackend::new(io::stdout()))?;
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
    let daemon_runtime = match client_runtime::InteractiveDaemonRuntime::connect(
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
    let next_daemon_reconnect = Instant::now() + client_runtime::DAEMON_RECONNECT_INTERVAL;
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
    let backend: Box<dyn BitBakeBackend> = Box::new(ProcessBackend::new(build_dir.clone()));
    let metadata_backend_authoritative = false;
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
    let build_jobs = BuildJobCoordinator::default();
    let devtool_jobs = DevtoolJobCoordinator::default();
    let devtool_runner = None;
    let pending_devtool_modify = None;
    let pending_daemon_devtool_modify = None;
    let pending_devtool_update = None;
    let pending_devtool_finish = None;
    let pending_devtool_deploy = None;
    let pending_devtool_undeploy = None;
    let pending_devtool_upgrade = None;
    let pending_devtool_reset = None;
    let signature_adapter = SignatureAdapter::new(session_build_dir.clone());
    let signature_operation = None;
    let package_adapter = PackageDataAdapter::new(session_build_dir.clone());
    let mut package_operation = None;
    let image_artifact_adapter = app
        .workspace
        .variables
        .get("DEPLOY_DIR_IMAGE")
        .map(PathBuf::from)
        .map(ImageArtifactAdapter::new);
    let mut image_artifact_operation = None;
    let rootfs_composition_operation = None;
    let global_content_search_operation = None;
    let history_root = daemon_state_root()?;
    let history_load = None;
    app.saved_builds.reload_requested = true;
    let clone_operation = None;
    let source_git_poller = source_git::SourceGitPoller::default();
    let environment_operation = None;
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
    let sdk_artifact_operation = None;
    let sdk_capability_operation = None;
    let sdk_operation = None;
    let pending_sdk_build = None;
    let qemu_inspector = QemuCapabilityInspector::default();
    let qemu_operation = None;
    let wic_inspector = wic_capability_inspector(&app);
    let wic_capability_operation = None;
    let wic_device_inspector = WicDeviceInspector::default();
    let wic_device_operation = None;
    let wic_operation = None;
    let initialized_paths = initialized_path_directories();
    let mut test_coordinator = TestCliCoordinator::new(
        session_build_dir.clone(),
        initialized_paths.clone(),
        ptest_capability(&app),
    );
    let pending_test_build = None;
    let mut security_coordinator =
        SecurityCliCoordinator::new(session_build_dir.clone(), initialized_paths.clone());
    let pending_security_build = None;
    let mut qa_coordinator =
        QaCliCoordinator::new(session_build_dir.clone(), initialized_paths.clone());
    let pending_qa_build = None;
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
    let startup_platform_inspection = begin_startup_platform_inspection(&mut app);
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
    let telemetry_sampler = HostTelemetrySampler::default();
    let next_telemetry_sample = Instant::now();
    let telemetry_was_visible = client_telemetry_visible(&app);
    let next_animation_tick = Instant::now();
    let next_elapsed_refresh = Instant::now();
    let frame_interval = interactive_frame_interval(refresh);
    let render_scheduler = RenderScheduler::default();
    let environment_browser_io = environment_setup::EnvironmentBrowserIo::default();
    let recipe_inspection_operation = None;
    let devtool_status_operation = None;
    let platform_inspection_operation = None;
    let render_measurement_started = Instant::now();
    let prefix_state = PrefixState::default();
    #[cfg(unix)]
    let termination = termination_receiver()?;
    let runtime = InteractiveRuntime {
        guard,
        terminal,
        app,
        #[cfg(unix)]
        daemon_runtime,
        #[cfg(unix)]
        next_daemon_reconnect,
        daemon_attached,
        backend_kind,
        backend,
        metadata_backend_authoritative,
        session,
        session_build_dir,
        session_path,
        cancellation_timeout,
        color_forced_off,
        editor_command,
        internal_tracing_capture,
        build_jobs,
        devtool_jobs,
        devtool_runner,
        pending_devtool_modify,
        pending_daemon_devtool_modify,
        pending_devtool_update,
        pending_devtool_finish,
        pending_devtool_deploy,
        pending_devtool_undeploy,
        pending_devtool_upgrade,
        pending_devtool_reset,
        signature_adapter,
        signature_operation,
        package_adapter,
        package_operation,
        image_artifact_adapter,
        image_artifact_operation,
        rootfs_composition_operation,
        global_content_search_operation,
        history_root,
        history_load,
        clone_operation,
        source_git_poller,
        environment_operation,
        sdk_artifact_adapter,
        sdk_tool_adapter,
        sdk_artifact_operation,
        sdk_capability_operation,
        sdk_operation,
        pending_sdk_build,
        qemu_inspector,
        qemu_operation,
        wic_inspector,
        wic_capability_operation,
        wic_device_inspector,
        wic_device_operation,
        wic_operation,
        test_coordinator,
        pending_test_build,
        security_coordinator,
        pending_security_build,
        qa_coordinator,
        pending_qa_build,
        maintenance_coordinator,
        telemetry_sampler,
        next_telemetry_sample,
        telemetry_was_visible,
        next_animation_tick,
        next_elapsed_refresh,
        frame_interval,
        render_scheduler,
        environment_browser_io,
        recipe_inspection_operation,
        devtool_status_operation,
        platform_inspection_operation,
        render_measurement_started,
        prefix_state,
        startup_platform_inspection,
        #[cfg(unix)]
        termination,
    };
    runtime.run().await
}
