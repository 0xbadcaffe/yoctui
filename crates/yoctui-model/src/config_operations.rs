//! Config operations.
use super::*;

pub(crate) fn filtered_config_identities(app: &App) -> Vec<VariableIdentity> {
    let query = app.metadata_query.to_ascii_lowercase();
    let mut identities = app
        .workspace
        .variables
        .iter()
        .filter(|(name, value)| {
            query.is_empty()
                || name.to_ascii_lowercase().contains(&query)
                || value.to_ascii_lowercase().contains(&query)
        })
        .map(|(name, _)| VariableIdentity {
            name: name.clone(),
            recipe: None,
        })
        .collect::<Vec<_>>();
    identities.sort_by(|left, right| left.name.cmp(&right.name));
    identities
}

pub(crate) fn selected_config_identity(app: &App) -> Option<VariableIdentity> {
    filtered_config_identities(app)
        .get(app.config_selection)
        .map(|identity| VariableIdentity {
            name: identity.name.clone(),
            recipe: app.config_scope.clone(),
        })
}

pub fn selected_config_copy_value(app: &App, value: ConfigCopyValue) -> Result<&str, String> {
    let identity = selected_config_identity(app)
        .ok_or_else(|| "No configuration variable is selected.".to_owned())?;
    if app.variable_detail_loading.contains(&identity) {
        return Err(format!(
            "Configuration detail for {} is still loading.",
            identity.name
        ));
    }
    if let Some(error) = app.variable_detail_errors.get(&identity) {
        return Err(format!(
            "Configuration detail for {} is unavailable: {error}",
            identity.name
        ));
    }
    let detail = app.variable_details.get(&identity).ok_or_else(|| {
        format!(
            "Load authoritative detail for {} with Enter before copying.",
            identity.name
        )
    })?;
    match value {
        ConfigCopyValue::Effective => detail
            .effective_value
            .as_deref()
            .ok_or_else(|| format!("The effective value for {} is unavailable.", identity.name)),
        ConfigCopyValue::Unexpanded => detail
            .unexpanded_value
            .as_deref()
            .ok_or_else(|| format!("The unexpanded value for {} is unavailable.", identity.name)),
    }
}

pub(crate) fn selected_config_sources(
    app: &App,
) -> Result<(VariableIdentity, Vec<ConfigSourceChoice>), String> {
    let identity = selected_config_identity(app)
        .ok_or_else(|| "No configuration variable is selected.".to_owned())?;
    if app.variable_detail_loading.contains(&identity) {
        return Err(format!(
            "Configuration detail for {} is still loading.",
            identity.name
        ));
    }
    if let Some(error) = app.variable_detail_errors.get(&identity) {
        return Err(format!(
            "Configuration detail for {} is unavailable: {error}",
            identity.name
        ));
    }
    let detail = app.variable_details.get(&identity).ok_or_else(|| {
        format!(
            "Load authoritative detail for {} with Enter before opening a source.",
            identity.name
        )
    })?;
    let mut seen = HashSet::new();
    let sources = detail
        .operations
        .iter()
        .filter_map(|operation| {
            let path = operation.file.clone()?;
            (!path.as_os_str().is_empty() && seen.insert((path.clone(), operation.line))).then(
                || ConfigSourceChoice {
                    operation: operation.operation.clone(),
                    path,
                    line: operation.line,
                },
            )
        })
        .collect::<Vec<_>>();
    if sources.is_empty() {
        return Err(format!(
            "No file-backed defining operation is available for {}.",
            identity.name
        ));
    }
    Ok((identity, sources))
}

pub fn config_source_disabled_reason(app: &App) -> Option<String> {
    selected_config_sources(app).err()
}

pub(crate) fn comparison_field(
    global: Option<String>,
    recipe: Option<String>,
) -> ConfigComparisonField {
    let outcome = match (&global, &recipe) {
        (Some(global), Some(recipe)) if global == recipe => ConfigComparisonOutcome::Equal,
        (Some(_), Some(_)) => ConfigComparisonOutcome::Different,
        _ => ConfigComparisonOutcome::Unavailable,
    };
    ConfigComparisonField {
        global,
        recipe,
        outcome,
    }
}

pub fn config_comparison(app: &App) -> Result<ConfigComparison, String> {
    let selected = selected_config_identity(app)
        .ok_or_else(|| "No configuration variable is selected.".to_owned())?;
    let recipe = app
        .config_scope
        .clone()
        .ok_or_else(|| "Select a recipe scope with s before comparing.".to_owned())?;
    if !app
        .workspace
        .recipes
        .iter()
        .any(|candidate| candidate.name == recipe)
    {
        return Err(format!("Recipe scope {recipe} is no longer available."));
    }
    let global_identity = VariableIdentity {
        name: selected.name.clone(),
        recipe: None,
    };
    let recipe_identity = VariableIdentity {
        name: selected.name.clone(),
        recipe: Some(recipe.clone()),
    };
    for identity in [&global_identity, &recipe_identity] {
        let scope = identity.recipe.as_deref().unwrap_or("global");
        if app.variable_detail_loading.contains(identity) {
            return Err(format!("Detail for {scope} scope is still loading."));
        }
        if let Some(error) = app.variable_detail_errors.get(identity) {
            return Err(format!("Detail for {scope} scope is unavailable: {error}"));
        }
    }
    let global = app
        .variable_details
        .get(&global_identity)
        .ok_or_else(|| format!("Load global detail for {} before comparing.", selected.name))?;
    let scoped = app.variable_details.get(&recipe_identity).ok_or_else(|| {
        format!(
            "Load {recipe} detail for {} before comparing.",
            selected.name
        )
    })?;
    Ok(ConfigComparison {
        variable: selected.name,
        recipe,
        effective: comparison_field(
            global.effective_value.clone(),
            scoped.effective_value.clone(),
        ),
        unexpanded: comparison_field(
            global.unexpanded_value.clone(),
            scoped.unexpanded_value.clone(),
        ),
    })
}

pub const EDITABLE_CONFIG_VARIABLES: &[&str] = &["DISTRO", "MACHINE"];

pub(crate) fn config_edit_context(
    app: &App,
) -> Result<(VariableIdentity, String, PathBuf), String> {
    if app.config_scope.is_some() {
        return Err("Recipe-scoped configuration is read-only; select global scope.".into());
    }
    let identity = selected_config_identity(app)
        .ok_or_else(|| "No configuration variable is selected.".to_owned())?;
    if !EDITABLE_CONFIG_VARIABLES.contains(&identity.name.as_str()) {
        return Err(format!(
            "{} is read-only; editable variables are {}.",
            identity.name,
            EDITABLE_CONFIG_VARIABLES.join(", ")
        ));
    }
    if app.variable_detail_loading.contains(&identity) {
        return Err(format!(
            "Configuration detail for {} is still loading.",
            identity.name
        ));
    }
    if let Some(error) = app.variable_detail_errors.get(&identity) {
        return Err(format!(
            "Configuration detail for {} is unavailable: {error}",
            identity.name
        ));
    }
    let detail = app.variable_details.get(&identity).ok_or_else(|| {
        format!(
            "Load authoritative detail for {} with Enter before editing.",
            identity.name
        )
    })?;
    let value = detail
        .effective_value
        .clone()
        .ok_or_else(|| format!("The effective value for {} is unavailable.", identity.name))?;
    let destination = app
        .workspace
        .build_dir
        .as_ref()
        .map(|build_dir| build_dir.join("conf/local.conf"))
        .ok_or_else(|| {
            "An active build directory is required for configuration editing.".to_owned()
        })?;
    Ok((identity, value, destination))
}

pub fn config_edit_disabled_reason(app: &App) -> Option<String> {
    config_edit_context(app).err()
}

pub fn config_edit_assignment(name: &str, value: &str) -> Result<String, String> {
    if value.chars().any(char::is_control) {
        return Err("Configuration values cannot contain newlines or control characters.".into());
    }
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    Ok(format!("{name} = \"{escaped}\""))
}

pub(crate) fn popup_toml_value(content: &str, key: &str) -> Result<String, String> {
    let mut value = None;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((name, raw_value)) = line.split_once('=') else {
            return Err(format!("Expected `{key} = \"value\"`."));
        };
        if name.trim() != key || value.is_some() {
            return Err(format!("Only one `{key} = \"value\"` entry is allowed."));
        }
        let raw_value = raw_value.trim();
        if !(raw_value.starts_with('\"') && raw_value.ends_with('\"')) || raw_value.len() < 2 {
            return Err(format!("`{key}` must be a quoted TOML string."));
        }
        let unescaped = raw_value[1..raw_value.len() - 1]
            .replace("\\\\", "\\")
            .replace("\\\"", "\"");
        value = Some(unescaped);
    }
    value.ok_or_else(|| format!("Missing `{key} = \"value\"` entry."))
}

pub(crate) fn popup_toml_document(key: &str, value: &str, comment: Option<&str>) -> String {
    let mut document = comment.map_or_else(String::new, |comment| format!("# {comment}\n"));
    document.push_str(&format!(
        "{key} = \"{}\"\n",
        value.replace('\\', "\\\\").replace('\"', "\\\"")
    ));
    document
}

pub(crate) fn popup_toml_fields(content: &str) -> Result<HashMap<String, String>, String> {
    let mut fields = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, raw_value)) = line.split_once('=') else {
            return Err("Expected TOML `key = value` entries.".into());
        };
        let key = key.trim();
        if key.is_empty() || fields.contains_key(key) {
            return Err("TOML keys must be nonempty and occur only once.".into());
        }
        let raw_value = raw_value.trim();
        let value =
            if raw_value.starts_with('\"') && raw_value.ends_with('\"') && raw_value.len() >= 2 {
                raw_value[1..raw_value.len() - 1]
                    .replace("\\\\", "\\")
                    .replace("\\\"", "\"")
            } else {
                raw_value.to_owned()
            };
        fields.insert(key.to_owned(), value);
    }
    Ok(fields)
}

pub fn validate_config_edit_request(
    request: &ConfigEditRequest,
    build_dir: &Path,
) -> Result<(), String> {
    if request.identity.recipe.is_some() {
        return Err("Recipe-scoped configuration edits are not allowed.".into());
    }
    if !EDITABLE_CONFIG_VARIABLES.contains(&request.identity.name.as_str()) {
        return Err(format!(
            "{} is read-only; editable variables are {}.",
            request.identity.name,
            EDITABLE_CONFIG_VARIABLES.join(", ")
        ));
    }
    let expected_destination = build_dir.join("conf/local.conf");
    if request.destination != expected_destination {
        return Err(format!(
            "Configuration edit destination must be {}.",
            expected_destination.display()
        ));
    }
    let expected_assignment = config_edit_assignment(&request.identity.name, &request.value)?;
    if request.assignment != expected_assignment {
        return Err("Configuration edit assignment does not match the confirmed value.".into());
    }
    Ok(())
}

pub(crate) fn resolve_config_source(app: &App, path: &Path) -> Result<PathBuf, String> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(format!(
            "Relative configuration source escapes the build directory: {}.",
            path.display()
        ));
    }
    app.workspace
        .build_dir
        .as_ref()
        .map(|build_dir| build_dir.join(path))
        .ok_or_else(|| {
            format!(
                "Cannot resolve relative configuration source {} without an active build directory.",
                path.display()
            )
    })
}
