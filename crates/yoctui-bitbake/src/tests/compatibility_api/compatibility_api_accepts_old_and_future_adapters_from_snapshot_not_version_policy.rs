use super::*;

#[test]
fn compatibility_api_accepts_old_and_future_adapters_from_snapshot_not_version_policy() {
    let requirements = [
        (
            CapabilityId::BitBakeWorkspaceInspection,
            "tinfoil.adapter.legacy",
        ),
        (CapabilityId::BitBakeBuild, "tinfoil.adapter.legacy"),
        (CapabilityId::BitBakeNativeEvents, "tinfoil.adapter.legacy"),
    ];
    let mut old = BitBakeApiAuthority::new(
        authority(4, "1.52", &requirements),
        4,
        Path::new("/work/build"),
    )
    .unwrap();
    negotiate_all(&mut old);
    old.require(BitBakeApiOperation::Workspace).unwrap();
    old.require(BitBakeApiOperation::Build).unwrap();

    let future_requirements = [
        (
            CapabilityId::BitBakeWorkspaceInspection,
            "tinfoil.workspace",
        ),
        (CapabilityId::BitBakeNativeEvents, "tinfoil.native_events"),
    ];
    let mut future = BitBakeApiAuthority::new(
        authority(5, "99.0", &future_requirements),
        5,
        Path::new("/work/build"),
    )
    .unwrap();
    negotiate_all(&mut future);
    future.require(BitBakeApiOperation::Workspace).unwrap();
    future.require(BitBakeApiOperation::NativeEvents).unwrap();
}
