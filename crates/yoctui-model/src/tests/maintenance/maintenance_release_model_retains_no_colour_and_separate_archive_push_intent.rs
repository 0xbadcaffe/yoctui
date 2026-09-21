use super::*;

#[test]
fn maintenance_release_model_retains_no_colour_and_separate_archive_push_intent() {
    let comparison = BuildComparisonRequest::new(BuildComparisonRequest {
        repository: "/build/buildhistory".into(),
        from_revision: Some("HEAD^".into()),
        to_revision: Some("HEAD".into()),
        report_version: true,
        report_all: false,
        signatures: true,
        signature_diff: false,
        exclude_paths: vec!["images/*".into(), "images/*".into()],
        no_colour: true,
    })
    .unwrap();
    assert!(comparison.no_colour);
    assert_eq!(comparison.exclude_paths, vec!["images/*"]);

    let archive = GitArchiveRequest::new(GitArchiveRequest {
        data_dir: "/results".into(),
        git_dir: "/archives/release.git".into(),
        create: true,
        bare: true,
        create_tag: true,
        branch_name: "release/{machine}".into(),
        tag_name: Some("release/{tag_number}".into()),
        commit_subject: "release {commit}".into(),
        commit_body: "machine: {machine}".into(),
        tag_subject: "tag {tag_number}".into(),
        tag_body: "Yoctui release archive".into(),
        exclusions: vec!["tmp/*".into(), "tmp/*".into()],
        notes: vec![("release".into(), "/results/note.txt".into())],
        push_remote: Some("origin".into()),
    })
    .unwrap();
    assert!(MaintenanceOperation::GitArchive(archive.clone()).network_side_effect());
    let mut local = archive;
    local.push_remote = None;
    assert!(!MaintenanceOperation::GitArchive(local).network_side_effect());
}
