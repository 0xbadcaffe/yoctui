use super::*;

#[tokio::test]
async fn hardware_project_background_worker_installs_typed_browse_results() {
    let root = std::env::temp_dir().join(format!(
        "yoctui-project-worker-{}-{}",
        std::process::id(),
        NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&root).unwrap();
    fs::write(root.join("artifact.bin"), "stored only").unwrap();
    let mut app = App::new(32, 4096);
    let mut io = HardwareIo::default();
    let effect = yoctui_model::update(
        &mut app,
        Action::Hardware(HardwareAction::Project(
            yoctui_model::HardwareProjectAction::Request(
                yoctui_model::HardwareProjectOperation::ImportBrowse {
                    directory: root.clone(),
                },
            ),
        )),
    )
    .unwrap();
    io.submit(effect);
    tokio::time::timeout(Duration::from_secs(5), async {
        while !io.poll(&mut app).await {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(!app.hardware.projects.loading);
    assert_eq!(
        app.hardware.projects.import_browser.as_ref().unwrap().1[0].kind,
        None
    );
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn hardware_project_txt_is_searchable_and_binary_schematic_has_an_honest_limitation() {
    let root = std::env::temp_dir().join(format!(
        "yoctui-hardware-preview-{}-{}",
        std::process::id(),
        NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&root).unwrap();
    let txt = root.join("notes.txt");
    fs::write(&txt, "Bootloader complete\nKernel testing\x1b\n").unwrap();
    let request = HardwareLoadRequest {
        generation: 1,
        document: yoctui_model::HardwareDocument {
            path: txt,
            category: yoctui_model::HardwareCategory::Other,
            kind: HardwareDocumentKind::Text,
        },
        page: 1,
    };
    let (_, preview, searchable) = load_document(request).await.unwrap();
    assert_eq!(searchable, vec!["Bootloader complete", "Kernel testing"]);
    assert!(matches!(
        preview,
        HardwarePreview::Text {
            limitation: None,
            ..
        }
    ));
    let schematic = root.join("board.SchDoc");
    fs::write(&schematic, [0xd0, 0xcf, 0x11, 0xe0]).unwrap();
    let request = HardwareLoadRequest {
        generation: 2,
        document: yoctui_model::HardwareDocument {
            path: schematic,
            category: yoctui_model::HardwareCategory::Other,
            kind: HardwareDocumentKind::Altium,
        },
        page: 1,
    };
    let (_, preview, searchable) = load_document(request).await.unwrap();
    assert!(searchable.is_empty());
    assert!(
        matches!(preview, HardwarePreview::Text { limitation: Some(message), .. } if message.contains("board.pdf") && message.contains("native Altium/Xpedition rendering is unavailable"))
    );
    fs::remove_dir_all(root).unwrap();
}
