use super::*;

fn artifacts_app(total: usize) -> App {
    let mut app = App::new(20, 20_000);
    app.screen = Screen::Images;
    app.focus = FocusTarget::Workspace;
    let artifacts = (0..total)
        .map(|index| yoctui_model::ImageArtifact {
            identity: yoctui_model::ImageArtifactIdentity {
                machine: "qemux86-64".into(),
                image: format!("core-image-with-a-really-long-recipe-name-{index:03}"),
                path: format!(
                    "/deploy/qemux86-64/file-{index:03}-with-a-very-long-deploy-name.dts"
                )
                .into(),
            },
            kind: yoctui_model::ImageArtifactKind::Other,
            size_bytes: ImageArtifactField::Available(8192),
            modified_unix_seconds: ImageArtifactField::Available(1_700_000_000),
            checksums: ImageArtifactField::Unavailable,
            manifests: ImageArtifactField::Unavailable,
            licenses: ImageArtifactField::Unavailable,
            spdx: ImageArtifactField::Unavailable,
            wic_files: ImageArtifactField::Unavailable,
        })
        .collect::<Vec<_>>();
    app.image_artifact_selection = artifacts.first().map(|artifact| artifact.identity.clone());
    app.image_artifacts = ImageArtifactInventoryState::Partial {
        request: yoctui_model::ImageArtifactRequest {
            generation: 1,
            machine: "qemux86-64".into(),
        },
        inventory: yoctui_model::ImageArtifactInventory {
            machine: "qemux86-64".into(),
            deploy_directory: ImageArtifactField::Available("/deploy/qemux86-64".into()),
            artifacts,
        },
        limitations: vec!["skipped symlink".into()],
    };
    app
}

#[test]
fn image_artifact_viewport_keeps_every_selection_visible_with_long_wrapping_names() {
    let mut app = artifacts_app(80);
    // Draw the actual pane without the Inspector: it must not mask a hidden row.
    for (width, height) in [(140, 35), (85, 18), (60, 12), (40, 10)] {
        update(&mut app, Action::SelectImageArtifact { delta: isize::MIN });
        for index in 0..80 {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal
                .draw(|frame| image_artifacts_workspace(frame, &app, frame.area()))
                .unwrap();
            let text = buffer_text(terminal.backend().buffer());
            assert!(
                text.contains(&format!("file-{index:03}")),
                "{width}x{height} index {index}: {text}"
            );
            update(&mut app, Action::SelectImageArtifact { delta: 1 });
        }
        for delta in [-1, -10, isize::MIN, 10, isize::MAX] {
            update(&mut app, Action::SelectImageArtifact { delta });
            let file = app
                .selected_image_artifact()
                .unwrap()
                .identity
                .path
                .file_name()
                .unwrap()
                .to_string_lossy();
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal
                .draw(|frame| image_artifacts_workspace(frame, &app, frame.area()))
                .unwrap();
            let text = buffer_text(terminal.backend().buffer());
            assert!(text.contains(&file[..8]), "{text}");
        }
    }
}

#[test]
fn image_artifact_metadata_is_readable_and_tiny_empty_views_do_not_panic() {
    assert_eq!(
        super::super::image_inspector::image_artifact_timestamp(Some(1_700_000_000)),
        "2023-11-14 22:13:20Z"
    );
    assert_eq!(
        super::super::image_inspector::image_artifact_timestamp(Some(0)),
        "1970-01-01 00:00:00Z"
    );
    assert_eq!(
        super::super::image_inspector::image_artifact_timestamp(None),
        "unavailable"
    );
    assert_eq!(
        super::super::image_inspector::image_artifact_timestamp(Some(u64::MAX)),
        "unavailable"
    );
    let app = artifacts_app(1);
    let mut terminal = Terminal::new(TestBackend::new(140, 20)).unwrap();
    terminal
        .draw(|frame| image_artifacts_workspace(frame, &app, frame.area()))
        .unwrap();
    let text = buffer_text(terminal.backend().buffer());
    assert!(
        text.contains("8192") && text.contains("2023-11-14 22:13:20Z"),
        "{text}"
    );
    let inspector = super::super::image_inspector::image_artifact_inspector_text(&app);
    assert!(
        inspector.contains("8192 bytes") && inspector.contains("Last modified (UTC): 2023-11-14")
    );
    for total in [0, 1, 80] {
        let app = artifacts_app(total);
        for (width, height) in [(1, 1), (20, 3), (45, 8)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal
                .draw(|frame| image_artifacts_workspace(frame, &app, frame.area()))
                .unwrap();
        }
    }
}

fn buffer_text(buffer: &ratatui::buffer::Buffer) -> String {
    buffer.content().iter().map(|cell| cell.symbol()).collect()
}
