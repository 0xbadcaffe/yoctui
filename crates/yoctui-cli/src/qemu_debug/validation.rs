use super::*;
use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::Path,
};

pub(crate) fn validate_files(spec: &QemuDebugSpec) -> Result<()> {
    spec.validate().map_err(anyhow::Error::msg)?;
    if !spec.build_dir.is_dir() {
        bail!("Selected build directory is unavailable");
    }
    for path in [&spec.runqemu, &spec.gdb] {
        if !crate::terminal_launcher::executable(
            &fs::metadata(path).with_context(|| format!("Missing tool {}", path.display()))?,
        ) {
            bail!("Tool is not executable: {}", path.display());
        }
    }
    for path in [&spec.qemuboot, &spec.kernel, &spec.rootfs, &spec.symbols] {
        if !fs::symlink_metadata(path)
            .with_context(|| format!("Missing input {}", path.display()))?
            .is_file()
        {
            bail!(
                "Select an existing regular file, not a symlink/device/directory: {}",
                path.display()
            );
        }
    }
    validate_qemuboot(&spec.qemuboot, spec.boot_mode)?;
    if spec.boot_mode == yoctui_model::QemuDebugBootMode::OpenBmcRomulusFlash {
        if fs::metadata(&spec.rootfs)?.len() != 32 * 1024 * 1024 {
            bail!("Romulus flash must be the complete 32-MiB static.mtd image");
        }
        let mut header = [0_u8; 20];
        File::open(&spec.symbols)?.read_exact(&mut header)?;
        if header[4] != 1 || header[5] != 1 || header[18..20] != [40, 0] {
            bail!("Romulus flash requires matching little-endian 32-bit ARM vmlinux");
        }
    }
    validate_symbols(&spec.symbols)
}

fn validate_qemuboot(path: &Path, mode: yoctui_model::QemuDebugBootMode) -> Result<()> {
    let mut text = String::new();
    File::open(path)?
        .take(128 * 1024 + 1)
        .read_to_string(&mut text)?;
    if text.len() > 128 * 1024 {
        bail!("qemuboot configuration exceeds 128 KiB");
    }
    let mut section = false;
    let mut values = std::collections::BTreeMap::new();
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            section = line.eq_ignore_ascii_case("[config_bsp]");
        }
        if section && let Some((name, value)) = line.split_once('=') {
            let name = name.trim();
            if matches!(
                name,
                "qb_system_name"
                    | "qb_default_kernel"
                    | "qb_machine"
                    | "qb_default_fstype"
                    | "qb_rootfs_opt"
            ) && values.insert(name, value.trim()).is_some()
            {
                bail!("Duplicate qemuboot prerequisite: {name}");
            }
        }
    }
    if !values.get("qb_system_name").is_some_and(|name| {
        name.starts_with("qemu-system-")
            && name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    }) {
        bail!("qemuboot config needs a valid config_bsp qb_system_name");
    }
    if mode == yoctui_model::QemuDebugBootMode::OpenBmcRomulusFlash {
        for (name, expected) in [
            ("qb_system_name", "qemu-system-arm"),
            ("qb_default_kernel", "none"),
            ("qb_machine", "-machine romulus-bmc"),
            ("qb_default_fstype", "static.mtd"),
            ("qb_rootfs_opt", "-drive file=@ROOTFS@,if=mtd,format=raw"),
        ] {
            if values.get(name).copied() != Some(expected) {
                bail!("Romulus flash requires config_bsp {name}={expected}");
            }
        }
        return Ok(());
    }
    if values
        .get("qb_default_kernel")
        .is_none_or(|value| value.is_empty() || *value == "none")
    {
        bail!(
            "Direct kernel mode needs a boot kernel; select OpenBMC Romulus flash mode for its supported static.mtd configuration"
        );
    }
    Ok(())
}

fn validate_symbols(path: &Path) -> Result<()> {
    let mut file = File::open(path)?;
    validate_symbol_file(&mut file)
}

pub(crate) fn validate_symbol_file(file: &mut File) -> Result<()> {
    let length = file.metadata()?.len();
    let mut header = [0_u8; 64];
    file.read_exact(&mut header)
        .context("Symbols must be an uncompressed ELF vmlinux")?;
    if &header[..4] != b"\x7fELF" || ![1, 2].contains(&header[4]) || ![1, 2].contains(&header[5]) {
        bail!("Symbols must be an uncompressed ELF vmlinux, not bzImage or a text map");
    }
    let little = header[5] == 1;
    let number = |bytes: &[u8]| -> u64 {
        if little {
            bytes
                .iter()
                .rev()
                .fold(0, |value, byte| (value << 8) | u64::from(*byte))
        } else {
            bytes
                .iter()
                .fold(0, |value, byte| (value << 8) | u64::from(*byte))
        }
    };
    let elf64 = header[4] == 2;
    let (table, stride, count, names) = if elf64 {
        (
            number(&header[40..48]),
            number(&header[58..60]),
            number(&header[60..62]),
            number(&header[62..64]),
        )
    } else {
        (
            number(&header[32..36]),
            number(&header[46..48]),
            number(&header[48..50]),
            number(&header[50..52]),
        )
    };
    if count == 0
        || count > 8192
        || names >= count
        || stride != if elf64 { 64 } else { 40 }
        || table
            .checked_add(count * stride)
            .is_none_or(|end| end > length)
    {
        bail!("Unsupported or malformed ELF section table");
    }
    file.seek(SeekFrom::Start(table))?;
    let mut sections = vec![0; (count * stride) as usize];
    file.read_exact(&mut sections)?;
    let section = &sections[(names * stride) as usize..((names + 1) * stride) as usize];
    let (offset, size) = if elf64 {
        (number(&section[24..32]), number(&section[32..40]))
    } else {
        (number(&section[16..20]), number(&section[20..24]))
    };
    if size > 1024 * 1024 || offset.checked_add(size).is_none_or(|end| end > length) {
        bail!("Invalid or oversized ELF section-name table");
    }
    file.seek(SeekFrom::Start(offset))?;
    let mut strings = vec![0; size as usize];
    file.read_exact(&mut strings)?;
    let mut symbols = false;
    let mut dwarf = false;
    for section in sections.chunks_exact(stride as usize) {
        let index = number(&section[..4]) as usize;
        let Some(tail) = strings.get(index..) else {
            bail!("Invalid ELF section name");
        };
        let name = tail.split(|byte| *byte == 0).next().unwrap_or_default();
        symbols |= name == b".symtab";
        dwarf |= name == b".debug_info" || name == b".zdebug_info";
    }
    if !symbols || !dwarf {
        bail!(
            "vmlinux needs its symbol table and DWARF debug information; build/install matching debug symbols first"
        );
    }
    Ok(())
}
