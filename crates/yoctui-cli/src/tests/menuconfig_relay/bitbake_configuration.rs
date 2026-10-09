use super::*;

#[test]
fn relay_configuration_roundtrips_through_real_bitbake_parser_when_available() {
    // Optional live check; ordinary CI does not have a Yocto checkout.
    let Some(library) = std::env::var_os("YOCTUI_TEST_BITBAKE_LIB") else {
        return;
    };
    let runtime = tempfile::tempdir().unwrap();
    let socket = runtime.path().join("relay.sock");
    let mut environment = std::collections::BTreeMap::new();
    configure_environment(
        &mut environment,
        Path::new("/opt/Roy's tools/yoctui"),
        &socket,
    );
    let (_directory, config) = configuration::create(&environment, &socket).unwrap();
    let output = Command::new("python3")
        .env("PYTHONPATH", library)
        .args([
            "-c",
            r#"import bb, bb.data, bb.parse, sys
d = bb.data.init()
d.setVar('TOPDIR', sys.argv[3])
d.setVar('BBPATH', sys.argv[3])
d.setVar('PATH', 'unchanged-host-path')
d.setVar('OVERRIDES', 'task-devshell')
bb.parse.handle(sys.argv[1], d)
assert d.getVar('OE_TERMINAL') == 'custom'
assert d.getVar('OE_TERMINAL_CUSTOMCMD') == sys.argv[2]
assert d.getVar('BB_NUMBER_PARSE_THREADS') == '2'
assert 'gnu-hosttools:' in d.getVar('PATH')
assert d.getVar('PATH').endswith('unchanged-host-path')
assert d.getVar('HOSTTOOLS_DIR') is None
"#,
        ])
        .arg(&config)
        .arg(&environment["OE_TERMINAL_CUSTOMCMD"])
        .arg(runtime.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "real BitBake parser rejected relay configuration: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
