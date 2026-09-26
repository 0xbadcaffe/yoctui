use super::*;

fn fixture_root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("yoctui-hardware-{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn hardware_browser_returns_only_supported_regular_non_symlink_entries() {
    let root = fixture_root("browse");
    fs::create_dir(root.join("subdir")).unwrap();
    fs::write(root.join("board.kicad_sch"), "(kicad_sch)").unwrap();
    fs::write(root.join("notes.txt"), "ignored").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(root.join("board.kicad_sch"), root.join("link.pdf")).unwrap();

    let (_, entries) = browse_directory(&root).unwrap();
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>(),
        vec!["subdir", "board.kicad_sch"]
    );
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn hardware_loader_decodes_bounded_raster_and_rejects_symlink() {
    let root = fixture_root("raster");
    let path = root.join("sensor.png");
    image::RgbImage::from_pixel(4, 3, image::Rgb([1, 2, 3]))
        .save(&path)
        .unwrap();
    let request = HardwareLoadRequest {
        generation: 7,
        document: yoctui_model::HardwareDocument {
            path: path.clone(),
            category: yoctui_model::HardwareCategory::Sensors,
            kind: HardwareDocumentKind::Raster,
        },
        page: 1,
    };
    let (_, preview, _) = load_document(request.clone()).await.unwrap();
    assert!(matches!(
        preview,
        HardwarePreview::Raster(HardwareRaster {
            width: 4,
            height: 3,
            ..
        })
    ));

    #[cfg(unix)]
    {
        let link = root.join("link.png");
        std::os::unix::fs::symlink(path, &link).unwrap();
        let mut linked = request;
        linked.document.path = link;
        assert!(
            load_document(linked)
                .await
                .unwrap_err()
                .to_string()
                .contains("non-symlink")
        );
    }
    let _ = fs::remove_dir_all(root);
}
