use super::*;

#[test]
fn folder_browser_lists_only_sorted_directories_including_hidden_and_spaces() {
    let (_temp, root, _) = fixture();
    for name in ["z", "folder with spaces", ".hidden", "a"] {
        fs::create_dir(root.join(name)).unwrap();
    }
    fs::write(root.join("z/nested-file"), "text").unwrap();
    let W::FoldersBrowsed {
        generation: 7,
        result: Ok((directory, entries)),
    } = execute(R::BrowseFolders {
        path: root.clone(),
        generation: 7,
    })
    else {
        panic!("directory scan failed")
    };
    assert_eq!(directory, root);
    assert_eq!(
        entries,
        [".hidden", "a", "folder with spaces", "z"].map(|name| root.join(name))
    );
}
#[test]
fn folder_browser_reports_missing_directory_and_regular_file_errors() {
    let (_temp, root, file) = fixture();
    for path in [root.join("missing"), file, "relative".into()] {
        assert!(matches!(
            execute(R::BrowseFolders {
                path,
                generation: 8
            }),
            W::FoldersBrowsed {
                generation: 8,
                result: Err(_)
            }
        ));
    }
}
