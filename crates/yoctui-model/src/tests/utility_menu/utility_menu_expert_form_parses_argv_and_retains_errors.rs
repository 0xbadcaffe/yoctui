use super::*;

#[test]
fn utility_menu_expert_form_parses_argv_and_retains_errors() {
    let mut form = ExpertArguments {
        input: "--name 'core image'".into(),
        ..Default::default()
    };
    assert_eq!(form.parse().unwrap(), ["--name", "core image"]);
    form.input = "'unterminated".into();
    assert!(form.parse().is_err());
    assert!(form.validation_error.is_some());
}
