use super::*;

#[test]
fn firmware_candidates_are_bounded_to_single_native_targets() {
    let mut candidates = Vec::new();
    push_firmware_candidate(&mut candidates, "u-boot-fslc");
    push_firmware_candidate(&mut candidates, "u-boot-fslc");
    push_firmware_candidate(&mut candidates, "u-boot; rm -rf / ");
    push_firmware_candidate(&mut candidates, "${BOOTLOADER}");
    assert_eq!(candidates, vec!["u-boot-fslc"]);
}
