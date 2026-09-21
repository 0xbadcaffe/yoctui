use super::*;

#[test]
fn maintenance_release_archive_workspace_preserves_local_and_push_intent() {
    for remote in [None, Some("origin")] {
        let mut state = ready_state();
        state.view = MaintenanceView::Release;
        let transition = update_maintenance(&mut state, MaintenanceAction::OpenGitArchiveForm);
        let MaintenanceDialogUpdate::Open(dialog) = transition.dialog else {
            panic!("Git archive form did not open");
        };
        let MaintenanceDialog::GitArchiveToml { editor, .. } = *dialog else {
            panic!("wrong Git archive dialog");
        };
        assert_eq!(editor.selected_text(), Some(""));
        assert!(editor.text.contains("create = true"));
        assert!(editor.text.contains("create_tag = true"));
        assert!(editor.text.contains("bare = false"));
        let valid = update_maintenance(
            &mut state,
            MaintenanceAction::ConfirmGitArchiveToml(format!(
                "data_dir = \"/release/data\"\ngit_dir = \"/release/archive.git\"\ncreate = true\nbare = false\ncreate_tag = true\nbranch_name = \"release/{{machine}}\"\ntag_name = \"release/{{tag_number}}\"\ncommit_subject = \"Release {{commit}}\"\ncommit_body = \"\"\ntag_subject = \"Release tag {{tag_number}}\"\ntag_body = \"\"\nexclusions = \"tmp/*,downloads/*,tmp/*\"\nnotes = \"release=/release/note.txt\"\npush_remote = \"{}\"\n",
                remote.unwrap_or_default()
            )),
        );
        assert!(matches!(
            valid.effect,
            Some(MaintenanceEffect::PreviewGitArchive {
                capability_request: 1,
                request: GitArchiveRequest {
                    exclusions,
                    notes,
                    push_remote,
                    ..
                },
            }) if exclusions == vec!["downloads/*", "tmp/*"]
                && notes == vec![("release".into(), PathBuf::from("/release/note.txt"))]
                && push_remote.as_deref() == remote
        ));
        assert_eq!(valid.dialog, MaintenanceDialogUpdate::Close);
    }
}
