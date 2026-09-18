use super::*;

#[test]
fn interactive_daemon_logging_keeps_stderr_out_of_terminal_sessions() {
    for args in [
        vec!["yoctui"],
        vec!["yoctui", "attach"],
        vec!["yoctui", "build", "image"],
    ] {
        assert!(uses_interactive_terminal(
            &Cli::try_parse_from(args).unwrap()
        ));
    }
    for args in [
        vec!["yoctui", "--headless"],
        vec!["yoctui", "doctor"],
        vec!["yoctui", "inspect"],
        vec!["yoctui", "daemon", "status"],
    ] {
        assert!(!uses_interactive_terminal(
            &Cli::try_parse_from(args).unwrap()
        ));
    }
}
