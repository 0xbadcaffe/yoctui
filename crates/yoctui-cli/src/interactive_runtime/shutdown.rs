use super::*;

impl InteractiveRuntime {
    pub(super) async fn shutdown(self) -> Result<()> {
        let mut runtime = self;
        let render_metrics = runtime.render_scheduler.metrics();
        if let Some(path) = std::env::var_os("YOCTUI_PERFORMANCE_METRICS_PATH") {
            let elapsed_seconds = runtime.render_measurement_started.elapsed().as_secs_f64();
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
        if let Some(daemon_client) = runtime.daemon_runtime.take()
            && let Err(error) = daemon_client.detach(&mut runtime.app)
        {
            tracing::warn!(%error, "daemon detach failed during client shutdown");
        }
        if let Some(operation) = runtime.signature_operation.take() {
            operation.cancellation.cancel();
            let _ = operation.handle.await;
        }
        runtime.stop_recipe_inspection().await;
        runtime.stop_devtool_status().await;
        runtime.stop_platform_inspection().await;
        if let Some(operation) = runtime.image_artifact_operation.take() {
            operation.cancellation.cancel();
            let _ = operation.handle.await;
        }
        if let Some(operation) = runtime.sdk_artifact_operation.take() {
            operation.cancellation.cancel();
            let _ = operation.handle.await;
        }
        if let Some(operation) = runtime.sdk_capability_operation.take() {
            operation.handle.abort();
            let _ = operation.handle.await;
        }
        if let Some(mut operation) = runtime.sdk_operation.take() {
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
        if let Some(mut operation) = runtime.qemu_operation.take() {
            if let Some(handle) = operation.cancellation.take() {
                handle.abort();
                let _ = handle.await;
            } else if let Some(mut runner) = operation.runner.take() {
                let _ = runner.cancel().await;
            }
        }
        if let Some(operation) = runtime.wic_capability_operation.take() {
            operation.handle.abort();
            let _ = operation.handle.await;
        }
        if let Some(mut operation) = runtime.wic_operation.take() {
            if let Some(handle) = operation.cancellation.take() {
                handle.abort();
                let _ = handle.await;
            } else if let Some(mut runner) = operation.runner.take() {
                let _ = runner.cancel().await;
            }
        }
        if let Some(mut operation) = runtime.test_coordinator.session.take() {
            let _ = operation.runner.cancel().await;
        }
        if let Some(mut operation) = runtime.test_coordinator.result.take() {
            let _ = operation.runner.cancel().await;
        }
        if let Some(operation) = runtime.test_coordinator.import.take() {
            operation.handle.abort();
            let _ = operation.handle.await;
        }
        if let Some(operation) = runtime.security_coordinator.capability.take() {
            operation.handle.abort();
            let _ = operation.handle.await;
        }
        if let Some(operation) = runtime.security_coordinator.report.take() {
            operation.cancellation.cancel();
            operation.handle.abort();
            let _ = operation.handle.await;
        }
        if let Some(mut operation) = runtime.security_coordinator.mapper.take() {
            let _ = operation.runner.cancel(operation.id).await;
        }
        runtime.maintenance_coordinator.shutdown().await;
        runtime.backend.shutdown().await?;
        let raw_favorites = runtime.app.raw_mode.favorites.clone();
        let mut preferences = runtime.app.effective_preferences();
        if runtime.color_forced_off {
            preferences.color_enabled = runtime.app.preferences.color_enabled;
        }
        runtime.session.last_target = runtime.app.build.target;
        runtime.session.last_screen = Some(runtime.app.screen);
        runtime.session.log_filter = runtime.app.logs.filter;
        runtime.session.log_recipe_filter = runtime.app.logs.recipe_filter;
        runtime.session.log_task_filter = runtime.app.logs.task_filter;
        runtime.session.log_build_filter = runtime.app.logs.build_filter;
        runtime.session.pane_layout = preferences
            .remember_pane_sizes
            .then(|| runtime.app.pane_layout.clone());
        runtime.session.preferences = Some(preferences);
        runtime.session.log_wrap = None;
        runtime.session.log_follow = None;
        runtime.session.theme = None;
        runtime.session.animation_speed = None;
        runtime.session.reduced_motion = None;
        runtime.session.color_enabled = None;
        runtime.session.keymap = yoctui_model::KeymapPreferences::default();
        runtime.session.onboarding = Some(runtime.app.onboarding.progress.clone());
        runtime
            .session
            .hardware_documents
            .clone_from(&runtime.app.hardware.documents);
        runtime.session.hardware_last_directory = runtime.app.hardware.last_directory.clone();
        runtime.session.recent_build_dirs = std::iter::once(runtime.session_build_dir)
            .chain(runtime.session.recent_build_dirs)
            .fold(Vec::new(), |mut directories, directory| {
                if !directories.contains(&directory) && directories.len() < 10 {
                    directories.push(directory);
                }
                directories
            });
        persist_raw_favorites(
            runtime.session_path.as_deref(),
            &mut runtime.session,
            &raw_favorites,
        )?;
        Ok(())
    }
}
