use super::*;
use yoctui_model::{Action, Screen, WorkbenchAction as W};

async fn loaded(store: &mut Store, app: &mut App) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while store.loading.is_some() && Instant::now() < deadline {
        store.poll(app).await;
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert!(store.loading.is_none());
}

#[tokio::test]
async fn workbench_recovery_lock_prevents_second_client_overwrite() {
    let fixture = tempfile::tempdir().unwrap();
    let path = fixture.path().join("workbenches.json");
    let mut first = Store::start_at(path.clone());
    let mut app = App::new(16, 4096);
    loaded(&mut first, &mut app).await;
    assert!(first.ready());
    let mut second = Store::start_at(path.clone());
    let mut other = App::new(16, 4096);
    loaded(&mut second, &mut other).await;
    assert!(!second.ready());
    assert!(other.workbench.persistence_error.is_some());
    yoctui_model::update(
        &mut app,
        Action::Workbench(W::OpenRecipe {
            recipe: "demo".into(),
            root: fixture.path().into(),
        }),
    );
    first.flush(&mut app).await.unwrap();
    assert!(path.is_file());
    assert_eq!(
        crate::workbench_io::load(&path).unwrap().sessions[0].name,
        "demo"
    );
    drop(first);
    let mut reopened = Store::start_at(path);
    loaded(&mut reopened, &mut other).await;
    assert!(reopened.ready());
    assert_eq!(other.workbench.sessions[0].name, "demo");
}

#[tokio::test]
async fn workbench_corrupt_recovery_fails_closed_and_preserves_original() {
    let fixture = tempfile::tempdir().unwrap();
    let path = fixture.path().join("workbenches.json");
    std::fs::write(&path, "corrupt").unwrap();
    let mut store = Store::start_at(path.clone());
    let mut app = App::new(16, 4096);
    loaded(&mut store, &mut app).await;
    assert!(!store.ready());
    store.flush(&mut app).await.unwrap();
    assert_eq!(std::fs::read_to_string(path).unwrap(), "corrupt");
}

#[tokio::test]
async fn workbench_loading_retains_deferred_devtool_handoff() {
    let fixture = tempfile::tempdir().unwrap();
    let mut store = Store::start_at(fixture.path().join("workbenches.json"));
    let mut app = App::new(16, 4096);
    app.screen = Screen::Workbench;
    app.workbench.recovering = true;
    app.workbench.deferred_recipe = Some(("demo".into(), fixture.path().into()));
    loaded(&mut store, &mut app).await;
    assert_eq!(app.workbench.deferred_recipe.as_ref().unwrap().0, "demo");
    assert!(!app.workbench.recovering);
}
