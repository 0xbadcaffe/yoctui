use super::*;

fn request_and_preview(
    interaction: RawInteractionMode,
) -> (RawCatalog, RawPreviewRequest, RawExecutionPreview) {
    let mut catalog = super::raw_preview_tests::catalog();
    let RawExecutionPolicy::Executable { template } =
        &mut catalog.commands.first_mut().unwrap().execution
    else {
        unreachable!();
    };
    template.interaction = interaction;
    let mut request = super::raw_preview_tests::request();
    request.additional_arguments =
        RawAdditionalArguments::from_vec(vec!["--dry-run".into()]).unwrap();
    let preview = catalog
        .preview(
            &request,
            Some(&super::raw_preview_tests::authority(
                request.capability_generation,
                true,
            )),
        )
        .unwrap();
    (catalog, request, preview)
}

fn confirmed(interaction: RawInteractionMode) -> RawConfirmedExecutionRequest {
    let (catalog, request, preview) = request_and_preview(interaction);
    RawConfirmedExecutionRequest::from_reviewed_preview(
        RawRequestId::new("raw-request:test-1").unwrap(),
        &catalog,
        &request,
        &preview,
    )
    .unwrap()
}

fn queued(interaction: RawInteractionMode) -> RawExecutionState {
    RawExecutionState::queued(
        confirmed(interaction),
        RawStreamId::new("raw-stream:stdout-1").unwrap(),
        RawStreamId::new("raw-stream:stderr-1").unwrap(),
        100,
        RawEventCursor::default(),
    )
    .unwrap()
}

fn apply(state: &mut RawExecutionState, kind: RawExecutionEventKind) {
    reduce_raw_execution(
        state,
        RawExecutionEvent {
            request_id: state.request.id.clone(),
            sequence: state.cursor.sequence + 1,
            generation: state.cursor.generation + 1,
            kind,
        },
    )
    .unwrap();
}

mod raw_execution_identities_are_bounded_disjoint_and_digest_is_deterministic;

mod raw_execution_reducer_covers_job_lifecycle_output_detach_and_success;

mod raw_pty_execution_cancellation_and_fail_closed_correlation_are_exact;

mod raw_execution_streams_are_independently_bounded_and_snapshots_reject_corruption;

mod raw_execution_terminal_failure_and_loss_paths_validate;

fn terminal_for_history(
    request_suffix: &str,
    outcome: RawExecutionOutcome,
    ended_unix_ms: u64,
) -> RawExecutionState {
    let mut state = queued(RawInteractionMode::NoninteractiveJob);
    state.request.id = RawRequestId::new(format!("raw-request:{request_suffix}")).unwrap();
    if outcome == RawExecutionOutcome::Succeeded {
        apply(
            &mut state,
            RawExecutionEventKind::Starting {
                owner: RawExecutionOwner::Job(
                    RawJobId::new(format!("raw-job:{request_suffix}")).unwrap(),
                ),
            },
        );
        apply(
            &mut state,
            RawExecutionEventKind::Running {
                started_unix_ms: 100,
            },
        );
    }
    if outcome == RawExecutionOutcome::Cancelled {
        apply(&mut state, RawExecutionEventKind::CancellationRequested);
    }
    apply(
        &mut state,
        RawExecutionEventKind::Finished {
            result: RawExecutionResult {
                outcome,
                exit_code: match outcome {
                    RawExecutionOutcome::Succeeded => Some(0),
                    RawExecutionOutcome::Failed => Some(2),
                    RawExecutionOutcome::Cancelled | RawExecutionOutcome::Lost => None,
                },
                message: Some("not retained".into()),
                elapsed_ms: ended_unix_ms.saturating_sub(100),
                durable_reference: Some(
                    RawDurableReferenceId::new(format!("raw-durable:{request_suffix}")).unwrap(),
                ),
            },
        },
    );
    state
}

mod raw_history_accepts_only_terminal_outcomes_and_sanitizes_execution_authority;

mod raw_history_install_is_bounded_newest_first_and_replaces_duplicate_requests;

mod raw_output_selection_follow_search_scroll_and_session_mapping_are_bounded;
