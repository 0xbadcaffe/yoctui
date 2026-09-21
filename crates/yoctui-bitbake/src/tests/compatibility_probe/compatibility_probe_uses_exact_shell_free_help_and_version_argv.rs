use super::*;

#[tokio::test]
async fn compatibility_probe_uses_exact_shell_free_help_and_version_argv() {
    let fixture = Fixture::new(
        "#!/bin/sh\nprintf '%s\\n' \"$@\" >> probe.argv\ncase \"$*\" in\n  *--version*) echo 'devtool 1.0' ;;\n  *) echo 'modify upgrade --force' ;;\nesac\n",
    );
    let context = fixture.context();
    let runner = CapabilityProbeRunner::default();
    let help = runner
        .probe(
            &context,
            &CapabilityProbeSpec::CommandHelp {
                tool: CapabilityToolId::Devtool,
                subcommand: Some("upgrade".into()),
            },
        )
        .await;
    assert_eq!(help.status, CapabilityProbeStatus::Positive);
    assert_eq!(help.evidence.argv[1..], ["upgrade", "--help"]);
    let version = runner
        .probe(
            &context,
            &CapabilityProbeSpec::CommandVersion {
                tool: CapabilityToolId::Devtool,
            },
        )
        .await;
    assert_eq!(version.status, CapabilityProbeStatus::Positive);
    assert_eq!(version.evidence.argv[1..], ["--version"]);
    assert_eq!(
        fs::read_to_string(fixture.root.join("probe.argv")).unwrap(),
        "upgrade\n--help\n--version\n"
    );
}
