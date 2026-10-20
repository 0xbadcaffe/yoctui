use super::*;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};

fn configured(executable: &str, socket: &Path) -> std::collections::BTreeMap<String, String> {
    let mut environment = std::collections::BTreeMap::new();
    configure_environment(&mut environment, Path::new(executable), socket);
    environment
}

#[test]
fn changing_the_relay_socket_does_not_change_bitbake_configuration_or_timestamp() {
    let runtime = tempfile::tempdir().unwrap();
    let first_socket = runtime.path().join("first.sock");
    let second_socket = runtime.path().join("second.sock");
    let first = configured("/opt/yoctui", &first_socket);
    let second = configured("/opt/yoctui", &second_socket);
    let config = configuration::create(&first, &first_socket).unwrap();
    let before = fs::metadata(&config).unwrap();
    let contents = fs::read_to_string(&config).unwrap();
    assert_eq!(
        configuration::create(&second, &second_socket).unwrap(),
        config
    );
    let after = fs::metadata(&config).unwrap();
    assert_eq!(before.modified().unwrap(), after.modified().unwrap());
    assert_eq!(before.ino(), after.ino());
    assert_eq!(fs::read_to_string(&config).unwrap(), contents);
    assert_eq!(
        first["OE_TERMINAL_CUSTOMCMD"],
        second["OE_TERMINAL_CUSTOMCMD"]
    );
    assert_ne!(first[SOCKET_ENV], second[SOCKET_ENV]);
    assert!(!first["BB_ENV_PASSTHROUGH_ADDITIONS"].contains(SOCKET_ENV));
    assert!(!contents.contains("BB_HASHCONFIG_IGNORE_VARS"));
    assert!(!contents.contains("BB_BASEHASH_IGNORE_VARS"));
    assert!(!first["OE_TERMINAL_CUSTOMCMD"].contains("--socket"));
}

#[test]
fn configuration_is_private_and_program_specific() {
    let runtime = tempfile::tempdir().unwrap();
    let socket = runtime.path().join("relay.sock");
    let config = configuration::create(&configured("/opt/first/yoctui", &socket), &socket).unwrap();
    let other = configuration::create(&configured("/opt/second/yoctui", &socket), &socket).unwrap();
    assert_ne!(config, other);
    assert_eq!(
        fs::metadata(&config).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        fs::metadata(config.parent().unwrap())
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
}

#[test]
fn configuration_refuses_symlinks_instead_of_overwriting_other_files() {
    let runtime = tempfile::tempdir().unwrap();
    let socket = runtime.path().join("relay.sock");
    let environment = configured("/opt/yoctui", &socket);
    let config = configuration::create(&environment, &socket).unwrap();
    let outside = runtime.path().join("unrelated.conf");
    fs::write(&outside, "preserve me").unwrap();
    fs::remove_file(&config).unwrap();
    symlink(&outside, &config).unwrap();
    assert!(configuration::create(&environment, &socket).is_err());
    assert_eq!(fs::read_to_string(outside).unwrap(), "preserve me");
}

#[test]
fn configuration_refuses_a_symlinked_or_public_cache_directory() {
    let runtime = tempfile::tempdir().unwrap();
    let socket = runtime.path().join("relay.sock");
    let environment = configured("/opt/yoctui", &socket);
    let config = configuration::create(&environment, &socket).unwrap();
    let directory = config.parent().unwrap();
    fs::set_permissions(directory, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(configuration::create(&environment, &socket).is_err());
    fs::set_permissions(directory, fs::Permissions::from_mode(0o700)).unwrap();
    let moved = runtime.path().join("moved");
    fs::rename(directory, &moved).unwrap();
    symlink(moved, directory).unwrap();
    assert!(configuration::create(&environment, &socket).is_err());
}

#[test]
fn stale_configuration_content_is_replaced_atomically() {
    let runtime = tempfile::tempdir().unwrap();
    let socket = runtime.path().join("relay.sock");
    let environment = configured("/opt/yoctui", &socket);
    let config = configuration::create(&environment, &socket).unwrap();
    let contents = fs::read(&config).unwrap();
    fs::write(&config, "stale").unwrap();
    assert_eq!(
        configuration::create(&environment, &socket).unwrap(),
        config
    );
    assert_eq!(fs::read(&config).unwrap(), contents);
}

#[test]
fn relay_environment_is_idempotent_and_preserves_unrelated_build_inputs() {
    let runtime = tempfile::tempdir().unwrap();
    let socket = runtime.path().join("relay.sock");
    let mut environment = configured("/opt/Roy's tools/yoctui", &socket);
    environment.insert("MACHINE".into(), "qemux86-64".into());
    environment.insert("DISTRO".into(), "poky".into());
    let before = environment.clone();
    configure_environment(
        &mut environment,
        Path::new("/opt/Roy's tools/yoctui"),
        &socket,
    );
    assert_eq!(environment, before);
}

#[test]
fn real_bitbake_hash_ignores_only_runtime_socket_not_recipe_configuration() {
    let Some(library) = std::env::var_os("YOCTUI_TEST_BITBAKE_LIB") else {
        return;
    };
    let runtime = tempfile::tempdir().unwrap();
    let socket = runtime.path().join("first.sock");
    let environment = configured("/opt/yoctui", &socket);
    let config = configuration::create(&environment, &socket).unwrap();
    let output = Command::new("python3")
        .env("PYTHONPATH", library)
        .args([
            "-c",
            r#"
import bb, bb.data, bb.parse, sys
def data(socket):
    d = bb.data.init()
    d.setVar('TOPDIR', sys.argv[2])
    d.setVar('BBPATH', sys.argv[2])
    d.setVar('BB_HASHCONFIG_IGNORE_VARS', 'BB_ORIGENV')
    original = bb.data.init()
    original.setVar('YOCTUI_MENUCONFIG_SOCKET', socket)
    d.setVar('BB_ORIGENV', original)
    bb.parse.handle(sys.argv[1], d)
    return d
first, second = data('/first.sock'), data('/second.sock')
assert first.get_hash() == second.get_hash()
assert second.getVar('BB_ORIGENV', False).getVar('YOCTUI_MENUCONFIG_SOCKET') == '/second.sock'
second.setVar('MACHINE', 'changed-machine')
assert first.get_hash() != second.get_hash()
print('stable-config-hash; runtime socket fresh; MACHINE still invalidates cache')
"#,
        ])
        .arg(config)
        .arg(runtime.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
