use super::*;

#[test]
fn test_runner_commands_are_exact_revalidated_and_child_environment_only() {
    let (_directory, adapter) = fixture("commands");
    let oe = request(&adapter, TestFamily::OeSelftest);
    let oe_command = adapter.command(&oe).unwrap();
    assert_eq!(
        oe_command.arguments(),
        ["-r", "tinfoil.Case.test_one", "-j", "4"]
    );
    assert!(oe_command.environment().is_empty());

    let bitbake = request(&adapter, TestFamily::BitbakeSelftest);
    let command = adapter.command(&bitbake).unwrap();
    assert_eq!(command.arguments(), ["-v"]);
    assert_eq!(
        command.environment().get(OsStr::new("BB_SKIP_NETTESTS")),
        Some(&OsString::from("yes"))
    );
    assert!(std::env::var_os("BB_SKIP_NETTESTS").is_none());

    let mut invalid = oe;
    invalid.skip_network = true;
    assert!(matches!(
        adapter.command(&invalid),
        Err(TestRunnerAdapterError::InvalidRequest(_))
    ));
}
