use super::*;
use yoctui_model::{PtyDimensions, PtySession, PtySessionId, PtySessionSpec};

fn fixture() -> (PathBuf, PtyContextAuthority) {
    let nonce = std::time::SystemTime::UNIX_EPOCH
        .elapsed()
        .unwrap()
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("yoctui-pty-context-{}-{nonce}", std::process::id()));
    for directory in [
        "source", "build", "layer", "recipe", "devtool", "deploy", "sdk",
    ] {
        fs::create_dir_all(root.join(directory)).unwrap();
    }
    let shell = fs::canonicalize("/bin/sh").unwrap();
    let build_environment = VerifiedPtyEnvironment {
        identity: "build-env-7".into(),
        shell: shell.clone(),
        environment: BTreeMap::from([(
            "BUILDDIR".into(),
            root.join("build").display().to_string(),
        )]),
    };
    let sdk_environment = VerifiedPtyEnvironment {
        identity: "sdk-env-3".into(),
        shell,
        environment: BTreeMap::from([(
            "SDKTARGETSYSROOT".into(),
            root.join("sdk/sysroot").display().to_string(),
        )]),
    };
    let entry = |identity: &str, directory: &str| PtyContextEntry {
        identity: identity.into(),
        directory: root.join(directory),
    };
    let authority = PtyContextAuthority::new(
        "workspace-1".into(),
        root.join("source"),
        root.join("build"),
        build_environment,
        vec![entry("meta-test", "layer")],
        vec![entry("busybox", "recipe")],
        vec![entry("devtool:busybox", "devtool")],
        vec![entry("qemux86-64", "deploy")],
        vec![(entry("sdk-x86_64", "sdk"), sdk_environment)],
    )
    .unwrap();
    (root, authority)
}

mod pty_context_resolves_all_authoritative_routes_without_shell_strings;

mod pty_context_rejects_stale_identity_and_changed_path;

mod pty_context_rejects_duplicate_authority_and_untrusted_environment;
