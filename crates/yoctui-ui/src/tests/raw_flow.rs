use super::*;
use yoctui_model::{RawInteractionMode, RawModeAction as Raw, RawModeView as View};

#[test]
fn raw_history_renders_retained_execution_and_metadata_without_placeholder() {
    let mut app = raw_output_app(RawInteractionMode::NoninteractiveJob);
    app.raw_mode.view = View::History;
    for (width, height) in [(80, 24), (100, 30), (160, 48), (240, 80)] {
        let output = rendered_text(&app, width, height);
        assert!(output.contains("Raw Mode / History"), "{output}");
        assert!(output.contains("RUNNING"), "{output}");
        assert!(output.contains("output retained"), "{output}");
        assert!(output.contains("Enter retained output"), "{output}");
        assert!(!output.contains("Some(Running)"));
    }
    let execution = app.raw_mode.execution_states.values_mut().next().unwrap();
    let event = yoctui_model::RawExecutionEvent {
        request_id: execution.request.id.clone(),
        sequence: execution.cursor.sequence + 1,
        generation: execution.cursor.generation + 1,
        kind: yoctui_model::RawExecutionEventKind::Finished {
            result: yoctui_model::RawExecutionResult {
                outcome: yoctui_model::RawExecutionOutcome::Succeeded,
                exit_code: Some(0),
                message: None,
                elapsed_ms: 100,
                durable_reference: None,
            },
        },
    };
    yoctui_model::reduce_raw_execution(execution, event).unwrap();
    app.raw_mode
        .history
        .push(yoctui_model::RawHistoryRecord::from_terminal(execution).unwrap());
    app.raw_mode.execution_states.clear();
    let output = rendered_text(&app, 160, 48);
    assert!(output.contains("SUCCEEDED"), "{output}");
    assert!(output.contains("metadata only"), "{output}");
    assert!(
        output.contains("original output cannot be reconstructed"),
        "{output}"
    );
}

#[test]
fn raw_output_final_line_is_visible_and_search_is_highlighted() {
    let mut app = raw_output_app(RawInteractionMode::NoninteractiveJob);
    let id = app.raw_mode.output.request.clone().unwrap();
    let execution = app.raw_mode.execution_states.get_mut(&id).unwrap();
    let event = yoctui_model::RawExecutionEvent {
        request_id: id,
        sequence: execution.cursor.sequence + 1,
        generation: execution.cursor.generation + 1,
        kind: yoctui_model::RawExecutionEventKind::Output {
            chunk: yoctui_model::RawOutputChunk {
                stream_id: execution.stdout.stream_id.clone(),
                stream: yoctui_model::RawOutputStream::Stdout,
                sequence: execution.stdout.next_sequence,
                text: (0..100)
                    .map(|index| format!("output-row-{index:03}\n"))
                    .collect(),
                truncated_bytes: 0,
                dropped_lines: 0,
            },
        },
    };
    yoctui_model::reduce_raw_execution(execution, event).unwrap();
    app.raw_mode.output.query = "output-row-099".into();
    for (width, height) in [(80, 24), (100, 30), (160, 48), (240, 80)] {
        let output = rendered_text(&app, width, height);
        assert!(
            output.contains("output-row-099"),
            "last row clipped at {width}x{height}: {output}"
        );
        assert!(output.contains("hits 1"), "{output}");
    }
}

#[test]
fn raw_long_argv_review_scrolls_to_last_argument_with_fixed_confirmation_hint() {
    let mut preview = raw_preview_fixture();
    for index in 5..65 {
        preview
            .indexed_arguments
            .push(yoctui_model::RawPreviewArgument {
                index,
                value: format!("native-argument-{index:03}-{}", "界".repeat(30)),
                source: yoctui_model::RawPreviewArgumentSource::Additional { index },
            });
    }
    for (width, height) in [(80, 24), (100, 30), (160, 48), (240, 80)] {
        let first = rendered_region_rows(width, height, |frame, area| {
            crate::raw_dialog::render_raw_execution_preview_at(frame, &preview, area, 0, None);
        })
        .join("\n");
        assert!(first.contains("Safety:"), "{first}");
        assert!(first.contains("Enter run"), "{first}");
        let last = rendered_region_rows(width, height, |frame, area| {
            crate::raw_dialog::render_raw_execution_preview_at(
                frame,
                &preview,
                area,
                usize::MAX,
                None,
            );
        })
        .join("\n");
        assert!(last.contains("native-argument-064"), "{last}");
        assert!(last.contains("Enter run"), "{last}");
        assert!(last.contains("Esc edit"), "{last}");
    }
}

#[test]
fn raw_favorite_overlay_shows_exact_target_and_modal_footer() {
    let mut app = raw_command_list_app();
    set_raw_command_query(&mut app, "--version");
    update(&mut app, Action::RawMode(Raw::ToggleFavorite));
    update(&mut app, Action::RawMode(Raw::OpenFavorites));
    update(&mut app, Action::RawMode(Raw::RemoveFavorite));
    for (width, height) in [(80, 24), (160, 48)] {
        let output = rendered_text(&app, width, height);
        assert!(output.contains("Remove Raw favorite?"), "{output}");
        assert!(output.contains("Enter remove"), "{output}");
        assert!(output.contains("not recipes or build files"), "{output}");
        assert!(!crate::footer::raw_mode_shortcuts(&app).contains("Quit"));
    }
    update(&mut app, Action::RawMode(Raw::CancelFavorite));
    update(&mut app, Action::RawMode(Raw::BeginFavoriteRename));
    assert!(rendered_text(&app, 160, 48).contains("Rename Raw favorite"));
}
