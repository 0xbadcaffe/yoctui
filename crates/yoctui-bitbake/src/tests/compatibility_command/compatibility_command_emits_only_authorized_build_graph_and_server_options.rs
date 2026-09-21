use super::*;

#[test]
fn compatibility_command_emits_only_authorized_build_graph_and_server_options() {
    let authority = authority(
        4,
        &[
            (
                CapabilityId::BitBakeBuild,
                BITBAKE_BUILD_ARGV_IMPLEMENTATION,
            ),
            (
                CapabilityId::BitBakeForceTask,
                BITBAKE_FORCE_TASK_ARGV_IMPLEMENTATION,
            ),
            (
                CapabilityId::BitBakeGraphGeneration,
                BITBAKE_GRAPH_ARGV_IMPLEMENTATION,
            ),
            (
                CapabilityId::BitBakeServerStart,
                BITBAKE_SERVER_START_ARGV_IMPLEMENTATION,
            ),
            (
                CapabilityId::BitBakeDumpSig,
                BITBAKE_DUMPSIG_ARGV_IMPLEMENTATION,
            ),
            (
                CapabilityId::BitBakeDiffSigs,
                BITBAKE_DIFFSIGS_ARGV_IMPLEMENTATION,
            ),
        ],
    );
    let planner = planner(&authority);
    assert_eq!(
        planner
            .build(&BuildRequest {
                targets: vec!["busybox".into()],
                task: Some("compile".into()),
                force: true,
            })
            .unwrap()
            .arguments,
        ["-f", "-c", "compile", "busybox"]
    );
    assert_eq!(
        planner.dependency_graph("busybox").unwrap().arguments,
        ["-g", "busybox"]
    );
    assert_eq!(
        planner
            .server_control(BitBakeServerCommandOperation::Start)
            .unwrap()
            .arguments,
        ["--server-only"]
    );
    assert!(matches!(
        planner.server_control(BitBakeServerCommandOperation::Stop),
        Err(BitBakeCommandAuthorizationError::CapabilityMissing {
            capability: CapabilityId::BitBakeServerStop
        })
    ));
    assert_eq!(
        planner
            .signature_dump(Path::new("/work/build/one.sigdata"))
            .unwrap()
            .arguments,
        ["/work/build/one.sigdata"]
    );
    assert_eq!(
        planner
            .signature_compare(
                Path::new("/work/build/one.sigdata"),
                Path::new("/work/build/two.sigdata")
            )
            .unwrap()
            .arguments,
        [
            "-c",
            "never",
            "/work/build/one.sigdata",
            "/work/build/two.sigdata"
        ]
    );
}
