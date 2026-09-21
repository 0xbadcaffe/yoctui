use super::*;

#[tokio::test]
async fn compatibility_probe_accepts_safe_same_directory_utility_symlink_without_losing_argv0() {
    use std::os::unix::fs::symlink;

    let fixture = Fixture::new("#!/bin/sh\necho modify\n");
    let target = fixture.root.join("devtool-real");
    fs::rename(&fixture.tool, &target).unwrap();
    symlink("devtool-real", &fixture.tool).unwrap();
    let observation = CapabilityProbeRunner::default()
        .probe(
            &fixture.context(),
            &CapabilityProbeSpec::Executable {
                tool: CapabilityToolId::Devtool,
            },
        )
        .await;
    assert_eq!(observation.status, CapabilityProbeStatus::Positive);
    assert_eq!(
        observation.evidence.argv,
        [fixture.tool.display().to_string()]
    );
}
