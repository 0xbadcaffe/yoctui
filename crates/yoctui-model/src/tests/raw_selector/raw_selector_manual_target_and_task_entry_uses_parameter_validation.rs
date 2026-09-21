use super::*;

#[test]
fn raw_selector_manual_target_and_task_entry_uses_parameter_validation() {
    let authority = RawSelectorAuthority::project(RawSelectorSources::default());
    for kind in [RawParameterKind::Target, RawParameterKind::Task] {
        let (command, parameter) = selector_command(kind);
        let selector = command.selector(&parameter, &authority).unwrap();
        assert!(selector.manual_entry);
        let valid = if kind == RawParameterKind::Target {
            "virtual/kernel"
        } else {
            "do_compile"
        };
        assert!(selector.parse_manual(valid).is_ok());
        assert!(selector.parse_manual("--option").is_err());
        assert!(selector.parse_manual("bad;command").is_err());
    }
}
