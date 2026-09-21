use super::*;

#[test]
fn ux_command_center_borrows_bounded_contexts_work_favorites_and_terminals() {
    let mut app = App::new(16, 4_096);
    for id in 1..=5 {
        let _ = update(
            &mut app,
            Action::QueueBackgroundJob(BackgroundJobSpec {
                id: BackgroundJobId(id),
                kind: BackgroundJobKind::Build,
                title: format!("build-{id}"),
                context: BackgroundJobContext {
                    workspace: Some(Screen::Recipes),
                    target: Some(format!("image-{id}")),
                    recipe: Some(format!("recipe-{id}")),
                    ..BackgroundJobContext::default()
                },
                cancellation_supported: true,
                queued_at: SystemTime::UNIX_EPOCH + Duration::from_secs(id),
            }),
        );
        app.daemon.pty_sessions.push(ClientDaemonPtySummary {
            id,
            name: format!("shell-{id}"),
            lifecycle: ClientDaemonLifecycle::Running,
            viewers: 1,
        });
    }
    let command = builtin_raw_catalog()
        .commands
        .iter()
        .find(|command| {
            command.parameters.is_empty()
                && matches!(command.execution, RawExecutionPolicy::Executable { .. })
        })
        .expect("the built-in catalog retains a parameterless executable command");
    app.raw_mode.favorites.push(
        RawFavorite::new(
            command,
            "Inspect environment",
            Default::default(),
            RawAdditionalArguments::from_vec(Vec::new()).unwrap(),
            0,
        )
        .unwrap(),
    );
    app.pty_selection = 4;

    let center = app.command_center_projection_at(SystemTime::UNIX_EPOCH);
    assert_eq!(
        center.recent_contexts.len(),
        COMMAND_CENTER_COLLECTION_LIMIT
    );
    assert_eq!(center.active_jobs.len(), COMMAND_CENTER_COLLECTION_LIMIT);
    assert_eq!(center.active_jobs[0].id, BackgroundJobId(5));
    assert_eq!(center.favorite_commands.len(), 1);
    assert_eq!(
        center.favorite_commands[0].favorite.name,
        "Inspect environment"
    );
    assert_eq!(center.terminals.len(), COMMAND_CENTER_COLLECTION_LIMIT);
    assert_eq!(
        center.terminals[0].id, 5,
        "selected terminal projects first"
    );
    assert_eq!(center.terminals[1].id, 1);

    let _ = update(&mut app, Action::OpenRawFavorites);
    assert_eq!(app.screen, Screen::RawMode);
    assert_eq!(app.raw_mode.view, RawModeView::Favorites);
}
