use super::*;

#[test]
fn raw_catalog_model_rejects_oversized_records() {
    let mut catalog = valid_catalog();
    catalog.commands[0].description = "x".repeat(MAX_RAW_TEXT_BYTES + 1);
    assert_eq!(
        catalog.validate(),
        Err(RawCatalogError::InvalidCommand(id("task-control.run")))
    );

    let invalid = RawCommandId::new("x".repeat(MAX_RAW_ID_BYTES + 1));
    assert!(matches!(
        invalid,
        Err(RawCatalogError::InvalidIdentity {
            field: "command",
            ..
        })
    ));
}
