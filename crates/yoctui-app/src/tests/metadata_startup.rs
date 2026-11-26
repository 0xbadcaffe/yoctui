use super::*;
use yoctui_model::{ClientMetadataStatus as Metadata, ClientReplicaStatus as Replica};
use yoctui_protocol::daemon::{
    DaemonBuildEvent, DaemonEvent, DaemonSnapshot, LogRecord, LogSeverity, SequencedEvent,
};
use yoctui_protocol::{RecipeData, WorkspaceData};

#[test]
fn metadata_startup_uses_daemon_evidence_and_ignores_stale_replicas() {
    let mut snapshot =
        compatibility_workspace_daemon_snapshot(&compatibility_workspace_authority(1));
    let read = |status, snapshot: &DaemonSnapshot| {
        crate::daemon_view::daemon_client_view(status, Some(snapshot), None).metadata
    };
    let record = |source: &str, severity, message: &str| LogRecord {
        source: source.into(),
        message: message.into(),
        severity,
        unix_ms: 0,
        recipe: None,
        task: None,
        path: None,
        build: None,
    };
    assert_eq!(read(Replica::Current, &snapshot), Metadata::Unknown);
    for (log, expected) in [
        (
            record(
                "daemon-metadata",
                LogSeverity::Info,
                "Loading initial compatibility authority",
            ),
            Metadata::Connecting,
        ),
        (
            record(
                "daemon-metadata",
                LogSeverity::Info,
                "Loading initial workspace and recipe inventory",
            ),
            Metadata::Loading,
        ),
        (
            record("bitbake", LogSeverity::Error, "Other job failed"),
            Metadata::Loading,
        ),
        (
            record(
                "daemon-metadata",
                LogSeverity::Error,
                "Initial metadata scan failed: timeout",
            ),
            Metadata::Failed("Initial metadata scan failed: timeout".into()),
        ),
    ] {
        snapshot.recent_logs.push(log);
        assert_eq!(read(Replica::Current, &snapshot), expected);
        assert_eq!(read(Replica::Stale, &snapshot), Metadata::Unknown);
    }
}

#[test]
fn current_inventory_updates_keep_the_selected_provider_and_reject_old_results() {
    let initial = compatibility_workspace_daemon_snapshot(&compatibility_workspace_authority(1));
    let mut app = App::new(100, 4096);
    let mut client = crate::DaemonClientSnapshot::default();
    client.replace_app(&mut app, initial.clone());

    let inventory = |revision, build: &str, names: &[&str]| SequencedEvent {
        sequence: initial.sequence + revision,
        generation: initial.generation + revision,
        event: DaemonEvent::Build(DaemonBuildEvent::Workspace {
            data: WorkspaceData {
                build_dir: Some(build.into()),
                source_dir: Some("/fixture/source".into()),
                variables: Default::default(),
                variable_provenance: Default::default(),
                variable_provenance_chain: Default::default(),
                bitbake_version: None,
                release: None,
                layers: vec![],
                recipes: names
                    .iter()
                    .map(|name| RecipeData {
                        name: (*name).into(),
                        file: Some(format!("/fixture/source/{name}.bb")),
                        version: None,
                        layer: None,
                        preferred_version: None,
                        append_count: None,
                    })
                    .collect(),
            },
        }),
    };
    let partial = inventory(1, "/fixture/build", &["b", "c"]);
    let expanded = inventory(2, "/fixture/build", &["a", "b", "c"]);
    client.apply_event_to_app(&mut app, &partial).unwrap();
    assert_eq!(app.daemon.metadata, Metadata::Ready);
    app.recipe_selection = 1;
    client.apply_event_to_app(&mut app, &expanded).unwrap();
    assert_eq!(app.recipe_selection, 2);
    assert_eq!(app.workspace.recipes[app.recipe_selection].name, "c");

    let old = inventory(1, "/fixture/old", &["obsolete"]);
    assert!(client.apply_event_to_app(&mut app, &old).is_err());
    assert_eq!(
        app.workspace.build_dir.as_deref(),
        Some(std::path::Path::new("/fixture/build"))
    );
    assert_eq!(app.workspace.recipes[app.recipe_selection].name, "c");
}
