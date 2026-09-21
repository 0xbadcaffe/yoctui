use super::*;

#[test]
fn ux_rootfs_protocol_rejects_unknown_variants_paths_bounds_and_schema() {
    let mut value = data();
    value.schema_version = 2;
    assert_eq!(
        value.validate(),
        Err(RootfsProtocolError::UnsupportedSchema(2))
    );
    value = data();
    if let RootfsAuthorityData::Available { records } = &mut value.filesystem_entries {
        records[0].kind = RootfsEntryKindData::Unknown;
    }
    assert_eq!(value.validate(), Err(RootfsProtocolError::InvalidRecord));
    if let RootfsAuthorityData::Available { records } = &mut value.filesystem_entries {
        records[0].kind = RootfsEntryKindData::RegularFile;
        records[0].path = "../escape".into();
    }
    assert_eq!(value.validate(), Err(RootfsProtocolError::InvalidRecord));

    value = data();
    value.limitations = vec!["x".into(); MAX_ROOTFS_WIRE_LIMITATIONS + 1];
    assert_eq!(
        value.validate(),
        Err(RootfsProtocolError::TooManyLimitations)
    );
}
