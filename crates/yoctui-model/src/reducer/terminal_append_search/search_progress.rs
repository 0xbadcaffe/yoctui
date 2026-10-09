use super::*;

pub(super) fn append(app: &mut App, generation: u64, query: String, hits: Vec<GlobalSearchHit>) {
    if generation != app.global_search_generation
        || query != app.command_palette_query
        || app.command_palette_mode != CommandPaletteMode::GlobalRegexSearch
        || !app.global_search_content.loading()
    {
        return;
    }
    let mut accumulated = app.global_search_content.hits().to_vec();
    for hit in hits {
        if accumulated.len() >= MAX_GLOBAL_SEARCH_HITS {
            break;
        }
        if hit.path.is_absolute()
            && hit.line > 0
            && hit.column > 0
            && !hit.preview.chars().any(char::is_control)
            && !accumulated.iter().any(|existing| {
                existing.path == hit.path
                    && existing.line == hit.line
                    && existing.column == hit.column
            })
        {
            accumulated.push(hit);
        }
    }
    app.global_search_content = GlobalSearchContentState::Streaming {
        generation,
        query,
        hits: accumulated,
    };
}
