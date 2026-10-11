use super::*;

#[tokio::test]
async fn compatibility_devtool_help_reuses_verified_bbpath_without_a_shell() {
    let fixture =
        Fixture::new("#!/bin/sh\nprintf '%s\\n' \"$@\" > argv\necho 'usage: finish --force'\n");
    let context = fixture
        .context()
        .with_devtool_bbpath("/build:/layers/meta with spaces".into())
        .unwrap();
    let probe = CapabilityProbeSpec::CommandHelp {
        tool: CapabilityToolId::Devtool,
        subcommand: Some("finish".into()),
    };
    let result = CapabilityProbeRunner::default()
        .probe(&context, &probe)
        .await;
    assert_eq!(result.status, CapabilityProbeStatus::Positive);
    assert_eq!(
        fs::read_to_string(fixture.root.join("argv")).unwrap(),
        "--bbpath\n/build:/layers/meta with spaces\nfinish\n--help\n"
    );
    assert_eq!(
        &result.evidence.argv[1..],
        [
            "--bbpath",
            "/build:/layers/meta with spaces",
            "finish",
            "--help"
        ]
    );
    assert!(fixture.context().with_devtool_bbpath("\0".into()).is_err());
}

#[tokio::test]
async fn compatibility_command_cache_reuses_output_but_checks_each_requirement() {
    let fixture = Fixture::new("#!/bin/sh\necho run >> calls\necho 'modify --force'\n");
    let context = fixture.context();
    let runner = CapabilityProbeRunner::default().with_command_cache();
    let yes = CapabilityProbeSpec::CommandOption {
        tool: CapabilityToolId::Devtool,
        subcommand: None,
        option: "--force".into(),
    };
    let no = CapabilityProbeSpec::CommandOption {
        tool: CapabilityToolId::Devtool,
        subcommand: None,
        option: "--missing".into(),
    };
    let (a, b) = tokio::join!(runner.probe(&context, &yes), runner.probe(&context, &no));
    assert_eq!(a.status, CapabilityProbeStatus::Positive);
    assert_eq!(b.status, CapabilityProbeStatus::Negative);
    assert_eq!(
        fs::read_to_string(fixture.root.join("calls"))
            .unwrap()
            .lines()
            .count(),
        1
    );
    let fresh = runner.with_command_cache();
    assert_eq!(
        fresh.probe(&context, &yes).await.status,
        CapabilityProbeStatus::Positive
    );
    assert_eq!(
        fs::read_to_string(fixture.root.join("calls"))
            .unwrap()
            .lines()
            .count(),
        2
    );
}
