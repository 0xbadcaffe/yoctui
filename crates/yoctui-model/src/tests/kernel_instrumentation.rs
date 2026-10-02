use super::*;

#[test]
fn kernel_instrumentation_presets_are_closed_deterministic_and_do_not_request_selftests() {
    for preset in [
        KernelInstrumentationPreset::Kasan,
        KernelInstrumentationPreset::Kcsan,
        KernelInstrumentationPreset::Ubsan,
        KernelInstrumentationPreset::Lockdep,
    ] {
        let fragment = preset.fragment();
        assert_eq!(fragment, preset.fragment());
        assert!(fragment.contains("NOT applied automatically"));
        let report = KernelInstrumentationReport::inspect(preset, &fragment).unwrap();
        assert!(report.matches_requested());
        for (name, value) in preset.requested() {
            if name.contains("TEST") {
                assert_eq!(*value, "n");
            }
        }
    }
    assert!(
        KernelInstrumentationPreset::Kasan
            .fragment()
            .contains("# CONFIG_KCSAN is not set")
    );
    assert!(
        KernelInstrumentationPreset::Kcsan
            .fragment()
            .contains("# CONFIG_KASAN is not set")
    );
    assert!(
        KernelInstrumentationPreset::Ubsan
            .fragment()
            .contains("# CONFIG_UBSAN_TRAP is not set")
    );
}

#[test]
fn kernel_instrumentation_config_distinguishes_absent_disabled_module_and_enabled() {
    let preset = KernelInstrumentationPreset::Kasan;
    let text = "CONFIG_KASAN=y\n# CONFIG_KASAN_GENERIC is not set\nCONFIG_KASAN_MODULE_TEST=m\n";
    let report = KernelInstrumentationReport::inspect(preset, text).unwrap();
    assert_eq!(report.options["CONFIG_KASAN"].as_deref(), Some("y"));
    assert_eq!(report.options["CONFIG_KASAN_GENERIC"].as_deref(), Some("n"));
    assert_eq!(
        report.options["CONFIG_KASAN_MODULE_TEST"].as_deref(),
        Some("m")
    );
    assert_eq!(report.options["CONFIG_HAVE_ARCH_KASAN"], None);
    assert!(!report.matches_requested());
    assert_ne!(
        report.revision,
        TextAreaRevision::of(&(text.to_owned() + "# unrelated change\n"))
    );
}

#[test]
fn kernel_instrumentation_config_rejects_duplicates_invalid_control_and_oversize() {
    for text in [
        "CONFIG_KASAN=y\nCONFIG_KASAN=n",
        "CONFIG_KASAN=y\n# CONFIG_KASAN is not set",
        "CONFIG_KASAN=garbage",
        "CONFIG_HAVE_ARCH_KASAN=\"y\"",
        "CONFIG_KASAN=y\0",
    ] {
        assert!(
            KernelInstrumentationReport::inspect(KernelInstrumentationPreset::Kasan, text).is_err()
        );
    }
    assert!(
        KernelInstrumentationReport::inspect(
            KernelInstrumentationPreset::Kasan,
            &"x".repeat(MAX_KERNEL_INSTRUMENTATION_CONFIG_BYTES + 1)
        )
        .is_err()
    );
    assert!(
        KernelInstrumentationReport::inspect(KernelInstrumentationPreset::Kasan, "")
            .unwrap()
            .options
            .values()
            .all(Option::is_none)
    );
}

#[test]
fn kernel_instrumentation_draft_requires_explicit_new_cfg_paths() {
    let valid = KernelInstrumentationDraft {
        config: "/build/kernel/.config".into(),
        output: "/layer/files/debug.cfg".into(),
        ..Default::default()
    };
    assert!(valid.validate().is_ok());
    for output in [
        "",
        "relative.cfg",
        "/layer/../debug.cfg",
        "/layer/.config",
        "/layer/debug.cfg\n",
    ] {
        let draft = KernelInstrumentationDraft {
            output: output.into(),
            ..valid.clone()
        };
        assert!(draft.validate().is_err());
    }
    assert!(
        KernelInstrumentationDraft {
            config: "relative".into(),
            ..valid
        }
        .validate()
        .is_err()
    );
}
