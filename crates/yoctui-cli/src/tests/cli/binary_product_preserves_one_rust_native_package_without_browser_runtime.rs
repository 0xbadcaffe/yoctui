use super::*;

#[test]
fn binary_product_preserves_one_rust_native_package_without_browser_runtime() {
    let manifest =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.toml")).unwrap();
    assert!(manifest.contains("members"));
    assert!(!manifest.to_ascii_lowercase().contains("electron"));
    assert!(!manifest.to_ascii_lowercase().contains("browser runtime"));
    assert!(matches!(
        Cli::try_parse_from(["yoctui", "daemon", "foreground"])
            .unwrap()
            .command,
        Some(Command::Daemon {
            command: DaemonCliCommand::Foreground
        })
    ));
}
