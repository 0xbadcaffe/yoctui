use super::*;
use std::ffi::OsString;

#[test]
fn graphics_terminal_handoff_uses_readable_xterm_and_preserves_cli_arguments() {
    let command = graphics_terminal_command(
        Path::new("/usr/bin/xterm"),
        Path::new("/opt/yoctui/bin/yoctui"),
        [
            OsString::from("--build-dir"),
            OsString::from("/tmp/build dir"),
            OsString::from("attach"),
        ],
        (200, 60),
    );

    assert_eq!(command.get_program(), OsStr::new("/usr/bin/xterm"));
    assert_eq!(
        command.get_args().collect::<Vec<_>>(),
        [
            "-ti",
            "vt340",
            "-fa",
            "Monospace",
            "-fs",
            "14",
            "-geometry",
            "140x40",
            "-e",
            "/opt/yoctui/bin/yoctui",
            "--build-dir",
            "/tmp/build dir",
            "attach",
        ]
        .map(OsStr::new)
    );
    assert!(command.get_envs().any(|(name, value)| {
        name == OsStr::new(HANDOFF_ENV) && value == Some(OsStr::new("1"))
    }));
}

#[test]
fn graphics_terminal_handoff_keeps_laptop_geometry_bounded() {
    for (size, geometry) in [
        ((80, 24), "80x24"),
        ((100, 30), "100x30"),
        ((120, 28), "120x28"),
        ((0, 0), "80x24"),
    ] {
        let command = graphics_terminal_command(
            Path::new("/usr/bin/xterm"),
            Path::new("/opt/yoctui"),
            ["attach"],
            size,
        );
        let arguments = command.get_args().collect::<Vec<_>>();
        let index = arguments
            .iter()
            .position(|arg| *arg == OsStr::new("-geometry"))
            .unwrap();
        assert_eq!(arguments[index + 1], OsStr::new(geometry));
    }
}
