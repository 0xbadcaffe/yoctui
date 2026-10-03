use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use yoctui_model::{
    FocusTarget, TerminalCreationKind, TerminalLaunchDestination, TerminalLaunchDialog,
    TerminalLaunchRequest,
};

#[derive(Default)]
struct Counts {
    opened: AtomicUsize,
    shutdown: AtomicUsize,
    dropped: AtomicUsize,
}

#[derive(Clone, Copy)]
enum Cleanup {
    Success,
    Failure,
    Pending,
}

struct FakeBackend {
    counts: Arc<Counts>,
    cleanup: Cleanup,
    child: Option<tokio::process::Child>,
}

impl Drop for FakeBackend {
    fn drop(&mut self) {
        self.counts.dropped.fetch_add(1, Ordering::SeqCst);
    }
}

#[async_trait::async_trait]
impl BitBakeBackend for FakeBackend {
    async fn inspect_workspace(&mut self) -> Result<Workspace, BackendError> {
        Ok(Workspace::default())
    }
    async fn list_recipes(&mut self, _filter: Option<String>) -> Result<Vec<Recipe>, BackendError> {
        Ok(vec![])
    }
    async fn list_layers(&mut self) -> Result<Vec<Layer>, BackendError> {
        Ok(vec![])
    }
    async fn get_variable(
        &mut self,
        name: String,
        recipe: Option<String>,
    ) -> Result<VariableValue, BackendError> {
        Ok(VariableValue {
            value: Some(format!("{name}:{}", recipe.unwrap_or_default())),
            ..VariableValue::default()
        })
    }
    async fn get_dependencies(
        &mut self,
        _recipe: String,
    ) -> Result<RecipeDependencies, BackendError> {
        Err(BackendError::NotRunning)
    }
    async fn get_dependency_graph(
        &mut self,
        _recipe: String,
    ) -> Result<DependencyGraphResponse, BackendError> {
        Err(BackendError::NotRunning)
    }
    async fn get_signature_dump(
        &mut self,
        _target: SignatureTarget,
    ) -> Result<SignatureDumpResponse, BackendError> {
        Err(BackendError::NotRunning)
    }
    async fn compare_signatures(
        &mut self,
        _request: SignatureComparisonRequest,
    ) -> Result<SignatureComparisonResponse, BackendError> {
        Err(BackendError::NotRunning)
    }
    async fn get_recipe_sources(&mut self, _recipe: String) -> Result<Vec<PathBuf>, BackendError> {
        Ok(vec!["/selected/source/recipe.bb".into()])
    }
    async fn get_recipe_metadata(
        &mut self,
        recipe: String,
    ) -> Result<RecipeMetadata, BackendError> {
        Ok(RecipeMetadata {
            recipe,
            ..RecipeMetadata::default()
        })
    }
    async fn get_layer_relationships(&mut self) -> Result<Vec<LayerRelationship>, BackendError> {
        Ok(vec![])
    }
    async fn start_build(&mut self, _request: BuildRequest) -> Result<(), BackendError> {
        panic!("native inspection cannot start a client build")
    }
    async fn cancel_build(&mut self) -> Result<(), BackendError> {
        panic!("native inspection cannot cancel a daemon build")
    }
    async fn next_event(&mut self) -> Result<BackendEvent, BackendError> {
        panic!("native inspection cannot own daemon events")
    }
    async fn shutdown(&mut self) -> Result<(), BackendError> {
        self.counts.shutdown.fetch_add(1, Ordering::SeqCst);
        match self.cleanup {
            Cleanup::Failure => Err(BackendError::Bridge("shutdown rejected".into())),
            Cleanup::Pending => std::future::pending().await,
            Cleanup::Success => {
                if let Some(child) = &mut self.child {
                    use tokio::io::AsyncWriteExt;
                    child.stdin.take().unwrap().write_all(b"finish\n").await?;
                    assert!(child.wait().await?.success());
                }
                Ok(())
            }
        }
    }
}

fn scope(cleanup: Cleanup) -> (NativeMetadataScope, Arc<Counts>) {
    let counts = Arc::new(Counts::default());
    let factory_counts = counts.clone();
    (
        NativeMetadataScope {
            factory: Box::new(move || {
                let counts = factory_counts.clone();
                Box::pin(async move {
                    counts.opened.fetch_add(1, Ordering::SeqCst);
                    Ok(Box::new(FakeBackend {
                        counts,
                        cleanup,
                        child: None,
                    }) as Box<dyn BitBakeBackend>)
                })
            }),
            shutdown_timeout: Duration::from_millis(20),
        },
        counts,
    )
}

fn count(value: &AtomicUsize) -> usize {
    value.load(Ordering::SeqCst)
}

#[tokio::test]
async fn inspection_lease_queries_release_before_return_and_can_repeat() {
    let (mut backend, counts) = scope(Cleanup::Success);
    assert_eq!(count(&counts.opened), 0);
    for name in ["MACHINE", "DISTRO"] {
        let value = backend
            .get_variable(name.into(), Some("busybox".into()))
            .await
            .unwrap();
        assert_eq!(
            value.value.as_deref(),
            Some(format!("{name}:busybox").as_str())
        );
        assert_eq!(count(&counts.opened), count(&counts.shutdown));
        assert_eq!(count(&counts.opened), count(&counts.dropped));
    }
    assert_eq!(
        backend
            .get_recipe_metadata("busybox".into())
            .await
            .unwrap()
            .recipe,
        "busybox"
    );
    assert!(backend.get_layer_relationships().await.unwrap().is_empty());
    assert_eq!(
        backend.get_recipe_sources("busybox".into()).await.unwrap(),
        vec![PathBuf::from("/selected/source/recipe.bb")]
    );
    assert!(
        backend
            .inspect_workspace()
            .await
            .unwrap()
            .recipes
            .is_empty()
    );
    assert!(backend.list_recipes(None).await.unwrap().is_empty());
    assert!(backend.list_layers().await.unwrap().is_empty());
    assert_eq!(count(&counts.opened), 8);
    assert_eq!(count(&counts.shutdown), 8);
    assert_eq!(count(&counts.dropped), 8);
}

#[tokio::test]
async fn inspection_lease_failure_still_releases_and_cleanup_errors_are_explicit() {
    let (mut backend, counts) = scope(Cleanup::Success);
    assert!(matches!(
        backend.get_dependencies("missing".into()).await,
        Err(BackendError::NotRunning)
    ));
    assert!(matches!(
        backend.get_dependency_graph("missing".into()).await,
        Err(BackendError::NotRunning)
    ));
    assert_eq!(count(&counts.shutdown), 2);
    assert_eq!(count(&counts.dropped), 2);
    for cleanup in [Cleanup::Failure, Cleanup::Pending] {
        let (mut backend, counts) = scope(cleanup);
        let error = backend
            .get_variable("MACHINE".into(), None)
            .await
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("cannot release native metadata connection")
        );
        assert_eq!(count(&counts.shutdown), 1);
        assert_eq!(count(&counts.dropped), 1);
        let error = backend
            .get_dependencies("missing".into())
            .await
            .unwrap_err();
        assert!(error.to_string().contains("inspection also failed"));
        assert_eq!(count(&counts.dropped), 2);
    }
}

#[tokio::test]
async fn inspection_lease_idle_and_native_build_control_never_spawn_a_bridge() {
    let (mut backend, counts) = scope(Cleanup::Success);
    assert!(
        backend
            .start_build(BuildRequest {
                targets: vec!["image".into()],
                task: None,
                force: false
            })
            .await
            .is_err()
    );
    assert!(backend.cancel_build().await.is_err());
    assert!(backend.next_event().await.is_err());
    backend.shutdown().await.unwrap();
    assert_eq!(count(&counts.opened), 0);
    assert_eq!(count(&counts.shutdown), 0);
}

#[tokio::test]
async fn inspection_lease_failed_factory_is_not_cached_as_ready() {
    let attempts = Arc::new(AtomicUsize::new(0));
    let factory_attempts = attempts.clone();
    let mut backend = NativeMetadataScope {
        factory: Box::new(move || {
            factory_attempts.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { Err(BackendError::Bridge("authority unavailable".into())) })
        }),
        shutdown_timeout: SHUTDOWN_TIMEOUT,
    };
    for _ in 0..2 {
        assert!(
            backend
                .get_variable("MACHINE".into(), None)
                .await
                .unwrap_err()
                .to_string()
                .contains("authority unavailable")
        );
    }
    assert_eq!(count(&attempts), 2);
}

#[tokio::test]
async fn inspection_lease_cleanup_reaps_only_the_owned_fake_process() {
    for cleanup in [Cleanup::Success, Cleanup::Failure, Cleanup::Pending] {
        let child = tokio::process::Command::new("/bin/sh")
            .args(["-c", "read -r line"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let pid = child.id().unwrap();
        let counts = Arc::new(Counts::default());
        let result = finish_query(
            Box::new(FakeBackend {
                counts: counts.clone(),
                cleanup,
                child: Some(child),
            }),
            Ok(42),
            Duration::from_millis(30),
        )
        .await;
        assert_eq!(result.is_ok(), matches!(cleanup, Cleanup::Success));
        tokio::time::timeout(Duration::from_secs(2), async {
            while Path::new(&format!("/proc/{pid}")).exists() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("owned fake metadata child was not reaped");
        assert_eq!(count(&counts.shutdown), 1);
        assert_eq!(count(&counts.dropped), 1);
    }
}

#[tokio::test]
async fn inspection_lease_cancelled_cleanup_drops_owned_child_without_a_late_result() {
    let child = tokio::process::Command::new("/bin/sh")
        .args(["-c", "read -r line"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let pid = child.id().unwrap();
    let counts = Arc::new(Counts::default());
    let worker_counts = counts.clone();
    let worker = tokio::spawn(async move {
        finish_query(
            Box::new(FakeBackend {
                counts: worker_counts,
                cleanup: Cleanup::Pending,
                child: Some(child),
            }),
            Ok(42),
            Duration::from_secs(60),
        )
        .await
    });
    tokio::task::yield_now().await;
    assert!(!worker.is_finished());
    worker.abort();
    assert!(worker.await.unwrap_err().is_cancelled());
    tokio::time::timeout(Duration::from_secs(2), async {
        while Path::new(&format!("/proc/{pid}")).exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("cancelled fake metadata child was not reaped");
    assert_eq!(count(&counts.shutdown), 1);
    assert_eq!(count(&counts.dropped), 1);
}

fn chooser(kind: TerminalCreationKind, destination: TerminalLaunchDestination) -> App {
    let mut app = App::new(10, 1000);
    app.screen = Screen::Recipes;
    app.focus = FocusTarget::Dialog;
    app.dialogs
        .push_back(Dialog::TerminalLaunch(TerminalLaunchDialog {
            request: TerminalLaunchRequest {
                name: "busybox".into(),
                kind,
                cwd: "/selected/build".into(),
                program: "/usr/bin/env".into(),
                arguments: vec!["devtool".into(), "edit-recipe".into(), "busybox".into()],
                completion: None,
            },
            destination,
            output_must_not_exist: None,
        }));
    app
}

#[test]
fn inspection_lease_pending_confirmation_preserves_chooser_and_cancel() {
    for destination in [
        TerminalLaunchDestination::Embedded,
        TerminalLaunchDestination::Detached,
    ] {
        for kind in [
            TerminalCreationKind::BuildShell,
            TerminalCreationKind::Devshell,
            TerminalCreationKind::Menuconfig,
            TerminalCreationKind::DevtoolShell,
            TerminalCreationKind::Utility,
        ] {
            let mut app = chooser(kind, destination);
            let dialog = app.active_dialog().cloned();
            assert!(hold_pending_inspection_launch(
                &mut app,
                true,
                true,
                Input::Enter
            ));
            assert_eq!(app.active_dialog().cloned(), dialog);
            assert_eq!(app.screen, Screen::Recipes);
            assert_eq!(app.focus, FocusTarget::Dialog);
            assert!(!hold_pending_inspection_launch(
                &mut app,
                true,
                true,
                Input::Esc
            ));
            assert_eq!(update(&mut app, Action::CancelTerminalLaunch), None);
            assert!(app.active_dialog().is_none());
        }
    }
    for (native, pending) in [(false, false), (false, true), (true, false)] {
        let mut app = chooser(
            TerminalCreationKind::Devshell,
            TerminalLaunchDestination::Embedded,
        );
        assert!(!hold_pending_inspection_launch(
            &mut app,
            native,
            pending,
            Input::Enter
        ));
        assert!(app.notification.is_none());
    }
    let mut app = chooser(
        TerminalCreationKind::GitUi,
        TerminalLaunchDestination::Embedded,
    );
    assert!(!hold_pending_inspection_launch(
        &mut app,
        true,
        true,
        Input::Enter
    ));
    let Some(Dialog::TerminalLaunch(dialog)) = app.active_dialog_mut() else {
        unreachable!()
    };
    dialog.request.kind = TerminalCreationKind::Utility;
    dialog.request.arguments = vec!["dtc".into()];
    assert!(!hold_pending_inspection_launch(
        &mut app,
        true,
        true,
        Input::Enter
    ));
}

#[test]
fn inspection_lease_waiting_chooser_remains_visible_and_focus_trapped() {
    use ratatui::backend::TestBackend;
    let mut app = chooser(
        TerminalCreationKind::Devshell,
        TerminalLaunchDestination::Embedded,
    );
    assert!(hold_pending_inspection_launch(
        &mut app,
        true,
        true,
        Input::Enter
    ));
    for (width, height) in [(160, 50), (80, 24), (40, 12)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();
        assert_eq!(app.focus, FocusTarget::Dialog);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::TerminalLaunch(_))
        ));
        if width >= 80 {
            let buffer = terminal.backend().buffer();
            let text = (0..height)
                .map(|y| {
                    (0..width)
                        .map(|x| buffer[(x, y)].symbol())
                        .collect::<String>()
                })
                .collect::<Vec<_>>()
                .join("\n");
            assert!(text.contains("Esc cancel without spawning"), "{text}");
            assert!(text.contains("busybox"), "{text}");
        }
    }
}
