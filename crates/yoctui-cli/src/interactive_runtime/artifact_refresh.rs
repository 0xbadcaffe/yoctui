use super::*;

impl InteractiveRuntime {
    pub(super) fn poll_image_artifact_refresh(&mut self) {
        // Reconfigure the adapter from current metadata, never a startup-only path.
        if self.image_artifact_operation.is_none() {
            self.image_artifact_adapter = self
                .app
                .workspace
                .variables
                .get("DEPLOY_DIR_IMAGE")
                .map(PathBuf::from)
                .map(ImageArtifactAdapter::new);
        }
        if let Some(effect) = self.image_artifact_refresh.poll(
            &mut self.app,
            self.image_artifact_operation.is_some(),
            Instant::now(),
        ) {
            begin_image_artifact_operation(
                &mut self.app,
                self.image_artifact_adapter.as_ref(),
                &mut self.image_artifact_operation,
                effect,
            );
            self.render_scheduler.invalidate(RenderCause::State);
        }
    }
}
