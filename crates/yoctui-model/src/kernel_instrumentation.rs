//! Requested instrumentation settings, not build or runtime compatibility claims.
use crate::TextAreaRevision;
use std::{collections::BTreeMap, path::PathBuf};

pub const MAX_KERNEL_INSTRUMENTATION_CONFIG_BYTES: usize = 512 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KernelInstrumentationPreset {
    #[default]
    Kasan,
    Kcsan,
    Ubsan,
    Lockdep,
}

impl KernelInstrumentationPreset {
    pub const SANITIZERS: [Self; 3] = [Self::Kasan, Self::Kcsan, Self::Ubsan];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Kasan => "KASAN generic / outline",
            Self::Kcsan => "KCSAN strict",
            Self::Ubsan => "UBSAN bounds / reports",
            Self::Lockdep => "lockdep / atomic sleep",
        }
    }

    pub const fn requested(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Self::Kasan => &[
                ("CONFIG_DEBUG_KERNEL", "y"),
                ("CONFIG_KASAN", "y"),
                ("CONFIG_KASAN_GENERIC", "y"),
                ("CONFIG_KASAN_OUTLINE", "y"),
                ("CONFIG_STACKTRACE", "y"),
                ("CONFIG_KCSAN", "n"),
                ("CONFIG_KASAN_KUNIT_TEST", "n"),
                ("CONFIG_KASAN_MODULE_TEST", "n"),
            ],
            Self::Kcsan => &[
                ("CONFIG_DEBUG_KERNEL", "y"),
                ("CONFIG_KASAN", "n"),
                ("CONFIG_KCSAN", "y"),
                ("CONFIG_KCSAN_STRICT", "y"),
                ("CONFIG_KCSAN_SELFTEST", "n"),
                ("CONFIG_KCSAN_KUNIT_TEST", "n"),
            ],
            Self::Ubsan => &[
                ("CONFIG_UBSAN", "y"),
                ("CONFIG_UBSAN_BOUNDS", "y"),
                ("CONFIG_UBSAN_TRAP", "n"),
            ],
            Self::Lockdep => &[
                ("CONFIG_DEBUG_KERNEL", "y"),
                ("CONFIG_PROVE_LOCKING", "y"),
                ("CONFIG_LOCKDEP", "y"),
                ("CONFIG_DEBUG_ATOMIC_SLEEP", "y"),
                ("CONFIG_DEBUG_LOCKING_API_SELFTESTS", "n"),
            ],
        }
    }

    pub const fn capabilities(self) -> &'static [&'static str] {
        match self {
            Self::Kasan => &[
                "CONFIG_HAVE_ARCH_KASAN",
                "CONFIG_CC_HAS_KASAN_GENERIC",
                "CONFIG_CC_HAS_WORKING_NOSANITIZE_ADDRESS",
                "CONFIG_SLUB",
                "CONFIG_SLAB",
                "CONFIG_SLUB_TINY",
                "CONFIG_DEBUG_SLAB",
            ],
            Self::Kcsan => &["CONFIG_HAVE_ARCH_KCSAN", "CONFIG_HAVE_KCSAN_COMPILER"],
            Self::Ubsan => &[
                "CONFIG_CC_HAS_UBSAN_ARRAY_BOUNDS",
                "CONFIG_CC_HAS_UBSAN_BOUNDS_STRICT",
            ],
            Self::Lockdep => &["CONFIG_LOCK_DEBUGGING_SUPPORT", "CONFIG_ARCH_NO_PREEMPT"],
        }
    }

    pub const fn reference(self) -> &'static str {
        match self {
            Self::Kasan => "https://docs.kernel.org/dev-tools/kasan.html",
            Self::Kcsan => "https://docs.kernel.org/dev-tools/kcsan.html",
            Self::Ubsan => "https://docs.kernel.org/dev-tools/ubsan.html",
            Self::Lockdep => "https://docs.kernel.org/locking/lockdep-design.html",
        }
    }

    pub fn fragment(self) -> String {
        let mut result = format!(
            "# Yoctui requested preset: {}\n# NOT applied automatically. Verify provider Kconfig/architecture/compiler.\n# Recheck resolved .config before deliberate build/boot. No self-tests requested.\n",
            self.label()
        );
        for (name, value) in self.requested() {
            if *value == "n" {
                result.push_str(&format!("# {name} is not set\n"));
            } else {
                result.push_str(&format!("{name}={value}\n"));
            }
        }
        result
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KernelInstrumentationDraft {
    pub preset: KernelInstrumentationPreset,
    pub config: String,
    pub output: String,
}

impl KernelInstrumentationDraft {
    pub fn validate(&self) -> Result<(), String> {
        for value in [&self.config, &self.output] {
            if value.len() > 4096
                || value.chars().any(char::is_control)
                || !yoctui_utils::is_absolute_normal_path(&PathBuf::from(value))
            {
                return Err(
                    "Select bounded normalized absolute .config and new .cfg paths.".into(),
                );
            }
        }
        if PathBuf::from(&self.output)
            .extension()
            .and_then(|value| value.to_str())
            != Some("cfg")
            || self.config == self.output
        {
            return Err("Export must be a NEW .cfg file, never the input .config.".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelInstrumentationReport {
    pub preset: KernelInstrumentationPreset,
    pub options: BTreeMap<String, Option<String>>,
    pub revision: TextAreaRevision,
}

impl KernelInstrumentationReport {
    pub fn inspect(preset: KernelInstrumentationPreset, text: &str) -> Result<Self, String> {
        if text.len() > MAX_KERNEL_INSTRUMENTATION_CONFIG_BYTES
            || text
                .chars()
                .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
        {
            return Err("Kernel config exceeds 512 KiB or contains invalid control bytes.".into());
        }
        let mut options = preset
            .requested()
            .iter()
            .map(|(name, _)| *name)
            .chain(preset.capabilities().iter().copied())
            .map(|name| (name.to_owned(), None))
            .collect::<BTreeMap<_, _>>();
        for line in text.lines().map(str::trim) {
            let entry = line.split_once('=').or_else(|| {
                line.strip_prefix("# ")?
                    .strip_suffix(" is not set")
                    .map(|name| (name, "n"))
            });
            if let Some((name, value)) = entry
                && let Some(observed) = options.get_mut(name.trim())
            {
                let value = value.trim();
                if observed.is_some() || !matches!(value, "y" | "m" | "n") {
                    return Err(format!(
                        "Duplicate or invalid boolean value for {}.",
                        name.trim()
                    ));
                }
                *observed = Some(value.to_owned());
            }
        }
        Ok(Self {
            preset,
            options,
            revision: TextAreaRevision::of(text),
        })
    }

    pub fn matches_requested(&self) -> bool {
        self.preset.requested().iter().all(|(name, value)| {
            self.options.get(*name).and_then(|value| value.as_deref()) == Some(*value)
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelInstrumentationPreview {
    pub draft: KernelInstrumentationDraft,
    pub report: KernelInstrumentationReport,
    pub destination_parent: PathBuf,
    /// Unix directory device/inode; non-Unix adapters retain path identity only.
    pub parent_identity: Option<(u64, u64)>,
}

#[cfg(test)]
#[path = "tests/kernel_instrumentation.rs"]
mod tests;
