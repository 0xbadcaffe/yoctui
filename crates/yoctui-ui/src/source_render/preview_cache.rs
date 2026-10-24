// Bounded renderer-local syntax cache. Exact content keys avoid stale edits;
// whole-document lexing preserves multiline state, but happens only on a miss.
use std::{cell::RefCell, collections::VecDeque, sync::Arc};

const MAX_CACHE_BYTES: usize = 16 * 1024 * 1024;
const MAX_SOURCE_BYTES: usize = 1024 * 1024;
const MAX_CACHE_ENTRIES: usize = 4;

struct PreviewCacheEntry {
    content: String,
    file_name: String,
    theme: yoctui_model::Theme,
    color: bool,
    text: Arc<Text<'static>>,
    bytes: usize,
}

#[derive(Default)]
struct PreviewCache {
    entries: VecDeque<PreviewCacheEntry>,
    bytes: usize,
}

thread_local! {
    static PREVIEW_CACHE: RefCell<PreviewCache> = RefCell::default();
}

pub(crate) fn cached_source_preview(content: &str, file_name: &str, app: &App) -> Arc<Text<'static>> {
    PREVIEW_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(index) = cache.entries.iter().position(|entry| {
            entry.content == content && entry.file_name == file_name
                && entry.theme == app.theme && entry.color == app.color_enabled
        }) {
            let entry = cache.entries.remove(index).expect("known cache entry");
            let result = Arc::clone(&entry.text);
            cache.entries.push_back(entry);
            return result;
        }
        let text = Arc::new(highlight_source_preview(content, file_name, app));
        let bytes = content.len() + file_name.len()
            + text.lines.capacity() * size_of::<Line<'static>>()
            + text.lines.iter().map(|line| {
                line.spans.capacity() * size_of::<Span<'static>>()
                    + line.spans.iter().map(|span| span.content.len()).sum::<usize>()
            }).sum::<usize>();
        if content.len() <= MAX_SOURCE_BYTES && bytes <= MAX_CACHE_BYTES {
            while cache.bytes + bytes > MAX_CACHE_BYTES || cache.entries.len() >= MAX_CACHE_ENTRIES {
                let oldest = cache.entries.pop_front().expect("cache requires eviction");
                cache.bytes -= oldest.bytes;
            }
            cache.bytes += bytes;
            cache.entries.push_back(PreviewCacheEntry {
                content: content.into(), file_name: file_name.into(), theme: app.theme,
                color: app.color_enabled, text: Arc::clone(&text), bytes,
            });
        }
        text
    })
}

#[cfg(test)]
#[path = "../tests/source_preview_cache.rs"]
mod preview_cache_tests;
