use super::*;

fn reset() {
    PREVIEW_CACHE.with(|cache| *cache.borrow_mut() = PreviewCache::default());
}

#[test]
fn source_preview_cache_reuses_exact_highlighting_without_relexing() {
    reset();
    let app = App::new(8, 1024);
    let content = "/* multiline\n comment */\nvalue = 3;\n";
    let first = cached_source_preview(content, "board.dts", &app);
    let second = cached_source_preview(content, "board.dts", &app);
    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(*first, highlight_source_preview(content, "board.dts", &app));
}

#[test]
fn source_preview_cache_invalidates_edits_language_theme_and_color() {
    reset();
    let mut app = App::new(8, 1024);
    let content = "value = 3;\n";
    let original = cached_source_preview(content, "board.dts", &app);
    for (text, file) in [("value = 4;\n", "board.dts"), (content, "recipe.bb")] {
        let current = cached_source_preview(text, file, &app);
        assert!(!Arc::ptr_eq(&original, &current));
        assert_eq!(*current, highlight_source_preview(text, file, &app));
    }
    app.theme = yoctui_model::Theme::WhiteClassic;
    let themed = cached_source_preview(content, "board.dts", &app);
    assert!(!Arc::ptr_eq(&original, &themed));
    app.color_enabled = false;
    let monochrome = cached_source_preview(content, "board.dts", &app);
    assert!(!Arc::ptr_eq(&themed, &monochrome));
    assert_eq!(*monochrome, highlight_source_preview(content, "board.dts", &app));
}

#[test]
fn source_preview_cache_is_bounded_and_evicts_least_recently_used() {
    reset();
    let app = App::new(8, 1024);
    let first = cached_source_preview("first", "one.txt", &app);
    for i in 0..MAX_CACHE_ENTRIES {
        cached_source_preview(&format!("content {i}"), "two.txt", &app);
    }
    PREVIEW_CACHE.with(|cache| {
        let cache = cache.borrow();
        assert_eq!(cache.entries.len(), MAX_CACHE_ENTRIES);
        assert!(cache.bytes <= MAX_CACHE_BYTES);
    });
    assert!(!Arc::ptr_eq(&first, &cached_source_preview("first", "one.txt", &app)));
}

#[test]
fn source_preview_cache_does_not_retain_oversized_documents() {
    reset();
    let mut app = App::new(8, 1024);
    app.color_enabled = false;
    let content = "x".repeat(MAX_SOURCE_BYTES + 1);
    let first = cached_source_preview(&content, "large.txt", &app);
    let second = cached_source_preview(&content, "large.txt", &app);
    assert!(!Arc::ptr_eq(&first, &second));
    PREVIEW_CACHE.with(|cache| assert!(cache.borrow().entries.is_empty()));
}
