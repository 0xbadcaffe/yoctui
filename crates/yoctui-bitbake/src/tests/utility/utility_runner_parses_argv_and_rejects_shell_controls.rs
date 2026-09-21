use super::*;

#[test]
fn utility_runner_parses_argv_and_rejects_shell_controls() {
    assert_eq!(
        parse_utility_arguments("--name 'core image' --flag").unwrap(),
        vec!["--name", "core image", "--flag"]
    );
    assert!(parse_utility_arguments("echo; rm -rf /").is_ok());
    assert!(parse_utility_arguments("unterminated\"").is_err());
}
