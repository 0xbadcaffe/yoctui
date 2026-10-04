//! Read-only, bounded discovery in the selected build; never walks source/work trees.
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};
use yoctui_model::{
    KernelDebugDefaultContext, KernelDebugDefaults, KernelDebugField as F, QemuDebugBootMode,
};

const MAX_ENTRIES: usize = 512;

#[cfg(test)]
fn discover(context: &KernelDebugDefaultContext) -> Result<KernelDebugDefaults, String> {
    discover_for_tool(context, yoctui_model::KernelDebugTool::QemuGdb)
}

pub(super) fn discover_for_tool(
    context: &KernelDebugDefaultContext,
    tool: yoctui_model::KernelDebugTool,
) -> Result<KernelDebugDefaults, String> {
    if !token(&context.machine) || context.image.as_ref().is_some_and(|v| !token(v)) {
        return Err("Build machine/image identity is invalid".into());
    }
    let build = context
        .build_dir
        .canonicalize()
        .map_err(|e| format!("Build directory unavailable: {e}"))?;
    if !context.build_dir.is_absolute() || !build.is_dir() {
        return Err("Build directory must be absolute and available".into());
    }
    let deploy = build.join("tmp/deploy/images").join(&context.machine);
    let shared = build
        .join("tmp/work-shared")
        .join(&context.machine)
        .join("kernel-build-artifacts");
    let mut defaults = KernelDebugDefaults::default();
    let abi = regular(&build, &shared.join("kernel-abiversion"))
        .and_then(|p| read_small(&p, 1024).ok())
        .map(|s| s.trim().to_owned())
        .filter(|s| token(s));
    let symbols = if let Some(abi) = &abi {
        choose_symbols(
            &build,
            &[
                shared.join(format!("vmlinux-{abi}")),
                build.join(format!(
                    "yoctui-debug-artifacts-{}/package/boot/vmlinux-{abi}",
                    context.machine
                )),
                build.join(format!(
                    "yoctui-debug-artifacts-{}/package/boot/.debug/vmlinux-{abi}",
                    context.machine
                )),
            ],
        )
    } else {
        None
    }
    .or_else(|| choose_symbols(&build, &[shared.join("vmlinux")]));
    if let Some(path) = symbols {
        put(&mut defaults, F::Symbols, &path);
    }
    if let Some(path) = regular(&build, &shared.join(".config")) {
        put(&mut defaults, F::KernelConfig, &path);
    }
    if tool != yoctui_model::KernelDebugTool::QemuGdb {
        let missing = !defaults.values.iter().any(|(f, _)| *f == F::Symbols);
        defaults.note = if missing { "Missing/ambiguous: vmlinux symbols; enter matching files manually." }
            else { "Selected build symbols found; confirm they match the target. Serial config/readiness remain explicit." }.into();
        return Ok(defaults);
    }
    let configs = entries(&build, &deploy)?
        .into_iter()
        .filter(|p| {
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or_default();
            name.ends_with(".qemuboot.conf")
                && context
                    .image
                    .as_ref()
                    .is_none_or(|image| name.starts_with(&format!("{image}-{}", context.machine)))
        })
        .collect::<Vec<_>>();
    let preferred = context.image.as_ref().and_then(|image| {
        regular(
            &build,
            &deploy.join(format!("{image}-{}.qemuboot.conf", context.machine)),
        )
    });
    let config = preferred.or_else(|| unique(configs));
    if let Some(path) = config {
        let values = parse(&read_small(&path, 128 * 1024)?)?;
        if values.get("machine").is_some_and(|v| v != &context.machine) {
            return Err("QEMU configuration belongs to another machine".into());
        }
        let name = values.get("image_name").filter(|v| token(v));
        if context.image.as_ref().is_some_and(|image| {
            name.is_none_or(|name| !name.starts_with(&format!("{image}-{}", context.machine)))
        }) {
            return Err("QEMU configuration belongs to another image".into());
        }
        put(&mut defaults, F::Qemuboot, &path);
        let flash = [
            ("qb_system_name", "qemu-system-arm"),
            ("qb_default_kernel", "none"),
            ("qb_machine", "-machine romulus-bmc"),
            ("qb_default_fstype", "static.mtd"),
            ("qb_rootfs_opt", "-drive file=@ROOTFS@,if=mtd,format=raw"),
        ]
        .iter()
        .all(|(key, value)| values.get(*key).map(String::as_str) == Some(*value));
        if let Some(memory) = values
            .get("qb_mem")
            .and_then(|s| s.strip_prefix("-m "))
            .filter(|s| s.parse::<u32>().is_ok_and(|n| (128..=262144).contains(&n)))
        {
            defaults.values.push((F::Memory, memory.to_string()));
        }
        if let (Some(name), Some(fstype)) =
            (name, values.get("qb_default_fstype").filter(|s| token(s)))
            && let Some(path) = regular(&build, &deploy.join(format!("{name}.{fstype}")))
        {
            put(&mut defaults, F::RootfsImage, &path);
            if flash && fs::metadata(&path).is_ok_and(|m| m.len() == 32 * 1024 * 1024) {
                defaults.boot_mode = Some(QemuDebugBootMode::OpenBmcRomulusFlash);
            }
        }
        let kernel = values
            .get("qb_default_kernel")
            .filter(|s| token(s) && *s != "none")
            .or_else(|| values.get("kernel_imagetype").filter(|s| token(s)));
        if let Some(kernel) = kernel {
            let candidates = vec![
                deploy.join(kernel),
                deploy.join(format!("{kernel}-{}.bin", context.machine)),
            ];
            let path = choose(&build, &candidates).or_else(|| {
                abi.as_ref().and_then(|abi| {
                    regular(
                        &build,
                        &build.join(format!(
                            "yoctui-debug-artifacts-{}/package/boot/{kernel}-{abi}",
                            context.machine
                        )),
                    )
                })
            });
            if let Some(path) = path {
                put(&mut defaults, F::KernelImage, &path);
            }
        }
    }
    let missing = [F::Symbols, F::Qemuboot, F::KernelImage, F::RootfsImage]
        .into_iter()
        .filter(|field| !defaults.values.iter().any(|(f, _)| f == field))
        .map(|f| match f {
            F::Symbols => "vmlinux symbols",
            F::Qemuboot => "qemuboot",
            F::KernelImage => "boot kernel",
            _ => "rootfs image",
        })
        .collect::<Vec<_>>();
    defaults.note = if missing.is_empty() {
        "Selected build defaults found; confirm symbols match the guest before launch.".into()
    } else {
        format!(
            "Missing/ambiguous: {}. Enter exact matching files; no other build searched.",
            missing.join(", ")
        )
    };
    Ok(defaults)
}

fn token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value != "."
        && value != ".."
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._+-".contains(&b))
}

fn regular(build: &Path, path: &Path) -> Option<PathBuf> {
    let path = path.canonicalize().ok()?;
    (path.starts_with(build) && fs::symlink_metadata(&path).ok()?.is_file()).then_some(path)
}

fn unique(mut paths: Vec<PathBuf>) -> Option<PathBuf> {
    paths.sort();
    paths.dedup();
    (paths.len() == 1).then(|| paths.remove(0))
}

fn choose(build: &Path, paths: &[PathBuf]) -> Option<PathBuf> {
    unique(paths.iter().filter_map(|p| regular(build, p)).collect())
}

fn choose_symbols(build: &Path, paths: &[PathBuf]) -> Option<PathBuf> {
    unique(
        paths
            .iter()
            .filter_map(|p| regular(build, p))
            .filter(|p| {
                File::open(p)
                    .and_then(|mut f| {
                        crate::qemu_debug::validate_symbol_file(&mut f)
                            .map_err(std::io::Error::other)
                    })
                    .is_ok()
            })
            .collect(),
    )
}

fn entries(build: &Path, directory: &Path) -> Result<Vec<PathBuf>, String> {
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let directory = directory.canonicalize().map_err(|e| e.to_string())?;
    if !directory.starts_with(build) {
        return Err("Artifact directory escapes selected build".into());
    }
    let mut paths = Vec::new();
    for (index, entry) in fs::read_dir(directory)
        .map_err(|e| e.to_string())?
        .enumerate()
    {
        if index >= MAX_ENTRIES {
            return Err(
                "Artifact directory exceeds 512 entries; enter exact paths manually".into(),
            );
        }
        let path = entry.map_err(|e| e.to_string())?.path();
        if let Some(path) = regular(build, &path) {
            paths.push(path);
        }
    }
    Ok(paths)
}

fn read_small(path: &Path, limit: u64) -> Result<String, String> {
    let mut text = String::new();
    File::open(path)
        .map_err(|e| e.to_string())?
        .take(limit + 1)
        .read_to_string(&mut text)
        .map_err(|e| e.to_string())?;
    if text.len() as u64 > limit {
        return Err("Artifact metadata exceeds size limit".into());
    }
    Ok(text)
}

fn parse(text: &str) -> Result<BTreeMap<String, String>, String> {
    let mut values = BTreeMap::new();
    let mut section = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            section = line == "[config_bsp]";
        }
        if section
            && let Some((key, value)) = line.split_once('=')
            && values
                .insert(key.trim().to_owned(), value.trim().to_owned())
                .is_some()
        {
            return Err("Duplicate QEMU configuration field".into());
        }
    }
    Ok(values)
}

fn put(defaults: &mut KernelDebugDefaults, field: F, path: &Path) {
    if let Some(value) = path
        .to_str()
        .filter(|s| s.len() <= 4096 && !s.chars().any(char::is_control))
    {
        defaults.values.push((field, value.to_owned()));
    }
}

#[cfg(test)]
#[path = "../tests/kernel_debug_defaults.rs"]
mod tests;
