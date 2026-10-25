use super::*;

impl InteractiveRuntime {
    pub(super) async fn poll_local_documents(&mut self) -> Result<()> {
        self.poll_workbench().await;
        self.render_scheduler.invalidate_if(
            self.layer_browser_io.poll(&mut self.app),
            RenderCause::State,
        );
        let editor_changed = self.poll_preferred_editor().await;
        self.render_scheduler
            .invalidate_if(editor_changed, RenderCause::State);
        if matches!(
            self.app.screen,
            Screen::DiffTool | Screen::Help | Screen::BuildPerformance | Screen::Maintenance
        ) {
            let size = self.terminal.size()?;
            self.app.sstate_cache.review_width =
                usize::from(size.width.saturating_sub(2).min(100).saturating_sub(2));
            self.app.sstate_cache.review_visible_rows =
                usize::from(size.height.saturating_sub(2).min(20).saturating_sub(4));
            let extra = u16::from(self.app.offline_notice().is_some()) * 2
                + u16::from(self.app.zoomed_pane.is_some() || size.width < 100);
            let height = size.height.saturating_sub(8 + extra);
            if self.app.screen == Screen::DiffTool {
                self.app
                    .file_diff
                    .reconcile_viewport(usize::from(height.saturating_sub(8)));
            } else if self.app.screen == Screen::Maintenance {
                self.app.sstate_cache.visible_rows = usize::from(height.saturating_sub(7)).max(1);
                let maximum = self
                    .app
                    .sstate_cache_rows()
                    .len()
                    .saturating_sub(self.app.sstate_cache.visible_rows);
                self.app.sstate_cache.scroll = self.app.sstate_cache.scroll.min(maximum);
            } else if self.app.screen == Screen::BuildPerformance {
                self.app.build_performance.visible_rows =
                    usize::from(height.saturating_sub(7)).max(1);
                let maximum = self
                    .app
                    .build_performance_rows()
                    .len()
                    .saturating_sub(self.app.build_performance.visible_rows);
                self.app.build_performance.scroll = self.app.build_performance.scroll.min(maximum);
            } else {
                self.app.help_visible_rows = usize::from(height.saturating_sub(2));
                let _ = update(&mut self.app, Action::ScrollHelp { delta: 0 });
            }
        }
        if self.app.screen == Screen::DiffTool {
            let root = self
                .app
                .source_repository_path()
                .map(Path::to_owned)
                .unwrap_or_else(|| self.session_build_dir.clone());
            if let Some(effect) = yoctui_model::file_diff_initial_request(&mut self.app, root) {
                self.file_diff_io.submit(effect);
            }
        }
        self.render_scheduler.invalidate_if(
            self.file_diff_io.poll(&mut self.app).await,
            RenderCause::State,
        );
        Ok(())
    }
}
