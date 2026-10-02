use super::*;
use std::os::unix::fs::PermissionsExt;
use std::sync::atomic::{AtomicU64, Ordering};

struct Fixture {
    root: PathBuf,
    source: PathBuf,
    build: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = env::temp_dir().join(format!(
            "yoctui-platform-profile-{}-{}-{}",
            std::process::id(),
            yoctui_utils::unix_ms(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let source = root.join("selected-source");
        let build = root.join("build/romulus");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(build.join("conf")).unwrap();
        fs::write(build.join("conf/local.conf"), "MACHINE = \"romulus\"\n").unwrap();
        fs::write(build.join("conf/bblayers.conf"), "BBLAYERS = \"\"\n").unwrap();
        Self {
            root,
            source,
            build,
        }
    }

    fn script(directory: &Path, body: &str) {
        let script = directory.join("oe-init-build-env");
        fs::write(&script, format!("#!/usr/bin/env bash\n{body}\n")).unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

#[tokio::test]
async fn platform_inspection_reconstructs_the_selected_build_environment() {
    let fixture = Fixture::new();
    Fixture::script(
        &fixture.root,
        "export BUILDDIR=\"$1\"\nexport YOCTUI_PLATFORM_INSPECTION_TEST=ready",
    );
    let environment = initialized_platform_environment(&fixture.build, None)
        .await
        .unwrap();
    assert_eq!(
        environment.get("YOCTUI_PLATFORM_INSPECTION_TEST"),
        Some(&"ready".to_owned())
    );
    assert_eq!(
        environment.get("BUILDDIR"),
        Some(&fixture.build.display().to_string())
    );
}

#[tokio::test]
async fn platform_inspection_uses_selected_sibling_source_not_ancestor_initializer() {
    let fixture = Fixture::new();
    Fixture::script(
        &fixture.root,
        "export BUILDDIR=\"$1\"\nexport YOCTUI_PLATFORM_INSPECTION_TEST=wrong-ancestor",
    );
    Fixture::script(
        &fixture.source,
        "export BUILDDIR=\"$1\"\nexport YOCTUI_PLATFORM_INSPECTION_TEST=selected-sibling",
    );
    let environment = initialized_platform_environment(&fixture.build, Some(&fixture.source))
        .await
        .unwrap();
    assert_eq!(
        environment.get("YOCTUI_PLATFORM_INSPECTION_TEST"),
        Some(&"selected-sibling".to_owned())
    );
    assert_eq!(
        environment.get("BUILDDIR"),
        Some(&fixture.build.canonicalize().unwrap().display().to_string())
    );
}

#[tokio::test]
async fn platform_inspection_missing_selected_source_does_not_fall_back_to_ancestor() {
    let fixture = Fixture::new();
    Fixture::script(&fixture.root, "export BUILDDIR=\"$1\"");
    let error = initialized_platform_environment(&fixture.build, Some(&fixture.source))
        .await
        .expect_err("an explicit source without its initializer must fail closed");
    assert!(
        error
            .to_string()
            .contains("cannot locate oe-init-build-env")
    );
}

#[tokio::test]
async fn platform_inspection_rejects_initializer_switching_the_build_directory() {
    let fixture = Fixture::new();
    Fixture::script(&fixture.source, "export BUILDDIR=\"$PWD\"");
    let error = initialized_platform_environment(&fixture.build, Some(&fixture.source))
        .await
        .expect_err("initializer must not redirect inspection to a different build");
    assert!(error.to_string().contains("different BUILDDIR"));
}

#[tokio::test]
async fn platform_inspection_reports_a_missing_selected_profile() {
    let fixture = Fixture::new();
    let error = initialized_platform_environment(&fixture.root.join("missing"), None)
        .await
        .expect_err("missing build profile must fail");
    assert!(
        error
            .to_string()
            .contains("cannot locate oe-init-build-env")
    );
}

#[tokio::test]
async fn platform_inspection_rejects_uninitialized_build_with_selected_source() {
    let fixture = Fixture::new();
    Fixture::script(&fixture.source, "export BUILDDIR=\"$1\"");
    fs::remove_file(fixture.build.join("conf/bblayers.conf")).unwrap();
    let error = initialized_platform_environment(&fixture.build, Some(&fixture.source))
        .await
        .expect_err("an initializer alone is not an initialized build");
    assert!(
        error
            .to_string()
            .contains("cannot locate oe-init-build-env")
    );
}
