//! Conservative inspection of the files the upstream status initializer edits.
use std::{fs::File, io::Read, path::Path};

pub(super) enum WorkspaceStatusPreflight {
    Empty,
    Ready,
}

fn read_config(path: &Path) -> Result<Option<String>, String> {
    let mut options = File::options();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK);
    }
    let file = match options.open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("Cannot inspect {}: {error}", path.display())),
    };
    if !file
        .metadata()
        .map_err(|error| error.to_string())?
        .is_file()
    {
        return Err("Devtool configuration is not a regular file.".into());
    }
    let mut bytes = Vec::new();
    file.take(65537)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() > 65536 {
        return Err("Devtool configuration exceeds the inspection limit.".into());
    }
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| "Devtool configuration is not UTF-8.".into())
}

pub(super) fn inspect(build: &Path, executable: &Path) -> Result<WorkspaceStatusPreflight, String> {
    // Upstream SDK fixed setups already refuse missing workspaces and skip
    // _enable_workspace_layer. Preserve their ordinary status/error behavior.
    for directory in executable.parent().into_iter().flat_map(Path::ancestors) {
        if directory
            .join(".devtoolbase")
            .try_exists()
            .map_err(|error| error.to_string())?
        {
            return Ok(WorkspaceStatusPreflight::Ready);
        }
    }
    // ConfigParser interpolation/custom initialization cannot safely be inferred
    // from a textual substring. Leave these configurations to explicit operations.
    if read_config(&build.join("conf/devtool.conf"))?.is_some() {
        return Err("Status inspection cannot safely initialize a custom devtool.conf workspace. Use an explicitly configured workspace operation.".into());
    }
    let workspace = build.join("workspace");
    match workspace.try_exists() {
        Ok(false) => return Ok(WorkspaceStatusPreflight::Empty),
        Ok(true) => {}
        Err(error) => return Err(format!("Cannot inspect the devtool workspace: {error}")),
    }
    if !workspace.join("conf/layer.conf").is_file() {
        return Err(
            "Devtool workspace is not initialized; inspection will not create its layer.".into(),
        );
    }
    let configuration = read_config(&build.join("conf/bblayers.conf"))?
        .ok_or("Missing bblayers.conf; status inspection will not enable a workspace.")?;
    let expected = workspace.to_str().ok_or("Workspace path is not UTF-8.")?;
    if !enabled_literal_workspace(&configuration, expected) {
        return Err("Devtool workspace is disabled or its layer configuration is ambiguous; status inspection will not modify bblayers.conf.".into());
    }
    Ok(WorkspaceStatusPreflight::Ready)
}

fn enabled_literal_workspace(configuration: &str, workspace: &str) -> bool {
    let flattened = configuration.replace("\\\n", " ");
    let mut value = None;
    for line in flattened.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Includes/Python can change BBLAYERS behind a superficial path match.
        if matches!(
            line.split_whitespace().next(),
            Some("include" | "require" | "python")
        ) {
            return false;
        }
        if !line.contains("BBLAYERS") {
            continue;
        }
        let Some(assignment) = line.strip_prefix("BBLAYERS").map(str::trim_start) else {
            return false;
        };
        let expression = ["??=", "?=", "="]
            .into_iter()
            .find_map(|operator| assignment.strip_prefix(operator));
        let Some(expression) = expression.map(str::trim) else {
            return false;
        };
        let Some(expression) = expression
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
        else {
            return false;
        };
        if value.is_some() || expression.contains(['$', '\\', '"', '#']) {
            return false;
        }
        value = Some(expression);
    }
    value.is_some_and(|value| {
        value
            .split_whitespace()
            .any(|layer| layer.trim_end_matches('/') == workspace.trim_end_matches('/'))
    })
}
