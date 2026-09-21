use super::*;

#[tokio::test]
async fn maintenance_release_workspace_builds_exact_previews_for_every_form() {
    let fixture = Fixture::new("#!/bin/sh\nexit 0\n");
    let (mut app, mut coordinator, capability_request, history) = refreshed_release_coordinator(
        &fixture,
        "#!/bin/sh\nexit 0\n",
        "#!/bin/sh\nprintf 'comparison\\n'\n",
        "#!/bin/sh\nexit 0\n",
    )
    .await;

    let locked = fixture.root.join("locked.inc");
    let input = fixture.root.join("input-cache");
    let output = fixture.root.join("output-cache");
    fs::write(&locked, b"SIGGEN_LOCKEDSIGS = \"\"\n").unwrap();
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(&output).unwrap();
    let requests = [
        Effect::Maintenance(MaintenanceEffect::PreviewLockedSignatureCache {
            capability_request,
            request: LockedSignatureCacheRequest::new(locked, input, output, "ubuntu".into(), None)
                .unwrap(),
        }),
        Effect::Maintenance(MaintenanceEffect::PreviewBuildHistoryComparison {
            capability_request,
            request: BuildComparisonRequest::new(BuildComparisonRequest {
                repository: history,
                from_revision: Some("HEAD^".into()),
                to_revision: Some("HEAD".into()),
                report_version: true,
                report_all: false,
                signatures: true,
                signature_diff: false,
                exclude_paths: vec!["images/*".into()],
                no_colour: true,
            })
            .unwrap(),
        }),
        Effect::Maintenance(MaintenanceEffect::PreviewGitArchive {
            capability_request,
            request: GitArchiveRequest::new(GitArchiveRequest {
                data_dir: fixture.root.clone(),
                git_dir: fixture.root.join("release.git"),
                create: true,
                bare: false,
                create_tag: false,
                branch_name: "release".into(),
                tag_name: None,
                commit_subject: "Release".into(),
                commit_body: String::new(),
                tag_subject: "Tag".into(),
                tag_body: String::new(),
                exclusions: Vec::new(),
                notes: Vec::new(),
                push_remote: None,
            })
            .unwrap(),
        }),
    ];
    let expected = [
        MaintenanceTool::LockedSignatureCache,
        MaintenanceTool::BuildHistoryDiff,
        MaintenanceTool::GitArchive,
    ];
    for (index, (effect, tool)) in requests.into_iter().zip(expected).enumerate() {
        app.dialogs.clear();
        coordinator.handle_effect(&mut app, effect).await;
        poll_until(&mut coordinator, &mut app, |app, _| {
            matches!(
                app.active_dialog(),
                Some(yoctui_model::Dialog::Maintenance(dialog))
                    if matches!(dialog.as_ref(), yoctui_model::MaintenanceDialog::Confirm(_))
            )
        })
        .await;
        let Some(yoctui_model::Dialog::Maintenance(dialog)) = app.active_dialog() else {
            panic!("release confirmation is absent");
        };
        let yoctui_model::MaintenanceDialog::Confirm(preview) = dialog.as_ref() else {
            panic!("wrong release confirmation");
        };
        assert_eq!(preview.operation.tool(), tool);
        assert!(!preview.arguments.is_empty());
        if index == 2 {
            assert!(!preview.operation.network_side_effect());
        }
    }
    coordinator.shutdown().await;
}
