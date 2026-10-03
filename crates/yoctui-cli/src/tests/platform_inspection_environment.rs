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

    fn metadata_script(&self, module: Option<&str>) {
        let library = self.source.join("bitbake/lib");
        fs::create_dir_all(&library).unwrap();
        if let Some(content) = module {
            fs::create_dir_all(library.join("bb")).unwrap();
            fs::write(library.join("bb/__init__.py"), content).unwrap();
        }
        Self::script(
            &self.source,
            &format!(
                "export BUILDDIR=\"$1\"\nexport PYTHONPATH=\"{}\"",
                library.display(),
            ),
        );
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

#[tokio::test]
async fn metadata_environment_supplies_selected_bb_to_the_real_child_only() {
    let fixture = Fixture::new();
    fixture.metadata_script(Some("SELECTED = 'selected-sibling'\n"));
    let environment = super::super::metadata_backend::initialized_metadata_environment(
        &fixture.build,
        Some(&fixture.source),
    )
    .await
    .unwrap();
    assert_eq!(
        environment.get("BUILDDIR"),
        Some(&fixture.build.canonicalize().unwrap().display().to_string())
    );
    let output = tokio::process::Command::new("python3")
        .args(["-S", "-c", "import bb; print(bb.SELECTED)"])
        .envs(environment)
        .current_dir(&fixture.build)
        .output()
        .await
        .unwrap();
    assert!(output.status.success(), "{:?}", output.stderr);
    assert_eq!(output.stdout, b"selected-sibling\n");
}

#[tokio::test]
async fn metadata_environment_does_not_invent_missing_bb_availability() {
    let fixture = Fixture::new();
    fixture.metadata_script(None);
    let environment = super::super::metadata_backend::initialized_metadata_environment(
        &fixture.build,
        Some(&fixture.source),
    )
    .await
    .unwrap();
    let output = tokio::process::Command::new("python3")
        .args(["-S", "-c", "import bb"])
        .envs(environment)
        .current_dir(&fixture.build)
        .output()
        .await
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("No module named 'bb'"));
}

#[tokio::test]
async fn metadata_environment_reports_initializer_failure_without_fallback() {
    let fixture = Fixture::new();
    Fixture::script(&fixture.root, "export BUILDDIR=\"$1\"");
    Fixture::script(
        &fixture.source,
        "echo selected-initializer-failed >&2\nreturn 13",
    );
    let error = super::super::metadata_backend::initialized_metadata_environment(
        &fixture.build,
        Some(&fixture.source),
    )
    .await
    .expect_err("selected initializer errors must not use a working ancestor");
    assert!(format!("{error:#}").contains("selected-initializer-failed"));
}

#[tokio::test]
async fn metadata_environment_cancellation_reaps_the_owned_initializer() {
    let fixture = Fixture::new();
    let marker = fixture.source.join("initializer.pid");
    Fixture::script(
        &fixture.source,
        &format!("echo $$ > \"{}\"\nsleep 30", marker.display(),),
    );
    let source = fixture.source.clone();
    let build = fixture.build.clone();
    let worker = tokio::spawn(async move {
        super::super::metadata_backend::initialized_metadata_environment(&build, Some(&source))
            .await
    });
    tokio::time::timeout(Duration::from_secs(3), async {
        while !marker.exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("fake initializer did not start");
    let pid: u32 = fs::read_to_string(&marker).unwrap().trim().parse().unwrap();
    worker.abort();
    assert!(worker.await.unwrap_err().is_cancelled());
    tokio::time::timeout(Duration::from_secs(3), async {
        while PathBuf::from(format!("/proc/{pid}")).exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("cancelled initializer remained alive");
}
