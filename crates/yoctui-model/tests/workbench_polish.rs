use std::path::Path;
use yoctui_model::*;

fn send(app: &mut App, action: WorkbenchAction) -> Option<Effect> {
    update(app, Action::Workbench(action))
}
fn fixture(text: &str) -> App {
    let mut app = App::new(32, 8192);
    app.workspace.build_dir = Some("/build/poky".into());
    app.workspace.recipes.push(Recipe {
        name: "demo".into(),
        file: Some("/layers/demo.bb".into()),
        ..Default::default()
    });
    send(
        &mut app,
        WorkbenchAction::OpenRecipe {
            recipe: "demo".into(),
            root: "/source".into(),
        },
    );
    open(&mut app, "main.c", text);
    send(
        &mut app,
        WorkbenchAction::Viewport {
            rows: 8,
            columns: 30,
        },
    );
    app
}
fn open(app: &mut App, name: &str, text: &str) {
    app.workbench.file_request += 1;
    send(
        app,
        WorkbenchAction::FileLoaded {
            session: app.workbench.active.unwrap(),
            generation: app.workbench.file_request,
            root: "/source".into(),
            path: Path::new("/source").join(name),
            result: Ok(text.into()),
        },
    );
}
fn tab(app: &App) -> &WorkbenchTab {
    let s = app.workbench.current().unwrap();
    &s.tabs[s.tab]
}
fn metadata_authority(app: &mut App) {
    let capability = CapabilityId::BitBakeRecipeMetadata;
    install_workspace_compatibility(
        app,
        DaemonCompatibilitySnapshot {
            snapshot: CapabilitySnapshot {
                generation: 1,
                environment: YoctoEnvironmentIdentity::default(),
                capabilities: vec![CapabilityRecord {
                    id: capability,
                    state: CapabilityState::Available,
                    evidence: vec![CapabilityEvidence {
                        kind: CapabilityEvidenceKind::DirectProbe,
                        outcome: CapabilityEvidenceOutcome::Positive,
                        subject: capability.as_str().into(),
                        detail: "fixture metadata inspection".into(),
                        argv: vec!["fixture".into()],
                    }],
                }],
            },
            implementations: [(
                capability,
                CapabilityImplementation {
                    id: "fixture.metadata".into(),
                    kind: CapabilityImplementationKind::Command,
                },
            )]
            .into_iter()
            .collect(),
        },
    )
    .unwrap();
}

#[test]
fn workbench_scroll_keeps_cursor_and_page_motion_uses_actual_editor_height() {
    let text = (0..100).map(|i| format!("line{i}\n")).collect::<String>();
    let mut app = fixture(&text);
    send(&mut app, WorkbenchAction::ScrollEditor(40));
    assert_eq!(tab(&app).viewport.top, 40);
    assert_eq!(tab(&app).document.cursor, 0);
    send(&mut app, WorkbenchAction::ScrollEditor(isize::MAX));
    assert_eq!(tab(&app).viewport.top, 93);
    send(
        &mut app,
        WorkbenchAction::Edit(PopupEditorCommand::PageDown),
    );
    assert_eq!(tab(&app).document.position().line, 8);
    assert!(tab(&app).viewport.top <= 8);
    send(
        &mut app,
        WorkbenchAction::Viewport {
            rows: 3,
            columns: 12,
        },
    );
    send(
        &mut app,
        WorkbenchAction::Edit(PopupEditorCommand::PageDown),
    );
    assert_eq!(tab(&app).document.position().line, 11);
}
#[test]
fn workbench_resize_reveals_active_cursor_but_preserves_independent_scroll() {
    let text = (0..100)
        .map(|i| format!("line{i}abcdefghijklmnopqrstuvwxyz\n"))
        .collect::<String>();
    let mut app = fixture(&text);
    send(
        &mut app,
        WorkbenchAction::Edit(PopupEditorCommand::SelectPosition {
            line: 20,
            column: 25,
            extend: false,
        }),
    );
    send(
        &mut app,
        WorkbenchAction::Viewport {
            rows: 3,
            columns: 8,
        },
    );
    assert!((tab(&app).viewport.top..tab(&app).viewport.top + 3).contains(&20));
    assert!((tab(&app).viewport.left..tab(&app).viewport.left + 8).contains(&25));
    send(&mut app, WorkbenchAction::ScrollEditor(20));
    let top = tab(&app).viewport.top;
    send(
        &mut app,
        WorkbenchAction::Viewport {
            rows: 5,
            columns: 12,
        },
    );
    assert_eq!(tab(&app).viewport.top, top);
    assert_eq!(tab(&app).document.position().line, 20);
}

#[test]
fn workbench_tab_and_wide_character_columns_roundtrip() {
    let line = "\t界ab\tx";
    for column in 0..=line.chars().count() {
        assert_eq!(
            workbench_source_column(line, workbench_visual_column(line, column)),
            column
        );
    }
    let mut app = fixture("\t界abcdefghijklmnopqrstuvwxyz");
    send(
        &mut app,
        WorkbenchAction::Viewport {
            rows: 4,
            columns: 8,
        },
    );
    send(&mut app, WorkbenchAction::Edit(PopupEditorCommand::End));
    assert!(tab(&app).viewport.left > 20);
}
#[test]
fn workbench_search_wraps_both_directions_and_can_be_cleared() {
    let mut app = fixture("needle\nother\nNEEDLE\nneedle");
    send(&mut app, WorkbenchAction::Form(WorkbenchFormKind::Find));
    for c in "needle".chars() {
        send(&mut app, WorkbenchAction::FormText(c));
    }
    send(&mut app, WorkbenchAction::ConfirmForm);
    assert_eq!(tab(&app).document.search_state().matches.len(), 3);
    send(&mut app, WorkbenchAction::SearchMatch { backwards: true });
    assert_eq!(tab(&app).document.position().line, 3);
    send(&mut app, WorkbenchAction::SearchMatch { backwards: false });
    assert_eq!(tab(&app).document.position().line, 0);
    send(&mut app, WorkbenchAction::Form(WorkbenchFormKind::Find));
    send(&mut app, WorkbenchAction::FormClear);
    send(&mut app, WorkbenchAction::ConfirmForm);
    assert!(tab(&app).document.search_state().query.is_empty());
}
#[test]
fn workbench_breakpoints_move_with_inserted_lines_and_survive_recovery() {
    let mut app = fixture("int first;\nint second;\n");
    send(&mut app, WorkbenchAction::BreakpointAt(2));
    send(&mut app, WorkbenchAction::Edit(PopupEditorCommand::Newline));
    assert_eq!(
        tab(&app).breakpoints.iter().copied().collect::<Vec<_>>(),
        vec![3]
    );
    send(&mut app, WorkbenchAction::Edit(PopupEditorCommand::Undo));
    assert_eq!(
        tab(&app).breakpoints.iter().copied().collect::<Vec<_>>(),
        vec![2]
    );
    send(&mut app, WorkbenchAction::ToggleWhitespace);
    let saved = app.workbench.saved();
    let restored = WorkbenchState::restore(saved).unwrap();
    assert!(restored.sessions[0].show_whitespace);
    assert!(restored.sessions[0].tabs[0].breakpoints.contains(&2));
}
#[test]
fn workbench_tabs_wrap_and_picker_never_discards_a_dirty_tab() {
    let mut app = fixture("first");
    open(&mut app, "second.c", "second");
    send(&mut app, WorkbenchAction::Tab(1));
    assert_eq!(app.workbench.current().unwrap().tab, 0);
    send(
        &mut app,
        WorkbenchAction::Edit(PopupEditorCommand::Insert('x')),
    );
    send(&mut app, WorkbenchAction::Tabs);
    send(&mut app, WorkbenchAction::TabPickerClose);
    assert_eq!(app.workbench.current().unwrap().tabs.len(), 2);
    send(&mut app, WorkbenchAction::TabPickerSelect(1));
    send(&mut app, WorkbenchAction::TabPickerClose);
    assert_eq!(app.workbench.current().unwrap().tabs.len(), 1);
    assert!(tab(&app).is_dirty());
}
#[test]
fn workbench_compile_reads_authoritative_tasks_and_reviews_the_exact_operation() {
    let mut app = fixture("int main() {}\n");
    metadata_authority(&mut app);
    assert!(
        matches!(send(&mut app, WorkbenchAction::Compile), Some(Effect::GetRecipeMetadata(name)) if name == "demo")
    );
    assert!(app.active_dialog().is_none());
    send(&mut app, WorkbenchAction::Cancel);
    update(
        &mut app,
        Action::RecipeMetadataLoaded(RecipeMetadata {
            recipe: "demo".into(),
            tasks: Some(vec!["do_compile".into()]),
            ..Default::default()
        }),
    );
    send(&mut app, WorkbenchAction::Compile);
    assert!(
        matches!(app.active_dialog(), Some(Dialog::RecipeTaskConfirmation(r)) if r.targets == ["demo"] && r.task.as_deref() == Some("compile") && !r.force)
    );
    update(&mut app, Action::CancelRecipeTask);
    assert_eq!(app.screen, Screen::Workbench);
    send(
        &mut app,
        WorkbenchAction::Edit(PopupEditorCommand::Insert('x')),
    );
    send(&mut app, WorkbenchAction::Compile);
    assert!(app.active_dialog().is_none());
}

#[test]
fn workbench_denied_compile_does_not_leave_an_orphaned_loading_indicator() {
    let mut app = fixture("int main() {}\n");
    assert!(send(&mut app, WorkbenchAction::Compile).is_none());
    assert!(app.workbench.pending_compile.is_none());
    assert!(app.notification.is_some());
}

#[test]
fn workbench_previous_store_format_loads_with_safe_defaults() {
    let app = fixture("source");
    let mut value = serde_json::to_value(app.workbench.saved()).unwrap();
    let session = value["sessions"][0].as_object_mut().unwrap();
    session.remove("debug");
    session.remove("show_whitespace");
    for tab in session["tabs"].as_array_mut().unwrap() {
        tab.as_object_mut().unwrap().remove("breakpoints");
    }
    let saved: SavedWorkbenches = serde_json::from_value(value).unwrap();
    let restored = WorkbenchState::restore(saved).unwrap();
    assert_eq!(restored.sessions[0].debug, WorkbenchDebugConfig::default());
    assert!(!restored.sessions[0].show_whitespace);
    assert!(restored.sessions[0].tabs[0].breakpoints.is_empty());
    assert_eq!(restored.sessions[0].tabs[0].document.text, "source");
}
#[test]
fn workbench_sdk_and_devtool_build_prefill_the_bound_recipe() {
    let mut app = fixture("source");
    for command in [DevtoolUtilityCommand::Build, DevtoolUtilityCommand::IdeSdk] {
        send(&mut app, WorkbenchAction::Utility(command));
        let Some(Dialog::YoctoUtility(dialog)) = app.active_dialog() else {
            panic!()
        };
        let args = dialog.arguments().unwrap();
        assert!(args.contains(&"demo".into()));
        if command == DevtoolUtilityCommand::IdeSdk {
            assert!(args.windows(2).any(|pair| pair == ["--ide", "none"]));
            assert!(args.contains(&"--no-strip".into()));
        }
        update(&mut app, Action::CancelYoctoUtility);
        assert_eq!(app.screen, Screen::Workbench);
    }
}
#[test]
fn workbench_gdb_uses_argv_and_maps_breakpoints_to_the_compiled_source_prefix() {
    let mut config = WorkbenchDebugConfig {
        local: true,
        executable: "/build/program with spaces".into(),
        source_prefix: "/compiled/source".into(),
        ..Default::default()
    };
    let request = config
        .request(
            "/source".into(),
            "/usr/bin/gdb".into(),
            7,
            &[("/source/main with spaces.c".into(), 12)],
        )
        .unwrap();
    assert!(
        request
            .arguments
            .contains(&"--se=/build/program with spaces".into())
    );
    assert!(
        request
            .arguments
            .contains(&"break -source \"/compiled/source/main with spaces.c\" -line 12".into())
    );
    assert!(
        !request
            .arguments
            .iter()
            .any(|s| s.starts_with("target remote"))
    );
    config.local = false;
    config.endpoint = "127.0.0.1:2345\nshell touch /tmp/unsafe".into();
    assert!(config.validate().is_err());
}
#[test]
fn workbench_debug_results_cannot_open_a_cancelled_launch_review() {
    let mut app = fixture("source");
    send(&mut app, WorkbenchAction::Debug);
    let dialog = app.workbench.debug_dialog.as_mut().unwrap();
    dialog.config.local = true;
    dialog.config.executable = "/build/demo".into();
    let Some(Effect::Workbench(WorkbenchRequest::PrepareDebug {
        session,
        generation,
        ..
    })) = send(&mut app, WorkbenchAction::DebugReview)
    else {
        panic!()
    };
    send(&mut app, WorkbenchAction::Cancel);
    let request = WorkbenchDebugConfig {
        local: true,
        executable: "/build/demo".into(),
        ..Default::default()
    }
    .request("/source".into(), "/usr/bin/gdb".into(), session, &[])
    .unwrap();
    send(
        &mut app,
        WorkbenchAction::DebugPrepared {
            session,
            generation,
            result: Ok(request),
        },
    );
    assert!(app.active_dialog().is_none());
}

fn terminal_fixture() -> App {
    let mut app = fixture("source");
    app.daemon.instance_id = Some(DaemonModelInstanceId([1; 16]));
    app.daemon.status = ClientReplicaStatus::Current;
    app.terminal.client_id = Some([7; 16]);
    app.daemon.pty_sessions.push(ClientDaemonPtySummary {
        id: 41,
        name: "Workbench GDB".into(),
        lifecycle: ClientDaemonLifecycle::Running,
        viewers: 1,
    });
    app.daemon.pty_details.push(ClientDaemonPtyDetails {
        id: 41,
        kind: ClientDaemonPtyKind::Utility,
        cwd: "/source".into(),
        columns: 100,
        rows: 30,
        writer: Some([7; 16]),
        writer_epoch: 9,
        exit_code: None,
        restartable: false,
    });
    app.select_terminal_pane(app.pane_layout.focused, 0);
    let instance = app.daemon.instance_id;
    let s = app.workbench.current_mut().unwrap();
    s.terminal = Some(41);
    s.terminal_daemon = instance;
    s.focus = WorkbenchFocus::Terminal;
    app
}
#[test]
fn workbench_immediate_debugger_return_waits_for_the_released_writer_epoch() {
    let mut app = terminal_fixture();
    assert!(matches!(
        send(&mut app, WorkbenchAction::ReleaseTerminal),
        Some(Effect::Terminal(TerminalEffect::ReleaseControl {
            session_id: 41,
            writer_epoch: 9
        }))
    ));
    assert!(!app.selected_terminal_is_writer());
    send(&mut app, WorkbenchAction::Focus(WorkbenchFocus::Editor));
    assert!(send(&mut app, WorkbenchAction::Terminal).is_none());
    assert!(app.terminal.reopen_after_release);
    assert!(app.pending_terminal_writer_effect().is_none());
    app.daemon.pty_details[0].writer = None;
    app.daemon.pty_details[0].writer_epoch = 10;
    assert_eq!(
        app.pending_terminal_writer_effect(),
        Some(TerminalEffect::TakeControl {
            session_id: 41,
            expected_epoch: 10
        })
    );
    assert!(app.pending_terminal_writer_effect().is_none());
}
#[test]
fn workbench_deferred_reopen_does_not_steal_control_after_focus_or_owner_changes() {
    for scenario in 0..3 {
        let mut app = terminal_fixture();
        send(&mut app, WorkbenchAction::ReleaseTerminal);
        send(&mut app, WorkbenchAction::Terminal);
        app.daemon.pty_details[0].writer_epoch = 10;
        app.daemon.pty_details[0].writer = if scenario == 1 { Some([8; 16]) } else { None };
        if scenario == 0 {
            send(&mut app, WorkbenchAction::Focus(WorkbenchFocus::Editor));
        }
        if scenario == 2 {
            send(&mut app, WorkbenchAction::Menu);
        }
        assert!(app.pending_terminal_writer_effect().is_none());
        assert!(!app.selected_terminal_is_writer());
    }
}
