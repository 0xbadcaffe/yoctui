use super::*;

#[test]
fn demo_error_recovery_hints_keep_modifier_route_log_controls_and_confirmation() {
    let mut app = concept_failed_errors_app();
    for accessible in [false, true] {
        if accessible {
            app.preferences.symbols = SymbolPreference::Ascii;
            app.color_enabled = false;
            app.reduced_motion = true;
        }
        for width in 80..=200 {
            let output = rendered_text_at(&app, width, 50, literal_now());
            for expected in [
                "Alt+b rebuild",
                "Enter source",
                "l match",
                "o external",
                "Rebuild: review + confirmation required.",
            ] {
                assert!(
                    output.contains(expected),
                    "{width} lost {expected}: {output}"
                );
            }
            if width >= 160 {
                assert!(output.contains("[Alt+b] Rebuild options"), "{output}");
                assert!(
                    output.contains("Rebuild requires review and confirmation."),
                    "{output}"
                );
            }
            assert!(!output.contains("[B] Rebuild options"), "{output}");
            assert!(!output.contains("· B rebuild options"), "{output}");
        }
        let minimum = rendered_text_at(&app, 80, 24, literal_now());
        assert!(minimum.contains("Build errors"), "{minimum}");
        assert!(!minimum.contains("[B] Rebuild options"), "{minimum}");
    }
    assert_eq!(app.screen, Screen::Errors);
    assert_eq!(app.build.status, BuildStatus::Failed);
    assert_eq!(app.build.exit_code, Some(1));
    assert!(app.active_dialog().is_none());
}
