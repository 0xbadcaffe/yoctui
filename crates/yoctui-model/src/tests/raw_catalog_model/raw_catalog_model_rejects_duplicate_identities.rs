use super::*;

#[test]
fn raw_catalog_model_rejects_duplicate_identities() {
    let mut duplicate_command = valid_catalog();
    duplicate_command
        .commands
        .push(duplicate_command.commands[0].clone());
    assert_eq!(
        duplicate_command.validate(),
        Err(RawCatalogError::DuplicateCommand(id("task-control.run")))
    );

    let mut duplicate_parameter = valid_catalog();
    let repeated = duplicate_parameter.commands[0].parameters[0].clone();
    duplicate_parameter.commands[0].parameters.push(repeated);
    assert_eq!(
        duplicate_parameter.validate(),
        Err(RawCatalogError::InvalidParameters(id("task-control.run")))
    );
}
