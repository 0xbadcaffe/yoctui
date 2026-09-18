use super::*;

pub(crate) fn config_edit_request(build_dir: &Path, name: &str, value: &str) -> ConfigEditRequest {
    ConfigEditRequest {
        identity: VariableIdentity {
            name: name.into(),
            recipe: None,
        },
        value: value.into(),
        destination: build_dir.join("conf/local.conf"),
        assignment: yoctui_model::config_edit_assignment(name, value).unwrap(),
    }
}
