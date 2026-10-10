//! One config-only Tinfoil connection, rather than one connection per variable.
use std::{collections::BTreeMap, path::Path};

use super::{DaemonCompatibilityError, process_helpers::run_read_only};

pub(super) const VARIABLES: [&str; 10] = [
    "MACHINE",
    "DISTRO",
    "DISTRO_VERSION",
    "DISTRO_CODENAME",
    "OE_VERSION",
    "COREBASE",
    "LAYERSERIES_CORENAMES",
    "BBLAYERS",
    "BB_HASHSERVE",
    "PRSERV_HOST",
];

const QUERY: &str = r#"
import contextlib, json, os, sys
sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(sys.argv[1])), 'lib'))
import bb.tinfoil
with contextlib.redirect_stdout(sys.stderr):
    with bb.tinfoil.Tinfoil(tracking=True, setup_logging=False) as t:
        t.prepare(quiet=2, config_only=True)
        d = t.finalizeData()
        values = {name: d.getVar(name) for name in json.loads(sys.argv[2])}
print(json.dumps({'build_directory': os.path.realpath(os.getcwd()), 'values': values}))
"#;

pub(super) async fn query(
    getvar: &Path,
    build: &Path,
    environment: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, DaemonCompatibilityError> {
    let python = environment
        .get("PYTHON")
        .map(String::as_str)
        .unwrap_or("python3");
    let tool = getvar
        .to_str()
        .ok_or_else(|| DaemonCompatibilityError::StartupProbe("non-UTF8 getvar path".into()))?;
    let variables = serde_json::to_string(&VARIABLES).expect("static variables serialize");
    let started = std::time::Instant::now();
    let output = run_read_only(
        Path::new(python),
        &["-c", QUERY, tool, &variables],
        build,
        environment,
    )
    .await?;
    tracing::info!(
        elapsed_ms = started.elapsed().as_millis() as u64,
        variables = VARIABLES.len(),
        "daemon discovery: batched datastore query finished"
    );
    parse(&output, build)
}

fn parse(output: &str, build: &Path) -> Result<BTreeMap<String, String>, DaemonCompatibilityError> {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Report {
        build_directory: std::path::PathBuf,
        values: BTreeMap<String, Option<String>>,
    }
    let report: Report = serde_json::from_str(output).map_err(|error| {
        DaemonCompatibilityError::StartupProbe(format!("invalid datastore query: {error}"))
    })?;
    if report.build_directory != build
        || report.values.len() != VARIABLES.len()
        || report
            .values
            .keys()
            .any(|key| !VARIABLES.contains(&key.as_str()))
        || report
            .values
            .values()
            .flatten()
            .any(|value| value.len() > 4096 || value.contains('\0'))
    {
        return Err(DaemonCompatibilityError::StartupProbe(
            "datastore query identity or bounds mismatch".into(),
        ));
    }
    Ok(report
        .values
        .into_iter()
        .filter_map(|(key, value)| value.map(|value| (key, value)))
        .collect())
}

#[cfg(test)]
#[path = "../tests/daemon_compatibility/datastore_query.rs"]
mod tests;
