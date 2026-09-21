use super::*;

#[test]
fn git_status_rename_origin_cannot_spoof_headers() {
    let status = parse_git_status(b"# branch.head master\0# branch.upstream origin/master\0# branch.ab +2 -3\x002 R. details\0# branch.ab +99 -99\0u UU details\0? strange\nname\0").unwrap();
    assert_eq!(
        (
            status.ahead,
            status.behind,
            status.staged,
            status.conflicts,
            status.untracked
        ),
        (2, 3, 1, 1, 1)
    );
    assert!(parse_git_status(b"# branch.ab rubbish\0").is_err());
}
