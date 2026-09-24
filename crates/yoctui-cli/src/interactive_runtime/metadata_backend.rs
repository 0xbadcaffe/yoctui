use super::*;

const PROCESS_BACKEND_MESSAGE: &str =
    "Kernel and firmware inspection require --backend bridge and an attached Yoctui daemon.";

pub(crate) fn metadata_backend_start_required(
    backend_kind: &Backend,
    authoritative: bool,
) -> Result<bool, &'static str> {
    if authoritative {
        return Ok(false);
    }
    match backend_kind {
        Backend::Bridge => Ok(true),
        Backend::Process => Err(PROCESS_BACKEND_MESSAGE),
    }
}

async fn ensure_authoritative_metadata_backend(
    backend_kind: &Backend,
    build_dir: &Path,
    cancellation_timeout: Duration,
    backend: &mut Box<dyn BitBakeBackend>,
    authoritative: &mut bool,
) -> Result<(), String> {
    let required =
        metadata_backend_start_required(backend_kind, *authoritative).map_err(str::to_owned)?;
    if !required {
        return Ok(());
    }

    let replacement = select_backend_with_timeout(
        Backend::Bridge,
        build_dir.to_path_buf(),
        Some(cancellation_timeout),
    )
    .await
    .map_err(|error| format!("{error:#}"))?;
    let mut placeholder = std::mem::replace(backend, replacement);
    if let Err(error) = placeholder.shutdown().await {
        tracing::debug!(%error, "placeholder metadata backend shutdown failed");
    }
    *authoritative = true;
    Ok(())
}

pub(super) async fn inspect_kernel(
    app: &mut App,
    backend_kind: &Backend,
    build_dir: &Path,
    cancellation_timeout: Duration,
    backend: &mut Box<dyn BitBakeBackend>,
    authoritative: &mut bool,
) {
    if let Err(message) = ensure_authoritative_metadata_backend(
        backend_kind,
        build_dir,
        cancellation_timeout,
        backend,
        authoritative,
    )
    .await
    {
        let _ = compatibility_workspace_action(app, Action::KernelFailed(message));
        return;
    }
    inspect_kernel_workbench(app, backend.as_mut()).await;
}

pub(super) async fn inspect_firmware(
    app: &mut App,
    backend_kind: &Backend,
    build_dir: &Path,
    cancellation_timeout: Duration,
    backend: &mut Box<dyn BitBakeBackend>,
    authoritative: &mut bool,
) {
    if let Err(message) = ensure_authoritative_metadata_backend(
        backend_kind,
        build_dir,
        cancellation_timeout,
        backend,
        authoritative,
    )
    .await
    {
        let _ = compatibility_workspace_action(app, Action::FirmwareFailed(message));
        return;
    }
    inspect_firmware_workbench(app, backend.as_mut()).await;
}

impl InteractiveRuntime {
    pub(super) async fn inspect_kernel(&mut self) {
        inspect_kernel(
            &mut self.app,
            &self.backend_kind,
            &self.session_build_dir,
            self.cancellation_timeout,
            &mut self.backend,
            &mut self.metadata_backend_authoritative,
        )
        .await;
    }

    pub(super) async fn inspect_firmware(&mut self) {
        inspect_firmware(
            &mut self.app,
            &self.backend_kind,
            &self.session_build_dir,
            self.cancellation_timeout,
            &mut self.backend,
            &mut self.metadata_backend_authoritative,
        )
        .await;
    }
}
