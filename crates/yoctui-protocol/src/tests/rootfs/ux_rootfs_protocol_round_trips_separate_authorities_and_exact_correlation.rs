use super::*;

#[test]
fn ux_rootfs_protocol_round_trips_separate_authorities_and_exact_correlation() {
    let value = data();
    value.validate().unwrap();
    let encoded = serde_json::to_vec(&value).unwrap();
    let decoded: RootfsCompositionData = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(decoded, value);
    assert_eq!(decoded.request.generation, 9);
    assert_eq!(decoded.request.image.image, "core-image-minimal");
}
