use super::*;

#[test]
fn file_diff_reader_checks_regular_utf8_text_without_truncation() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("file with spaces.txt");
    fs::write(&path, "first\nsecond\n").unwrap();
    assert_eq!(read_text(&path).unwrap(), "first\nsecond\n");
    fs::write(&path, [b'a', 0, b'b']).unwrap();
    assert!(read_text(&path).unwrap_err().contains("binary"));
    fs::write(&path, [0xff]).unwrap();
    assert!(read_text(&path).unwrap_err().contains("UTF-8"));
    let file = fs::File::create(&path).unwrap();
    file.set_len(FILE_DIFF_MAX_BYTES as u64 + 1).unwrap();
    assert!(read_text(&path).unwrap_err().contains("2 MiB"));
    assert!(read_text(folder.path()).unwrap_err().contains("regular"));
}

#[tokio::test]
async fn file_diff_worker_delivers_real_files_and_handles_missing_paths() {
    let folder = tempfile::tempdir().unwrap();
    let paths = [folder.path().join("left"), folder.path().join("right")];
    fs::write(&paths[0], "same\nold\n").unwrap();
    fs::write(&paths[1], "same\nnew\n").unwrap();
    let mut app = App::new(16, 4096);
    app.file_diff.generation = 1;
    app.file_diff.comparing = true;
    let mut io = FileDiffIo::default();
    io.submit(Effect::FileDiff(FileDiffRequest::Compare {
        generation: 1,
        paths,
    }));
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while !io.poll(&mut app).await {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(app.file_diff.comparison.as_ref().unwrap().changed_rows, 1);
    assert!(!app.file_diff.comparing);
    let action = execute(FileDiffRequest::Directory {
        pane: 0,
        request: 2,
        directory: folder.path().join("missing"),
    });
    assert!(matches!(
        action,
        Action::FileDiff(FileDiffAction::DirectoryLoaded { result: Err(_), .. })
    ));
}
