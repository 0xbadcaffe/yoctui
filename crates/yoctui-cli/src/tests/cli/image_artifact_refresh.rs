use super::*;

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "yoctui-artifact-refresh-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("qemux86-64")).unwrap();
        Self(root)
    }
    fn deploy(&self) -> PathBuf {
        self.0.join("qemux86-64")
    }
    fn app(&self) -> App {
        let mut app = App::new(10, 1000);
        app.screen = Screen::Images;
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        app.workspace.variables.insert(
            "DEPLOY_DIR_IMAGE".into(),
            self.deploy().display().to_string(),
        );
        app
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

async fn scan(app: &mut App, effect: Effect) {
    let Effect::GetImageArtifacts(request) = effect else {
        panic!("expected artifact scan")
    };
    let response =
        ImageArtifactAdapter::new(PathBuf::from(&app.workspace.variables["DEPLOY_DIR_IMAGE"]))
            .scan(request)
            .await
            .unwrap();
    let event: BackendEvent = response.into();
    update(app, model_action_from_backend_event(event).unwrap());
}

async fn initial(io: &mut ImageArtifactRefresh, app: &mut App) {
    let now = Instant::now();
    assert!(io.poll(app, false, now).is_none());
    let effect = io.poll(app, false, now + Duration::from_secs(1)).unwrap();
    scan(app, effect).await;
}

async fn wait_refresh(io: &mut ImageArtifactRefresh, app: &mut App) {
    tokio::time::timeout(Duration::from_secs(4), async {
        loop {
            if let Some(effect) = io.poll(app, false, Instant::now()) {
                scan(app, effect).await;
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("deploy mutation must refresh without a keypress");
}

#[tokio::test]
async fn artifact_auto_refresh_watches_create_modify_remove_and_preserves_selection() {
    let fixture = Fixture::new();
    let first = fixture
        .deploy()
        .join("core-image-minimal-qemux86-64.rootfs.ext4");
    fs::write(&first, b"old").unwrap();
    let mut app = fixture.app();
    let mut io = ImageArtifactRefresh::default();
    initial(&mut io, &mut app).await;
    let selected = app.image_artifact_selection.clone();
    let second = fixture
        .deploy()
        .join("core-image-minimal-qemux86-64.rootfs-20261015011335.ext4.zst");
    fs::write(&second, b"new image").unwrap();
    wait_refresh(&mut io, &mut app).await;
    assert_eq!(app.image_artifacts.artifacts().unwrap().len(), 2);
    assert_eq!(app.image_artifact_selection, selected);
    fs::write(&first, b"changed-size").unwrap();
    let changed_time = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_123);
    fs::File::options()
        .write(true)
        .open(&first)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(changed_time))
        .unwrap();
    wait_refresh(&mut io, &mut app).await;
    let artifact = app.selected_image_artifact().unwrap();
    assert_eq!(artifact.size_bytes.available(), Some(&12));
    assert_eq!(
        artifact.modified_unix_seconds.available(),
        Some(&1_700_000_123)
    );
    fs::remove_file(&first).unwrap();
    wait_refresh(&mut io, &mut app).await;
    assert_eq!(app.image_artifacts.artifacts().unwrap().len(), 1);
    assert_eq!(app.selected_image_artifact().unwrap().identity.path, second);
    // Reading the inventory ourselves must not cause an endless watch/scan loop.
    tokio::time::sleep(Duration::from_millis(600)).await;
    assert!(io.poll(&mut app, false, Instant::now()).is_none());
}

#[tokio::test]
async fn artifact_auto_refresh_waits_for_build_and_active_scan_and_reenters_screen() {
    let fixture = Fixture::new();
    let mut app = fixture.app();
    let mut io = ImageArtifactRefresh::default();
    initial(&mut io, &mut app).await;
    let now = Instant::now();
    app.build.status = yoctui_model::BuildStatus::Running;
    assert!(io.poll(&mut app, false, now).is_none());
    fs::write(fixture.deploy().join("new.ext4"), b"image").unwrap();
    update(
        &mut app,
        Action::BuildCompleted {
            success: true,
            exit_code: Some(0),
        },
    );
    assert!(io.poll(&mut app, false, now).is_none());
    assert!(
        io.poll(&mut app, false, now + Duration::from_secs(1))
            .is_none()
    );
    update(&mut app, Action::DismissBuildCompletion);
    // A scan already in flight must not be cancelled/replaced or lose a dirty event.
    assert!(
        io.poll(&mut app, true, now + Duration::from_secs(1))
            .is_none()
    );
    tokio::time::sleep(Duration::from_millis(40)).await;
    let later = now + Duration::from_secs(2);
    let effect = io
        .poll(&mut app, false, later)
        .or_else(|| io.poll(&mut app, false, later + Duration::from_secs(1)))
        .unwrap();
    scan(&mut app, effect).await;
    assert_eq!(app.image_artifacts.artifacts().unwrap().len(), 1);
    app.screen = Screen::Tasks;
    assert!(io.poll(&mut app, false, later).is_none());
    app.screen = Screen::Images;
    assert!(matches!(
        io.poll(&mut app, false, later),
        Some(Effect::GetImageArtifacts(_))
    ));
}

#[tokio::test]
async fn artifact_auto_refresh_rearms_replaced_directory_and_changes_context_safely() {
    let fixture = Fixture::new();
    let mut app = fixture.app();
    let mut io = ImageArtifactRefresh::default();
    initial(&mut io, &mut app).await;
    fs::remove_dir(fixture.deploy()).unwrap();
    fs::create_dir(fixture.deploy()).unwrap();
    fs::write(fixture.deploy().join("new.ext4"), b"new").unwrap();
    wait_refresh(&mut io, &mut app).await;
    assert_eq!(app.image_artifacts.artifacts().unwrap().len(), 1);
    let other = fixture.0.join("qemuarm64");
    fs::create_dir(&other).unwrap();
    fs::write(other.join("other.ext4"), b"other").unwrap();
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemuarm64".into());
    app.workspace
        .variables
        .insert("DEPLOY_DIR_IMAGE".into(), other.display().to_string());
    initial(&mut io, &mut app).await;
    assert_eq!(
        app.image_artifacts.inventory().unwrap().machine,
        "qemuarm64"
    );
    assert!(
        app.image_artifacts
            .artifacts()
            .unwrap()
            .iter()
            .all(|a| a.identity.path.starts_with(&other))
    );
    app.require_daemon = true; // A disconnected client must not inspect stale local metadata.
    assert!(
        io.poll(&mut app, false, Instant::now() + Duration::from_secs(30))
            .is_none()
    );
}

#[tokio::test]
async fn artifact_auto_refresh_discovers_a_new_deploy_directory_after_first_build() {
    let fixture = Fixture::new();
    fs::remove_dir(fixture.deploy()).unwrap();
    let mut app = fixture.app();
    let mut io = ImageArtifactRefresh::default();
    app.build.status = yoctui_model::BuildStatus::Running;
    assert!(io.poll(&mut app, false, Instant::now()).is_none());
    fs::create_dir(fixture.deploy()).unwrap();
    fs::write(fixture.deploy().join("first.ext4"), b"first").unwrap();
    app.build.status = yoctui_model::BuildStatus::Completed;
    wait_refresh(&mut io, &mut app).await;
    assert_eq!(app.image_artifacts.artifacts().unwrap().len(), 1);
}
