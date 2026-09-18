use super::*;

pub(crate) struct QaCliFixture {
    pub(crate) root: PathBuf,
    pub(crate) build: PathBuf,
    pub(crate) reports: PathBuf,
    pub(crate) bin: PathBuf,
    pub(crate) provider: PathBuf,
    pub(crate) layer: PathBuf,
    pub(crate) source: PathBuf,
}

impl QaCliFixture {
    pub(crate) fn new(layer_runner_body: &str) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_QA_FIXTURE: AtomicU64 = AtomicU64::new(1);
        let root = std::env::temp_dir().join(format!(
            "yoctui-qa-cli-{}-{}",
            std::process::id(),
            NEXT_QA_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        let build = root.join("build");
        let reports = build.join("qa-reports");
        let bin = root.join("bin");
        let layer = root.join("meta-demo");
        let recipes = layer.join("recipes-core/busybox");
        for directory in [&build, &reports, &bin, &layer, &recipes] {
            fs::create_dir_all(directory).unwrap();
        }
        let provider = recipes.join("busybox_1.0.bb");
        fs::write(&provider, "SUMMARY = \"BusyBox\"\n").unwrap();
        let source = layer.join("conf/layer.conf");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::write(&source, "LAYERSERIES_COMPAT_meta-demo = \"scarthgap\"\n").unwrap();
        let runner = bin.join("yocto-check-layer");
        fs::write(&runner, layer_runner_body).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&runner).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&runner, permissions).unwrap();
        }
        Self {
            root: fs::canonicalize(root).unwrap(),
            build: fs::canonicalize(build).unwrap(),
            reports: fs::canonicalize(reports).unwrap(),
            bin: fs::canonicalize(bin).unwrap(),
            provider: fs::canonicalize(provider).unwrap(),
            layer: fs::canonicalize(layer).unwrap(),
            source: fs::canonicalize(source).unwrap(),
        }
    }

    pub(crate) fn app(&self) -> App {
        let mut app = App::new(20, 8_000);
        app.screen = Screen::Qa;
        app.workspace.build_dir = Some(self.build.clone());
        app.workspace.release = Some("6.0".into());
        app.workspace.variables.insert(
            "PACKAGE_QA_REPORT_ROOT".into(),
            self.reports.display().to_string(),
        );
        app.workspace.variables.insert(
            "YOCTO_CHECK_LAYER_REPORT_ROOT".into(),
            self.reports.display().to_string(),
        );
        app.workspace.recipes.push(yoctui_model::Recipe {
            name: "busybox".into(),
            file: Some(self.provider.clone()),
            ..yoctui_model::Recipe::default()
        });
        app.workspace.layers.push(yoctui_model::Layer {
            name: "meta-demo".into(),
            path: self.layer.clone(),
            priority: Some(6),
        });
        app.recipe_metadata.insert(
            "busybox".into(),
            yoctui_model::RecipeMetadata {
                recipe: "busybox".into(),
                tasks: Some(vec![
                    "do_checkuri".into(),
                    "do_patch_qa".into(),
                    "do_populate_lic".into(),
                    "do_package_qa".into(),
                ]),
                ..yoctui_model::RecipeMetadata::default()
            },
        );
        app
    }

    pub(crate) fn write_report(&self) -> PathBuf {
        let report = self.reports.join("recipe-qa.json");
        fs::write(
                &report,
                format!(
                    r#"{{"findings":[{{"status":"warning","severity":"warning","message":"license checksum needs review","rule":"license-checksum","source":{{"path":"{}","line":1}}}}]}}"#,
                    self.source.display()
                ),
            )
            .unwrap();
        fs::canonicalize(report).unwrap()
    }
}

impl Drop for QaCliFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
