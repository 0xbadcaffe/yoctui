use super::*;

impl InteractiveRuntime {
    pub(super) async fn poll_runtime(&mut self) -> Result<bool> {
        let runtime = self;
        let (internal_records, ingress_dropped) = runtime.internal_tracing_capture.drain(256);
        runtime.render_scheduler.invalidate_if(
            runtime.environment_browser_io.poll(&mut runtime.app).await,
            RenderCause::State,
        );
        let recipe_metadata_changed = runtime.poll_recipe_inspection().await;
        runtime
            .render_scheduler
            .invalidate_if(recipe_metadata_changed, RenderCause::State);
        let devtool_status_changed = runtime.poll_devtool_status().await;
        runtime
            .render_scheduler
            .invalidate_if(devtool_status_changed, RenderCause::State);
        let platform_inspection_changed = runtime.poll_platform_inspection().await;
        runtime
            .render_scheduler
            .invalidate_if(platform_inspection_changed, RenderCause::State);
        if ingress_dropped > 0 {
            let _ = update(
                &mut runtime.app,
                Action::InternalLogIngressDropped(ingress_dropped),
            );
            runtime.render_scheduler.invalidate(RenderCause::State);
        }
        if !internal_records.is_empty() {
            runtime.render_scheduler.invalidate(RenderCause::State);
        }
        for record in internal_records {
            let _ = update(&mut runtime.app, Action::InternalLog(record));
        }
        #[cfg(unix)]
        if termination_requested(&mut runtime.termination) {
            return Ok(true);
        }
        #[cfg(unix)]
        let daemon_poll_result = runtime
            .daemon_runtime
            .as_mut()
            .map(|daemon_client| daemon_client.poll(&mut runtime.app));
        #[cfg(unix)]
        let daemon_poll_error = match daemon_poll_result {
            Some(Ok(changed)) => {
                runtime
                    .render_scheduler
                    .invalidate_if(changed, RenderCause::State);
                None
            }
            Some(Err(error)) => Some(error),
            None => None,
        };
        #[cfg(unix)]
        if let Some(error) = daemon_poll_error {
            tracing::warn!(%error, "yoctui daemon client disconnected; reconnecting");
            runtime.daemon_runtime = None;
            runtime.app.daemon.status = yoctui_model::ClientReplicaStatus::Disconnected;
            yoctui_model::invalidate_workspace_compatibility(&mut runtime.app);
            let _ = update(
                &mut runtime.app,
                Action::BuildAuthorityLost {
                    message: error.to_string(),
                },
            );
            runtime.render_scheduler.invalidate(RenderCause::State);
            runtime.next_daemon_reconnect =
                Instant::now() + client_runtime::DAEMON_RECONNECT_INTERVAL;
        }
        #[cfg(unix)]
        if runtime.daemon_runtime.is_none() && Instant::now() >= runtime.next_daemon_reconnect {
            runtime.next_daemon_reconnect =
                Instant::now() + client_runtime::DAEMON_RECONNECT_INTERVAL;
            match client_runtime::InteractiveDaemonRuntime::connect(
                &mut runtime.app,
                Duration::from_millis(250),
            ) {
                Ok(daemon_client) => {
                    runtime.daemon_runtime = Some(daemon_client);
                    runtime.render_scheduler.invalidate(RenderCause::State);
                }
                Err(error) => tracing::debug!(%error, "daemon reattach not yet available"),
            }
        }
        if let Some(identity) = runtime.pending_daemon_devtool_modify.as_ref() {
            match daemon_devtool_modify_completion(&runtime.app, identity) {
                DaemonDevtoolModifyCompletion::Pending => {}
                DaemonDevtoolModifyCompletion::Succeeded => {
                    let identity = runtime
                        .pending_daemon_devtool_modify
                        .take()
                        .expect("daemon Devtool identity was present");
                    complete_devtool_modify(&mut runtime.app, &runtime.session_build_dir, identity)
                        .await;
                    runtime.render_scheduler.invalidate(RenderCause::State);
                }
                DaemonDevtoolModifyCompletion::Failed => {
                    let identity = runtime
                        .pending_daemon_devtool_modify
                        .take()
                        .expect("daemon Devtool identity was present");
                    runtime.app.notification = Some(format!(
                        "Devtool modify {} failed in the daemon; inspect Jobs and Logs.",
                        identity.name
                    ));
                    runtime.render_scheduler.invalidate(RenderCause::State);
                }
            }
        }
        if build_archive::poll_load(
            &mut runtime.app,
            &mut runtime.history_load,
            &runtime.history_root,
        )
        .await
        {
            runtime.render_scheduler.invalidate(RenderCause::State);
        }
        if runtime.source_git_poller.poll(&mut runtime.app).await {
            runtime.render_scheduler.invalidate(RenderCause::State);
        }
        let local_operation_active = runtime.history_load.is_some()
            || runtime.environment_operation.is_some()
            || runtime.clone_operation.is_some()
            || runtime.signature_operation.is_some()
            || runtime.package_operation.is_some()
            || runtime.image_artifact_operation.is_some()
            || runtime.rootfs_composition_operation.is_some()
            || runtime.global_content_search_operation.is_some()
            || runtime.recipe_inspection_operation.is_some()
            || runtime.devtool_status_operation.is_some()
            || runtime.platform_inspection_operation.is_some()
            || runtime.sdk_artifact_operation.is_some()
            || runtime.sdk_capability_operation.is_some()
            || runtime.sdk_operation.is_some()
            || runtime.qemu_operation.is_some()
            || runtime.wic_capability_operation.is_some()
            || runtime.wic_device_operation.is_some()
            || runtime.wic_operation.is_some()
            || runtime.test_coordinator.session.is_some()
            || runtime.test_coordinator.import.is_some()
            || runtime.test_coordinator.result.is_some()
            || runtime.security_coordinator.capability.is_some()
            || runtime.security_coordinator.report.is_some()
            || runtime.security_coordinator.mapper.is_some()
            || runtime.qa_coordinator.capability.is_some()
            || runtime.qa_coordinator.layer_capability.is_some()
            || runtime.qa_coordinator.report.is_some()
            || runtime.qa_coordinator.layer.is_some()
            || runtime.maintenance_coordinator.operation_active();
        environment_operation::poll(
            &mut runtime.app,
            &mut runtime.backend,
            &mut runtime.environment_operation,
        )
        .await;
        for (activity, active) in [
            (
                yoctui_model::BackgroundActivity::Initializing,
                runtime.environment_operation.is_some(),
            ),
            (
                yoctui_model::BackgroundActivity::Cancelling,
                runtime.app.build.status == yoctui_model::BuildStatus::Cancelling,
            ),
            (
                yoctui_model::BackgroundActivity::Loading,
                local_operation_active
                    && runtime.clone_operation.is_none()
                    && runtime.environment_operation.is_none(),
            ),
        ] {
            compatibility_workspace_action(
                &mut runtime.app,
                Action::SetBackgroundActivity { activity, active },
            );
        }
        clone_operation::poll(&mut runtime.app, &mut runtime.clone_operation).await;
        poll_signature_operation(&mut runtime.app, &mut runtime.signature_operation).await;
        poll_package_operation(&mut runtime.app, &mut runtime.package_operation).await;
        poll_image_artifact_operation(
            &mut runtime.app,
            &mut runtime.image_artifact_operation,
            &runtime.qemu_inspector,
            &runtime.wic_inspector,
            &mut runtime.wic_capability_operation,
        )
        .await;
        poll_rootfs_composition_operation(
            &mut runtime.app,
            &mut runtime.rootfs_composition_operation,
        )
        .await;
        poll_global_content_search(
            &mut runtime.app,
            &mut runtime.global_content_search_operation,
        )
        .await;
        poll_sdk_artifact_operation(&mut runtime.app, &mut runtime.sdk_artifact_operation).await;
        poll_sdk_capability_operation(&mut runtime.app, &mut runtime.sdk_capability_operation)
            .await;
        if let Some(completed) = poll_sdk_job(&mut runtime.app, &mut runtime.sdk_operation).await
            && matches!(completed, SdkOperation::Publish(_))
            && let Some(effect) = update(&mut runtime.app, Action::RefreshSdkArtifactInventory)
        {
            begin_sdk_artifact_operation(
                &mut runtime.app,
                runtime.sdk_artifact_adapter.as_ref(),
                &mut runtime.sdk_artifact_operation,
                effect,
            );
        }
        poll_wic_capability_operation(
            &mut runtime.app,
            &runtime.wic_inspector,
            &mut runtime.wic_capability_operation,
        )
        .await;
        poll_wic_device_operation(&mut runtime.app, &mut runtime.wic_device_operation).await;
        poll_qemu_job(&mut runtime.app, &mut runtime.qemu_operation).await;
        poll_wic_job(&mut runtime.app, &mut runtime.wic_operation).await;
        runtime.test_coordinator.poll(&mut runtime.app).await;
        runtime.security_coordinator.poll(&mut runtime.app).await;
        runtime.qa_coordinator.poll(&mut runtime.app).await;
        runtime.maintenance_coordinator.poll(&mut runtime.app).await;
        runtime
            .render_scheduler
            .invalidate_if(local_operation_active, RenderCause::Presentation);
        let telemetry_now = Instant::now();
        let telemetry_visible = client_telemetry_visible(&runtime.app);
        if telemetry_visible != runtime.telemetry_was_visible {
            runtime.next_telemetry_sample = telemetry_now;
            runtime.telemetry_was_visible = telemetry_visible;
        }
        if telemetry_now >= runtime.next_telemetry_sample {
            let telemetry = runtime.telemetry_sampler.sample(&runtime.session_build_dir);
            let telemetry_changed = telemetry != runtime.app.host_telemetry;
            let _ = update(&mut runtime.app, Action::HostTelemetryUpdated(telemetry));
            runtime.next_telemetry_sample = telemetry_now + client_telemetry_interval(&runtime.app);
            runtime.render_scheduler.invalidate_if(
                telemetry_visible && telemetry_changed,
                RenderCause::Telemetry,
            );
        }
        let presentation_now = Instant::now();
        let visible_animation = has_visible_indeterminate_activity(&runtime.app);
        if visible_animation && presentation_now >= runtime.next_animation_tick {
            let _ = update(&mut runtime.app, Action::Tick);
            runtime.next_animation_tick = presentation_now + animation_interval(&runtime.app);
            runtime
                .render_scheduler
                .invalidate(RenderCause::Presentation);
        } else if !visible_animation {
            // Returning to an animated workspace shows activity immediately,
            // without accumulating hidden animation phases.
            runtime.next_animation_tick = presentation_now;
        }
        let live_elapsed = has_live_elapsed_time(&runtime.app);
        if live_elapsed && presentation_now >= runtime.next_elapsed_refresh {
            runtime.next_elapsed_refresh = presentation_now + ELAPSED_REFRESH_INTERVAL;
            runtime
                .render_scheduler
                .invalidate(RenderCause::Presentation);
        } else if !live_elapsed {
            runtime.next_elapsed_refresh = presentation_now;
        }
        if runtime.guard.take_full_redraw_request() {
            runtime.terminal.clear()?;
            runtime.render_scheduler.invalidate(RenderCause::Resize);
        }
        if let Some(effect) = runtime.app.pending_platform_writer_effect()
            && let Some(daemon_client) = runtime.daemon_runtime.as_mut()
        {
            let session_id = match &effect {
                yoctui_model::TerminalEffect::TakeControl { session_id, .. } => *session_id,
                _ => unreachable!("platform writer request is always take-control"),
            };
            if let Err(error) = daemon_client.route_effect(&runtime.app, &Effect::Terminal(effect))
            {
                runtime.app.retry_platform_writer_control(session_id);
                runtime.app.notification = Some(format!(
                    "Could not take menuconfig terminal control: {error}"
                ));
            }
        }
        if (runtime.app.screen == Screen::TerminalSessions
            || runtime.app.platform_menuconfig_visible())
            && runtime.app.selected_terminal_is_menuconfig()
            && runtime.app.terminal.mode == yoctui_model::TerminalWorkbenchMode::Live
            && let Some(daemon_client) = runtime.daemon_runtime.as_mut()
        {
            let size = runtime.terminal.size()?;
            if let Some(dimensions) =
                yoctui_app::terminal_workspace_dimensions(&runtime.app, size.width, size.height)
                && daemon_client.resize_selected_terminal(&runtime.app, dimensions)?
            {
                runtime.render_scheduler.invalidate(RenderCause::Resize);
            }
        }
        if runtime
            .render_scheduler
            .take_frame_with_interval(ordinary_frame_interval(&runtime.app))
        {
            runtime.terminal.draw(|f| render(f, &runtime.app))?;
        }
        let startup_platform_inspection = runtime.startup_platform_inspection.take();
        match startup_platform_inspection {
            Some(Screen::Kernel) => runtime.begin_platform_inspection(
                platform_inspection_operation::PlatformInspectionRequest::Kernel,
            ),
            Some(Screen::Firmware) => runtime.begin_platform_inspection(
                platform_inspection_operation::PlatformInspectionRequest::Firmware,
            ),
            _ => {}
        }
        if startup_platform_inspection.is_some() {
            runtime.render_scheduler.invalidate(RenderCause::State);
        }
        Ok(false)
    }
}
