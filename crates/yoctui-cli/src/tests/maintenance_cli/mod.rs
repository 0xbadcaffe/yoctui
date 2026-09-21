use super::*;
use std::sync::atomic::{AtomicU64, Ordering};
use yoctui_model::{
    MAX_MAINTENANCE_OUTPUT, MaintenanceCapability, MaintenanceSessionStatus, SstateReadinessMode,
    SstateReadinessRequest,
};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(1);

struct Fixture {
    root: PathBuf,
    build: PathBuf,
    bin: PathBuf,
}

impl Fixture {
    fn new(script: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "yoctui-maintenance-workflow-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
        ));
        let build = root.join("build");
        let bin = root.join("bin");
        fs::create_dir_all(&build).unwrap();
        fs::create_dir_all(&bin).unwrap();
        let executable = bin.join("oe-check-sstate");
        Self::publish_executable(&executable, script);
        Self { root, build, bin }
    }

    fn publish_executable(executable: &Path, script: &str) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let temporary = executable.with_extension(format!(
                "fixture-write-{}",
                NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::write(&temporary, script).unwrap();
            fs::set_permissions(&temporary, fs::Permissions::from_mode(0o755)).unwrap();
            fs::rename(temporary, executable).unwrap();
        }
        #[cfg(not(unix))]
        fs::write(executable, script).unwrap();
    }

    fn install(&self, name: &str, script: &str) -> PathBuf {
        let executable = self.bin.join(name);
        Self::publish_executable(&executable, script);
        executable
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

async fn poll_until(
    coordinator: &mut MaintenanceCliCoordinator,
    app: &mut App,
    complete: impl Fn(&App, &MaintenanceCliCoordinator) -> bool,
) {
    let result = tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            coordinator.poll(app).await;
            if complete(app, coordinator) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await;
    if result.is_err() {
        panic!(
            "Maintenance CLI workflow did not complete: notification={:?}, inspection_active={}, cleanup_preview_active={}, operation_active={}",
            app.notification,
            coordinator.inspection.is_some(),
            coordinator.cleanup_preview.is_some(),
            coordinator.operation.is_some()
        );
    }
}

async fn refreshed_coordinator(fixture: &Fixture) -> (App, MaintenanceCliCoordinator, u64) {
    let mut app = App::new(100, 64 * 1024);
    app.workspace.build_dir = Some(fixture.build.clone());
    let mut coordinator =
        MaintenanceCliCoordinator::new(&app, &fixture.build, vec![fixture.bin.clone()]).unwrap();
    let effect = update(
        &mut app,
        Action::Maintenance(MaintenanceAction::InspectCapability),
    )
    .unwrap();
    assert!(coordinator.handle_effect(&mut app, effect).await);
    poll_until(&mut coordinator, &mut app, |app, _| {
        matches!(
            app.maintenance.capability,
            MaintenanceCapability::Available { .. } | MaintenanceCapability::Partial { .. }
        )
    })
    .await;
    let request = app.maintenance.capability.request().unwrap();
    (app, coordinator, request)
}

async fn refreshed_cleanup_coordinator(
    fixture: &Fixture,
    script: &str,
) -> (App, MaintenanceCliCoordinator, u64, PathBuf, PathBuf) {
    fixture.install("sstate-cache-management.py", script);
    let cache = fixture.root.join("sstate-cache");
    let stamps = fixture.root.join("stamps");
    fs::create_dir_all(&cache).unwrap();
    fs::create_dir_all(&stamps).unwrap();
    let mut app = App::new(100, 64 * 1024);
    app.workspace
        .variables
        .insert("SSTATE_DIR".into(), cache.display().to_string());
    app.workspace
        .variables
        .insert("STAMPS_DIR".into(), stamps.display().to_string());
    let mut coordinator =
        MaintenanceCliCoordinator::new(&app, &fixture.build, vec![fixture.bin.clone()]).unwrap();
    let effect = update(
        &mut app,
        Action::Maintenance(MaintenanceAction::InspectCapability),
    )
    .unwrap();
    coordinator.handle_effect(&mut app, effect).await;
    poll_until(&mut coordinator, &mut app, |app, _| {
        app.maintenance
            .capability
            .snapshot()
            .is_some_and(|snapshot| snapshot.supports(MaintenanceTool::SstateCacheManagement))
    })
    .await;
    let request = app.maintenance.capability.request().unwrap();
    (app, coordinator, request, cache, stamps)
}

async fn refreshed_service_coordinator(
    fixture: &Fixture,
    script: &str,
) -> (App, MaintenanceCliCoordinator, u64) {
    fixture.install("bitbake-prserv-tool", script);
    let mut app = App::new(100, 64 * 1024);
    app.workspace
        .variables
        .insert("PRSERV_HOST".into(), "localhost:8585".into());
    let mut coordinator =
        MaintenanceCliCoordinator::new(&app, &fixture.build, vec![fixture.bin.clone()]).unwrap();
    let effect = update(
        &mut app,
        Action::Maintenance(MaintenanceAction::InspectCapability),
    )
    .unwrap();
    coordinator.handle_effect(&mut app, effect).await;
    poll_until(&mut coordinator, &mut app, |app, _| {
        app.maintenance
            .capability
            .snapshot()
            .is_some_and(|snapshot| snapshot.supports(MaintenanceTool::PrServiceTool))
    })
    .await;
    let request = app.maintenance.capability.request().unwrap();
    (app, coordinator, request)
}

async fn refreshed_release_coordinator(
    fixture: &Fixture,
    locked_script: &str,
    buildhistory_script: &str,
    archive_script: &str,
) -> (App, MaintenanceCliCoordinator, u64, PathBuf) {
    fixture.install("gen-lockedsig-cache", locked_script);
    fixture.install("buildhistory-diff", buildhistory_script);
    fixture.install("oe-git-archive", archive_script);
    let history = fixture.root.join("buildhistory");
    fs::create_dir_all(history.join(".git")).unwrap();
    fs::write(history.join(".git/HEAD"), b"ref: refs/heads/main\n").unwrap();
    let mut app = App::new(100, 64 * 1024);
    app.workspace
        .variables
        .insert("NATIVELSBSTRING".into(), "ubuntu".into());
    app.workspace
        .variables
        .insert("BUILDHISTORY_DIR".into(), history.display().to_string());
    let mut coordinator =
        MaintenanceCliCoordinator::new(&app, &fixture.build, vec![fixture.bin.clone()]).unwrap();
    let effect = update(
        &mut app,
        Action::Maintenance(MaintenanceAction::InspectCapability),
    )
    .unwrap();
    coordinator.handle_effect(&mut app, effect).await;
    poll_until(&mut coordinator, &mut app, |app, _| {
        app.maintenance
            .capability
            .snapshot()
            .is_some_and(|snapshot| {
                [
                    MaintenanceTool::LockedSignatureCache,
                    MaintenanceTool::BuildHistoryDiff,
                    MaintenanceTool::GitArchive,
                ]
                .into_iter()
                .all(|tool| snapshot.supports(tool))
            })
    })
    .await;
    let request = app.maintenance.capability.request().unwrap();
    (app, coordinator, request, history)
}

async fn begin_readiness(
    app: &mut App,
    coordinator: &mut MaintenanceCliCoordinator,
    request: u64,
    id: MaintenanceSessionId,
    timeout_seconds: u64,
) {
    let snapshot = app.maintenance.capability.snapshot().unwrap().clone();
    let readiness = SstateReadinessRequest::new(
        vec!["core-image-minimal".into()],
        SstateReadinessMode::IsolatedTmpdir,
        None,
        None,
        timeout_seconds,
    )
    .unwrap();
    let (preview, _) =
        MaintenanceSstateCommandSpec::readiness(id, request, &snapshot, id.0, readiness).unwrap();
    let _ = update(
        app,
        Action::Maintenance(MaintenanceAction::BeginOperation(preview.clone())),
    );
    let effect = update(
        app,
        Action::Maintenance(MaintenanceAction::ConfirmOperation(preview)),
    )
    .unwrap();
    assert!(coordinator.handle_effect(app, effect).await);
}

mod maintenance_workflow_refresh_is_correlated_and_preserves_unavailable_tools;

mod maintenance_workflow_runner_maps_success_and_bounded_output;

mod maintenance_workflow_rejects_cancellation_for_an_unowned_session;

mod maintenance_workflow_runner_maps_nonzero_and_timeout_distinctly;

mod maintenance_workflow_cancels_only_the_exact_active_session;

mod maintenance_sstate_workspace_builds_exact_readiness_confirmation;

mod maintenance_sstate_workspace_discovers_exact_cleanup_candidates_before_phrase;

mod maintenance_sstate_workspace_preview_failure_never_opens_destructive_dialog;

mod maintenance_service_workspace_builds_distinct_exact_previews;

mod maintenance_service_workspace_success_installs_exact_export_evidence;

mod maintenance_service_workspace_rejects_stale_and_invalid_previews;

mod maintenance_service_workspace_reports_nonzero_without_export_evidence;

mod maintenance_release_workspace_builds_exact_previews_for_every_form;

mod maintenance_release_workspace_defers_push_until_local_head_success;

mod maintenance_release_workspace_installs_changed_locked_cache_evidence;

mod maintenance_release_workspace_bounds_comparison_output_and_reports_nonzero;

mod maintenance_release_workspace_rejects_stale_and_invalid_requests;
