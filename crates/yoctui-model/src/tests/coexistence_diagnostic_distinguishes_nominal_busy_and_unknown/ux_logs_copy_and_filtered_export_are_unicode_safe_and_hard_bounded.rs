use super::*;

#[test]
fn ux_logs_copy_and_filtered_export_are_unicode_safe_and_hard_bounded() {
    let mut app = App::new(200, 2_000_000);
    for index in 0..100 {
        let _ = update(
            &mut app,
            Action::Log(log(&format!("{index:03}-{}", "構".repeat(40_000)))),
        );
    }
    let Some(Effect::CopyToClipboard(copy)) = update(&mut app, Action::CopySelectedLog) else {
        panic!("selected log copy must remain a typed effect");
    };
    assert!(copy.len() <= MAX_LOG_COPY_BYTES);
    assert!(copy.ends_with("[copy truncated at 64 KiB]"));

    let export = format_log_export(&app.logs);
    assert!(export.content.len() <= MAX_LOG_EXPORT_BYTES);
    assert!(export.truncated);
    assert!(export.included < 100);
    assert!(export.omitted > 0);
    assert!(export.content.ends_with("[export truncated at 256 KiB]"));
    let Some(Effect::CopyToClipboard(effect_export)) = update(&mut app, Action::ExportFilteredLogs)
    else {
        panic!("filtered export must use the typed clipboard effect");
    };
    assert_eq!(effect_export, export.content);
}
