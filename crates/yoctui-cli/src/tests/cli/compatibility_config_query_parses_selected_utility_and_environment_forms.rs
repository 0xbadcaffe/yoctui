use super::*;

#[test]
fn compatibility_config_query_parses_selected_utility_and_environment_forms() {
    assert_eq!(
        config_value_from_authorized_output(
            "MACHINE",
            yoctui_bitbake::BITBAKE_GETVAR_UTILITY_IMPLEMENTATION,
            "qemux86-64\n",
        )
        .unwrap(),
        "qemux86-64"
    );
    assert_eq!(
        config_value_from_authorized_output(
            "MACHINE",
            yoctui_bitbake::BITBAKE_GETVAR_ENVIRONMENT_IMPLEMENTATION,
            "# history\nMACHINE=\"old\"\nMACHINE=\"qemuarm64\"\n",
        )
        .unwrap(),
        "qemuarm64"
    );
    assert!(
        config_value_from_authorized_output(
            "MACHINE",
            yoctui_bitbake::BITBAKE_GETVAR_ENVIRONMENT_IMPLEMENTATION,
            "DISTRO=\"poky\"\n",
        )
        .unwrap_err()
        .to_string()
        .contains("absent")
    );
}
