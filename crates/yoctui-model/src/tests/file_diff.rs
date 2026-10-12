use super::*;

fn compared(left: &str, right: &str) -> FileDiffComparison {
    compare_file_text(["left".into(), "right".into()], left, right).unwrap()
}

#[test]
fn file_diff_aligns_insertions_deletions_and_replacements() {
    let diff = compared("one\ntwo\nthree\nfour\n", "one\nnew\ntwo\nchanged\nfour\n");
    assert_eq!(diff.rows.len(), 5);
    assert!(diff.rows[0].equal);
    assert!(diff.rows[1].left.is_none());
    assert_eq!(diff.rows[1].right.as_ref().unwrap().text, "new");
    assert!(diff.rows[2].equal);
    assert_eq!(diff.rows[3].left.as_ref().unwrap().text, "three");
    assert_eq!(diff.rows[3].right.as_ref().unwrap().text, "changed");
    assert!(diff.rows[4].equal);
    assert_eq!(diff.changed_rows, 2);
}

#[test]
fn file_diff_preserves_every_line_in_ambiguous_and_empty_comparisons() {
    for left in ["", "a\n", "a\na\nb\n", "b\na\nb\n", "a", "a\r\n"] {
        for right in ["", "a\n", "a\na\nb\n", "b\na\nb\n", "a", "a\r\n"] {
            let diff = compared(left, right);
            for (index, input) in [left, right].iter().enumerate() {
                let cells: Vec<_> = diff
                    .rows
                    .iter()
                    .filter_map(|row| {
                        if index == 0 {
                            row.left.as_ref()
                        } else {
                            row.right.as_ref()
                        }
                    })
                    .collect();
                let lines: Vec<_> = input.split_inclusive('\n').collect();
                assert_eq!(cells.len(), lines.len());
                for (number, (cell, line)) in cells.iter().zip(lines).enumerate() {
                    assert_eq!(cell.number, number + 1);
                    assert_eq!(cell.text, line.strip_suffix('\n').unwrap_or(line));
                }
            }
            assert_eq!(diff.identical, left == right);
            assert_eq!(diff.changed_rows == 0, left == right);
        }
    }
}

#[test]
fn file_diff_newline_and_large_replace_regions_are_not_silently_ignored() {
    let diff = compared("same\n", "same");
    assert_eq!(diff.final_newline, [true, false]);
    assert_eq!(diff.changed_rows, 1);
    let diff = compared(&"a\n".repeat(10_000), &"b\n".repeat(10_000));
    assert_eq!(diff.rows.len(), 10_000);
    assert_eq!(diff.changed_rows, 10_000);
    assert!(
        compare_file_text(
            ["a".into(), "b".into()],
            &"\n".repeat(FILE_DIFF_MAX_LINES + 1),
            ""
        )
        .is_err()
    );
}

fn loaded_app() -> App {
    let mut app = App::new(16, 4096);
    update(&mut app, Action::Open(Screen::DiffTool));
    for pane in 0..2 {
        let Some(Effect::FileDiff(FileDiffRequest::Directory {
            pane: actual,
            request,
            directory,
        })) = file_diff_initial_request(&mut app, "/tmp".into())
        else {
            panic!("scan")
        };
        assert_eq!(pane, actual);
        update(
            &mut app,
            Action::FileDiff(FileDiffAction::DirectoryLoaded {
                pane,
                request,
                directory,
                result: Ok(vec![LayerBrowserEntry {
                    path: format!("/tmp/file {pane}.txt").into(),
                    is_dir: false,
                    ..Default::default()
                }]),
            }),
        );
    }
    app
}

#[test]
fn file_diff_marks_each_side_and_rejects_stale_completion_after_escape() {
    let mut app = loaded_app();
    assert_eq!(app.focus, FocusTarget::Workspace);
    update(&mut app, Action::FileDiff(FileDiffAction::Compare));
    assert!(app.file_diff.error.is_some());
    update(&mut app, Action::FileDiff(FileDiffAction::Mark));
    update(&mut app, Action::FileDiff(FileDiffAction::SwitchPane));
    update(&mut app, Action::FileDiff(FileDiffAction::Mark));
    let Some(Effect::FileDiff(FileDiffRequest::Compare { generation, paths })) =
        update(&mut app, Action::FileDiff(FileDiffAction::Compare))
    else {
        panic!("compare")
    };
    assert_eq!(
        paths,
        [
            PathBuf::from("/tmp/file 0.txt"),
            PathBuf::from("/tmp/file 1.txt")
        ]
    );
    update(&mut app, Action::FileDiff(FileDiffAction::Back));
    update(
        &mut app,
        Action::FileDiff(FileDiffAction::Compared {
            generation,
            result: Ok(compared("a", "b")),
        }),
    );
    assert!(app.file_diff.comparison.is_none());
    assert_eq!(app.focus, FocusTarget::Workspace);
    update(&mut app, Action::FileDiff(FileDiffAction::Back));
    assert_eq!(app.focus, FocusTarget::Navigator);
    assert!(app.file_diff.panes.iter().all(|pane| pane.marked.is_some()));
}

#[test]
fn file_diff_escape_returns_to_browsers_then_navigator_and_keeps_marks() {
    let mut app = loaded_app();
    app.file_diff.comparison = Some(compared("a", "b"));
    update(&mut app, Action::FileDiff(FileDiffAction::Back));
    assert!(app.file_diff.comparison.is_none());
    assert_eq!(app.focus, FocusTarget::Workspace);
    update(&mut app, Action::FileDiff(FileDiffAction::Back));
    assert_eq!(app.screen, Screen::DiffTool);
    assert_eq!(app.focus, FocusTarget::Navigator);
}
