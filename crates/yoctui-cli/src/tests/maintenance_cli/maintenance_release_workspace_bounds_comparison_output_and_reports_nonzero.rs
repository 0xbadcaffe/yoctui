use super::*;

#[tokio::test]
async fn maintenance_release_workspace_bounds_comparison_output_and_reports_nonzero() {
    for (script, expected, expected_dropped) in [
        (
            "#!/bin/sh\ni=0\nwhile [ \"$i\" -lt 520 ]; do printf 'comparison-%s\\n' \"$i\"; i=$((i + 1)); done\n",
            MaintenanceSessionStatus::Succeeded,
            8,
        ),
        (
            "#!/bin/sh\nprintf 'comparison denied\\n' >&2\nexit 7\n",
            MaintenanceSessionStatus::Failed,
            0,
        ),
    ] {
        let fixture = Fixture::new("#!/bin/sh\nexit 0\n");
        let (mut app, mut coordinator, capability_request, history) =
            refreshed_release_coordinator(
                &fixture,
                "#!/bin/sh\nexit 0\n",
                script,
                "#!/bin/sh\nexit 0\n",
            )
            .await;
        let request = BuildComparisonRequest::new(BuildComparisonRequest {
            repository: history,
            from_revision: Some("HEAD^".into()),
            to_revision: Some("HEAD".into()),
            report_version: false,
            report_all: false,
            signatures: false,
            signature_diff: false,
            exclude_paths: Vec::new(),
            no_colour: true,
        })
        .unwrap();
        coordinator
            .handle_effect(
                &mut app,
                Effect::Maintenance(MaintenanceEffect::PreviewBuildHistoryComparison {
                    capability_request,
                    request,
                }),
            )
            .await;
        poll_until(&mut coordinator, &mut app, |app, _| {
            app.active_dialog().is_some()
        })
        .await;
        let Some(yoctui_model::Dialog::Maintenance(dialog)) = app.active_dialog().cloned() else {
            panic!("comparison confirmation is absent");
        };
        let yoctui_model::MaintenanceDialog::Confirm(preview) = dialog.as_ref() else {
            panic!("wrong comparison confirmation");
        };
        let effect = update(
            &mut app,
            Action::Maintenance(MaintenanceAction::ConfirmOperation(preview.clone())),
        )
        .unwrap();
        coordinator.handle_effect(&mut app, effect).await;
        poll_until(&mut coordinator, &mut app, |app, coordinator| {
            !coordinator.operation_active()
                && app
                    .maintenance
                    .sessions
                    .back()
                    .is_some_and(|session| session.status.is_terminal())
        })
        .await;
        let session = app.maintenance.sessions.back().unwrap();
        assert_eq!(session.status, expected);
        assert_eq!(session.dropped_lines, expected_dropped);
        assert!(session.output.len() <= MAX_MAINTENANCE_OUTPUT);
        assert!(app.maintenance.evidence.is_empty());
        coordinator.shutdown().await;
    }
}
