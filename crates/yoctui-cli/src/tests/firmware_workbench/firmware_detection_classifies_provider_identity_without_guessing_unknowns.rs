use super::*;

#[test]
fn firmware_detection_classifies_provider_identity_without_guessing_unknowns() {
    assert_eq!(
        classify_firmware_component(
            "virtual/bootloader",
            Some(Path::new("/layers/u-boot/u-boot_2026.bb")),
            None,
            None,
        ),
        PlatformComponent::UBoot
    );
    assert_eq!(
        classify_firmware_component("ovmf", None, None, Some("ovmf")),
        PlatformComponent::BiosUefi
    );
    assert_eq!(
        classify_firmware_component("virtual/bootloader", None, None, None),
        PlatformComponent::BootFirmware
    );
}
