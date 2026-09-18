use super::*;

pub(crate) struct SecurityCliFixture {
    pub(crate) root: PathBuf,
    pub(crate) build: PathBuf,
    pub(crate) reports: PathBuf,
    pub(crate) bin: PathBuf,
    pub(crate) provider: PathBuf,
}

impl SecurityCliFixture {
    pub(crate) fn new(mapper_body: &str) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_SECURITY_FIXTURE: AtomicU64 = AtomicU64::new(1);
        let root = std::env::temp_dir().join(format!(
            "yoctui-security-cli-{}-{}",
            std::process::id(),
            NEXT_SECURITY_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        let build = root.join("build");
        let reports = root.join("reports");
        let bin = root.join("bin");
        let layer = root.join("layer");
        for directory in [&build, &reports, &bin, &layer] {
            fs::create_dir_all(directory).unwrap();
        }
        let provider = layer.join("busybox.bb");
        fs::write(&provider, "SUMMARY = \"busybox\"\n").unwrap();
        let mapper = bin.join("cve-check-map-pkgs");
        fs::write(&mapper, mapper_body).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&mapper).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&mapper, permissions).unwrap();
        }
        Self {
            root,
            build,
            reports,
            bin,
            provider,
        }
    }

    pub(crate) fn app(&self) -> App {
        let mut app = App::new(20, 8_000);
        app.screen = Screen::Security;
        app.build.target = Some("core-image-minimal".into());
        app.workspace.build_dir = Some(self.build.clone());
        app.workspace.release = Some("6.0".into());
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        app.workspace
            .variables
            .insert("DISTRO".into(), "poky".into());
        app.workspace.variables.insert(
            "DEPLOY_DIR_IMAGE".into(),
            self.reports.display().to_string(),
        );
        app.workspace.recipes.push(yoctui_model::Recipe {
            name: "busybox".into(),
            file: Some(self.provider.clone()),
            ..yoctui_model::Recipe::default()
        });
        app.recipe_metadata.insert(
            "busybox".into(),
            yoctui_model::RecipeMetadata {
                recipe: "busybox".into(),
                tasks: Some(vec!["do_cve_check".into(), "do_create_recipe_sbom".into()]),
                ..yoctui_model::RecipeMetadata::default()
            },
        );
        app
    }

    pub(crate) fn write_cve(&self) -> PathBuf {
        let path = self.reports.join("busybox.cve.json");
        fs::write(
            &path,
            br#"{
                  "version": "1",
                  "packages": [{
                    "name": "busybox",
                    "version": "1.36",
                    "products": [{
                      "product": "busybox",
                      "cves": [{
                        "id": "CVE-2026-0001",
                        "status": "Unpatched",
                        "severity": "HIGH",
                        "mapping": {"source": "cve-check"}
                      }]
                    }]
                  }]
                }"#,
        )
        .unwrap();
        path
    }
}

impl Drop for SecurityCliFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
