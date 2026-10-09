//! Map Yocto's reproducible kernel prefix only within the selected build/machine.
use std::path::{Path, PathBuf};
use yoctui_model::QemuDebugSpec;

pub(super) fn discover(spec: &QemuDebugSpec) -> Option<PathBuf> {
    let build = spec.build_dir.canonicalize().ok()?;
    let config = spec.qemuboot.canonicalize().ok()?;
    let machine_dir = config.parent()?;
    if machine_dir.parent()? != build.join("tmp/deploy/images") {
        return None;
    }
    let machine = machine_dir.file_name()?.to_str()?;
    if !safe_token(machine) || machine == "." || machine == ".." {
        return None;
    }
    let source = build
        .join("tmp/work-shared")
        .join(machine)
        .join("kernel-source")
        .canonicalize()
        .ok()?;
    let main = source.join("init/main.c").canonicalize().ok()?;
    if !source.starts_with(&build) || !main.starts_with(&source) || !main.is_file() {
        return None;
    }
    // This value becomes a GDB command argument, not a shell argument. Reject
    // whitespace/quotes/control syntax rather than guessing GDB quoting rules.
    source.to_str().filter(|text| {
        text.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/._-+".contains(&b))
    })?;
    Some(source)
}

fn safe_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-+".contains(&b))
}

pub(super) fn arguments(spec: &QemuDebugSpec, socket: &Path) -> Vec<String> {
    let mut arguments = spec.gdb_arguments(socket);
    if let Some(source) = discover(spec) {
        arguments.extend([
            "-iex".into(),
            format!("set substitute-path /usr/src/kernel {}", source.display()),
        ]);
    }
    arguments
}
