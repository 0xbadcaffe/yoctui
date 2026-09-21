use super::*;

#[test]
fn ux_image_preview_policy_is_transport_invariant_and_never_fabricates_raster_authority() {
    let transports = [
        ImagePreviewTransport::DirectTerminal,
        ImagePreviewTransport::Ssh,
        ImagePreviewTransport::Tmux,
        ImagePreviewTransport::SshThroughTmux,
        ImagePreviewTransport::TestBackend,
    ];
    let kinds = [
        ImageArtifactKind::RootFilesystem,
        ImageArtifactKind::Kernel,
        ImageArtifactKind::Bootloader,
        ImageArtifactKind::Wic,
        ImageArtifactKind::Manifest,
        ImageArtifactKind::LicenseManifest,
        ImageArtifactKind::Spdx,
        ImageArtifactKind::Checksum,
        ImageArtifactKind::Other,
    ];
    for transport in transports {
        for kind in kinds {
            let decision = kind.image_preview_decision(transport);
            assert_eq!(decision.transport, transport);
            assert!(!decision.native_terminal_graphics_enabled());
            assert!(!decision.reason.is_empty());
        }
    }
    assert_eq!(
        ImageArtifactKind::RootFilesystem
            .image_preview_decision(ImagePreviewTransport::DirectTerminal)
            .fallback,
        ImagePreviewFallback::RootfsComposition
    );
}
