#[test]
fn ux_image_preview_rejection_is_transport_safe_and_keeps_candidate_out_of_binary() {
    use yoctui_model::{ImageArtifactKind, ImagePreviewFallback, ImagePreviewTransport};

    let workspace_manifest = include_str!("../../../../../Cargo.toml");
    assert!(
        !workspace_manifest
            .lines()
            .any(|line| line.trim_start().starts_with("ratatui-image =")),
        "a rejected optional renderer must not enter the shipped graph"
    );
    for transport in [
        ImagePreviewTransport::DirectTerminal,
        ImagePreviewTransport::Ssh,
        ImagePreviewTransport::Tmux,
        ImagePreviewTransport::SshThroughTmux,
        ImagePreviewTransport::TestBackend,
    ] {
        let decision = ImageArtifactKind::Wic.image_preview_decision(transport);
        assert!(!decision.native_terminal_graphics_enabled());
        assert_eq!(
            decision.fallback,
            ImagePreviewFallback::ExactArtifactMetadata
        );
        assert_eq!(decision.transport, transport);
    }
}
