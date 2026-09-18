use super::*;

#[test]
fn attached_startup_uses_only_daemon_metadata() {
    assert_eq!(
        startup_metadata_authority(true),
        StartupMetadataAuthority::DaemonSnapshot
    );
    assert_eq!(
        startup_metadata_authority(false),
        StartupMetadataAuthority::OfflineFiles
    );
}
