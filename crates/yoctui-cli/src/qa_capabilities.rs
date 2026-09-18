//! Qa capabilities.
use super::*;

pub(crate) fn qa_task_capability_input(
    app: &App,
    build_directory: PathBuf,
    requested_scope: Option<QaScope>,
) -> std::result::Result<QaTaskCapabilityInput, String> {
    let mut scopes = app
        .workspace
        .recipes
        .iter()
        .filter_map(|recipe| {
            let file = recipe.file.clone()?;
            let identity = RecipeIdentity {
                name: recipe.name.clone(),
                file,
            };
            let reported_tasks = app
                .recipe_metadata
                .get(&recipe.name)
                .and_then(|metadata| metadata.tasks.clone())
                .unwrap_or_default();
            let family_tasks = qa_family_task_bindings(&reported_tasks);
            let is_kernel = family_tasks
                .iter()
                .any(|binding| binding.family == QaCheckFamily::KernelConfiguration);
            Some(QaTaskScopeInput {
                identity,
                reported_tasks,
                family_tasks,
                is_kernel,
                report_roots: qa_task_report_roots(app),
            })
        })
        .collect::<Vec<_>>();
    scopes.sort_by(|left, right| {
        left.identity
            .name
            .cmp(&right.identity.name)
            .then_with(|| left.identity.file.cmp(&right.identity.file))
    });
    scopes.dedup_by(|left, right| left.identity == right.identity);
    let selected = requested_scope
        .map(|scope| scope.recipe)
        .filter(|identity| scopes.iter().any(|scope| scope.identity == *identity))
        .or_else(|| {
            app.qa
                .scope
                .as_ref()
                .map(|scope| scope.recipe.clone())
                .filter(|identity| scopes.iter().any(|scope| scope.identity == *identity))
        })
        .or_else(|| {
            app.workspace
                .recipes
                .get(app.recipe_selection)
                .and_then(|recipe| {
                    recipe.file.clone().map(|file| RecipeIdentity {
                        name: recipe.name.clone(),
                        file,
                    })
                })
                .filter(|identity| scopes.iter().any(|scope| scope.identity == *identity))
        })
        .or_else(|| scopes.first().map(|scope| scope.identity.clone()))
        .ok_or_else(|| "QA needs at least one exact recipe/provider identity".to_owned())?;
    Ok(QaTaskCapabilityInput {
        release: app.workspace.release.clone(),
        build_directory,
        selected,
        scopes,
    })
}

pub(crate) fn qa_family_task_bindings(reported_tasks: &[String]) -> Vec<QaFamilyTaskBinding> {
    [
        (
            QaCheckFamily::KernelConfiguration,
            ["do_kernel_configcheck"].as_slice(),
        ),
        (QaCheckFamily::UriFetch, ["do_checkuri"].as_slice()),
        (QaCheckFamily::Patch, ["do_patch_qa"].as_slice()),
        (QaCheckFamily::License, ["do_populate_lic"].as_slice()),
        (QaCheckFamily::RecipePackage, ["do_package_qa"].as_slice()),
    ]
    .into_iter()
    .flat_map(|(family, candidates)| {
        candidates
            .iter()
            .filter(|candidate| reported_tasks.iter().any(|task| task == **candidate))
            .map(move |task| QaFamilyTaskBinding {
                family,
                task: (*task).into(),
            })
    })
    .collect()
}

pub(crate) fn qa_task_report_roots(app: &App) -> Vec<QaReportRootInput> {
    [
        (
            QaCheckFamily::KernelConfiguration,
            "KERNEL_CONFIGCHECK_REPORT_ROOT",
        ),
        (QaCheckFamily::UriFetch, "URI_QA_REPORT_ROOT"),
        (QaCheckFamily::Patch, "PATCH_QA_REPORT_ROOT"),
        (QaCheckFamily::License, "LICENSE_QA_REPORT_ROOT"),
        (QaCheckFamily::RecipePackage, "PACKAGE_QA_REPORT_ROOT"),
    ]
    .into_iter()
    .filter_map(|(family, name)| {
        app.workspace
            .variables
            .get(name)
            .map(|value| QaReportRootInput {
                family,
                path: PathBuf::from(value),
            })
    })
    .collect()
}

pub(crate) fn qa_layer_capability_input(
    app: &App,
    build_directory: PathBuf,
    path_directories: Vec<PathBuf>,
) -> std::result::Result<QaLayerCapabilityInput, String> {
    let report_roots = ["YOCTO_CHECK_LAYER_REPORT_ROOT", "LAYER_QA_REPORT_ROOT"]
        .into_iter()
        .filter_map(|name| app.workspace.variables.get(name).map(PathBuf::from))
        .collect::<Vec<_>>();
    let layers = app
        .workspace
        .layers
        .iter()
        .map(|layer| {
            let identity = QaLayerIdentity::new(layer.name.clone(), layer.path.clone())
                .map_err(str::to_owned)?;
            let compatible_series = app
                .workspace
                .variables
                .get(&format!(
                    "LAYERSERIES_COMPAT_{}",
                    layer.name.replace('-', "_")
                ))
                .map(|value| value.split_whitespace().map(str::to_owned).collect())
                .unwrap_or_default();
            Ok(QaConfiguredLayerInput {
                check: QaCheckId::new("layer-qa".into()).expect("static QA check ID is valid"),
                identity,
                compatible_series,
                report_roots: report_roots.clone(),
            })
        })
        .collect::<std::result::Result<Vec<_>, String>>()?;
    let selected_layer = app
        .qa
        .layer_selection
        .clone()
        .filter(|identity| layers.iter().any(|layer| layer.identity == *identity))
        .or_else(|| layers.first().map(|layer| layer.identity.clone()))
        .ok_or_else(|| "layer QA needs at least one exact configured layer".to_owned())?;
    Ok(QaLayerCapabilityInput {
        release: app.workspace.release.clone(),
        build_directory,
        selected_layer,
        layers,
        executable_search_path: path_directories,
    })
}
