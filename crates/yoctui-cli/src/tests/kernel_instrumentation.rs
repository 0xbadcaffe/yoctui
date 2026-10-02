use super::*;
use yoctui_model::KernelInstrumentationPreset;

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "yoctui-kernel-instrumentation-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn draft(&self) -> KernelInstrumentationDraft {
        let path = self.0.join(".config");
        fs::write(&path, "CONFIG_DEBUG_KERNEL=y\n# CONFIG_KASAN is not set\n").unwrap();
        KernelInstrumentationDraft {
            preset: KernelInstrumentationPreset::Kasan,
            config: path.display().to_string(),
            output: self.0.join("debug.cfg").display().to_string(),
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn kernel_instrumentation_inspection_is_read_only_and_export_creates_only_reviewed_file() {
    let fixture = Fixture::new();
    let draft = fixture.draft();
    let before = fs::read(&draft.config).unwrap();
    let preview = inspect(&draft).unwrap();
    assert!(!preview.report.matches_requested());
    assert!(!Path::new(&draft.output).exists());
    assert_eq!(fs::read(&draft.config).unwrap(), before);
    assert_eq!(export(&preview).unwrap(), PathBuf::from(&draft.output));
    assert_eq!(
        fs::read_to_string(&draft.output).unwrap(),
        draft.preset.fragment()
    );
    assert_eq!(fs::read(&draft.config).unwrap(), before);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&draft.output).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    assert!(export(&preview).is_err());
    assert_eq!(
        fs::read_to_string(&draft.output).unwrap(),
        draft.preset.fragment()
    );
}

#[test]
fn kernel_instrumentation_changed_config_collision_and_parent_block_before_write() {
    let fixture = Fixture::new();
    let draft = fixture.draft();
    let preview = inspect(&draft).unwrap();
    fs::write(&draft.config, "CONFIG_KASAN=y\n").unwrap();
    assert!(
        export(&preview)
            .unwrap_err()
            .to_string()
            .contains("changed since review")
    );
    assert!(!Path::new(&draft.output).exists());
    let preview = inspect(&draft).unwrap();
    fs::write(&draft.output, "user file").unwrap();
    assert!(export(&preview).is_err());
    assert_eq!(fs::read_to_string(&draft.output).unwrap(), "user file");
    let mut draft = draft;
    draft.output = fixture.0.join("missing/debug.cfg").display().to_string();
    assert!(inspect(&draft).is_err());
}

#[test]
fn kernel_instrumentation_oversize_invalid_utf8_and_duplicate_configs_are_refused() {
    let fixture = Fixture::new();
    let draft = fixture.draft();
    for content in [
        vec![0xff],
        b"CONFIG_KASAN=y\nCONFIG_KASAN=n\n".to_vec(),
        vec![b'x'; MAX_KERNEL_INSTRUMENTATION_CONFIG_BYTES + 1],
    ] {
        fs::write(&draft.config, content).unwrap();
        assert!(inspect(&draft).is_err());
        assert!(!Path::new(&draft.output).exists());
    }
}

#[cfg(unix)]
#[test]
fn kernel_instrumentation_symlink_special_and_parent_substitution_are_refused() {
    use std::os::unix::{ffi::OsStrExt, fs::symlink};
    let fixture = Fixture::new();
    let draft = fixture.draft();
    symlink("missing", &draft.output).unwrap();
    assert!(inspect(&draft).is_err());
    let real_config = fixture.0.join("real.config");
    fs::rename(&draft.config, &real_config).unwrap();
    symlink(real_config, &draft.config).unwrap();
    assert!(inspect(&draft).is_err());
    let fifo = fixture.0.join("fifo");
    let name = std::ffi::CString::new(fifo.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    let mut draft = fixture.draft(); // Overwriting the symlink target is fixture-only.
    draft.config = fifo.display().to_string();
    assert!(inspect(&draft).is_err());
    let fixture = Fixture::new();
    let draft = fixture.draft();
    let parent = fixture.0.join("destination");
    fs::create_dir(&parent).unwrap();
    let mut draft = draft;
    draft.output = parent.join("new.cfg").display().to_string();
    let preview = inspect(&draft).unwrap();
    fs::rename(&parent, fixture.0.join("original-destination")).unwrap();
    fs::create_dir(&parent).unwrap();
    assert!(export(&preview).is_err());
    assert!(!Path::new(&draft.output).exists());
    assert!(create_output(&preview).is_err());
}
