//! Log state.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogTimeRange {
    All,
    LastMinute,
    LastFiveMinutes,
    LastHour,
}

impl LogTimeRange {
    pub const fn label(&self) -> &'static str {
        match self {
            Self::All => "all",
            Self::LastMinute => "1m",
            Self::LastFiveMinutes => "5m",
            Self::LastHour => "1h",
        }
    }

    pub(crate) const fn maximum_age(&self) -> Option<Duration> {
        match self {
            Self::All => None,
            Self::LastMinute => Some(Duration::from_secs(60)),
            Self::LastFiveMinutes => Some(Duration::from_secs(5 * 60)),
            Self::LastHour => Some(Duration::from_secs(60 * 60)),
        }
    }

    pub(crate) const fn next(&self) -> Self {
        match self {
            Self::All => Self::LastMinute,
            Self::LastMinute => Self::LastFiveMinutes,
            Self::LastFiveMinutes => Self::LastHour,
            Self::LastHour => Self::All,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogWindow<'a> {
    pub entries: Vec<&'a LogEntry>,
    pub start: usize,
    pub total: usize,
    pub selection: usize,
    pub maximum_horizontal_offset: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogState {
    pub entries: VecDeque<LogEntry>,
    pub(crate) normalized_messages: VecDeque<String>,
    pub max_entries: usize,
    pub max_bytes: usize,
    pub retained_bytes: usize,
    pub dropped: usize,
    pub dropped_warnings: usize,
    pub dropped_errors: usize,
    pub coalesced: usize,
    pub follow: bool,
    pub paused_len: Option<usize>,
    pub wrap: bool,
    pub filter: Option<Severity>,
    pub recipe_filter: Option<String>,
    pub task_filter: Option<String>,
    pub build_filter: Option<String>,
    pub source_filter: Option<PathBuf>,
    pub time_range: LogTimeRange,
    pub query: String,
    pub searching: bool,
    pub scroll_offset: usize,
    pub horizontal_offset: usize,
    pub selection: usize,
    pub jump_target: Option<u64>,
    pub bookmarks: BTreeSet<u64>,
    pub(crate) next_id: u64,
}
impl LogState {
    pub fn new(max_entries: usize, max_bytes: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            normalized_messages: VecDeque::new(),
            max_entries,
            max_bytes,
            retained_bytes: 0,
            dropped: 0,
            dropped_warnings: 0,
            dropped_errors: 0,
            coalesced: 0,
            follow: true,
            paused_len: None,
            wrap: false,
            filter: None,
            recipe_filter: None,
            task_filter: None,
            build_filter: None,
            source_filter: None,
            time_range: LogTimeRange::All,
            query: String::new(),
            searching: false,
            scroll_offset: 0,
            horizontal_offset: 0,
            selection: 0,
            jump_target: None,
            bookmarks: BTreeSet::new(),
            next_id: 1,
        }
    }
    pub fn insert(&mut self, entry: LogEntry) {
        self.insert_batch(std::iter::once(entry));
    }

    pub fn clear_entries(&mut self) {
        self.entries.clear();
        self.normalized_messages.clear();
        self.retained_bytes = 0;
        self.paused_len = None;
        self.selection = 0;
        self.scroll_offset = 0;
        self.horizontal_offset = 0;
        self.jump_target = None;
        self.bookmarks.clear();
    }

    pub fn insert_batch(&mut self, entries: impl IntoIterator<Item = LogEntry>) {
        let selected_id = (!self.follow)
            .then(|| self.selected().map(|entry| entry.id))
            .flatten();
        for entry in entries {
            self.insert_unreconciled(entry);
        }
        self.reconcile_selection(selected_id);
        if self.follow {
            self.selection = self.filtered().count().saturating_sub(1);
            self.scroll_offset = 0;
        }
    }

    pub(crate) fn insert_unreconciled(&mut self, mut entry: LogEntry) {
        if entry.diagnostic.is_none()
            && matches!(entry.severity, Severity::Warning | Severity::Error)
        {
            entry.diagnostic = Some(diagnostic_for_entry(&entry));
        }
        if self.max_entries == 0 || self.max_bytes == 0 {
            self.record_drop(&entry);
            return;
        }
        if self.paused_len.is_none()
            && !self.is_important(&entry)
            && self.entries.back().is_some_and(|last| {
                last.severity == entry.severity
                    && last.message == entry.message
                    && last.recipe == entry.recipe
                    && last.task == entry.task
                    && last.path == entry.path
                    && last.build == entry.build
            })
        {
            self.coalesced += 1;
            if let Some(last) = self.entries.back_mut() {
                last.timestamp = entry.timestamp;
            }
            return;
        }
        if entry.message.len() > self.max_bytes {
            let suffix = "\n[entry truncated to retention byte limit]";
            let keep = yoctui_utils::utf8_prefix(
                &entry.message,
                self.max_bytes
                    .saturating_sub(suffix.len())
                    .min(entry.message.len()),
            )
            .len();
            entry.message.truncate(keep);
            if suffix.len() <= self.max_bytes {
                entry.message.push_str(suffix);
            }
        }
        if entry.id == 0 {
            entry.id = self.next_id;
            self.next_id = self.next_id.wrapping_add(1).max(1);
        }
        let bytes = entry.message.len();
        let normalized = entry.message.to_lowercase();
        self.retained_bytes += bytes;
        self.entries.push_back(entry);
        self.normalized_messages.push_back(normalized);
        while self.entries.len() > self.max_entries || self.retained_bytes > self.max_bytes {
            let ordinary = self
                .entries
                .iter()
                .position(|candidate| !self.is_important(candidate));
            let index = ordinary.unwrap_or(0);
            let Some(old) = self.entries.remove(index) else {
                break;
            };
            let _ = self.normalized_messages.remove(index);
            if self.paused_len.is_some_and(|visible| index < visible) {
                self.paused_len = self.paused_len.map(|visible| visible.saturating_sub(1));
            }
            self.retained_bytes = self.retained_bytes.saturating_sub(old.message.len());
            self.bookmarks.remove(&old.id);
            self.record_drop(&old);
        }
    }
    pub fn filtered(&self) -> impl Iterator<Item = &LogEntry> {
        let query = self.query.to_lowercase();
        let visible_len = self.paused_len.unwrap_or(self.entries.len());
        let newest_timestamp = self
            .entries
            .iter()
            .take(visible_len)
            .map(|entry| entry.timestamp)
            .max();
        let maximum_age = self.time_range.maximum_age();
        self.entries
            .iter()
            .zip(self.normalized_messages.iter())
            .take(visible_len)
            .filter(move |(e, normalized)| {
                self.jump_target == Some(e.id)
                    || (self.filter.is_none_or(|s| s == e.severity)
                        && self
                            .recipe_filter
                            .as_ref()
                            .is_none_or(|recipe| e.recipe.as_ref() == Some(recipe))
                        && self
                            .task_filter
                            .as_ref()
                            .is_none_or(|task| e.task.as_ref() == Some(task))
                        && self
                            .build_filter
                            .as_ref()
                            .is_none_or(|build| e.build.as_ref() == Some(build))
                        && self
                            .source_filter
                            .as_ref()
                            .is_none_or(|source| e.path.as_ref() == Some(source))
                        && maximum_age.is_none_or(|maximum_age| {
                            newest_timestamp
                                .and_then(|newest| newest.duration_since(e.timestamp).ok())
                                .is_some_and(|age| age <= maximum_age)
                        })
                        && (query.is_empty() || normalized.contains(&query)))
            })
            .map(|(entry, _)| entry)
    }
    pub fn diagnostics(&self) -> impl Iterator<Item = &LogEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.diagnostic.is_some())
    }
    pub fn selected(&self) -> Option<&LogEntry> {
        self.filtered().nth(self.selection)
    }
    pub fn visible_count(&self) -> usize {
        self.filtered().count()
    }
    pub fn vertical_position(&self) -> Option<(usize, usize)> {
        let count = self.visible_count();
        (count > 0).then(|| (self.selection.min(count - 1) + 1, count))
    }
    pub fn maximum_horizontal_offset(&self) -> usize {
        self.filtered()
            .map(|entry| entry.message.chars().count())
            .max()
            .unwrap_or(0)
            .saturating_sub(1)
    }
    pub fn horizontal_position(&self) -> (usize, usize) {
        let maximum = self.maximum_horizontal_offset();
        (self.horizontal_offset.min(maximum), maximum)
    }
    pub fn match_position(&self) -> Option<(usize, usize)> {
        self.vertical_position()
    }
    pub fn window(&self, viewport: usize) -> LogWindow<'_> {
        let requested_selection = self.selection;
        let requested_end = requested_selection.saturating_add(1).max(viewport);
        let requested_start = requested_end.saturating_sub(viewport);
        let mut entries = Vec::with_capacity(viewport);
        let mut trailing = VecDeque::with_capacity(viewport);
        let mut total = 0;
        let mut maximum_message_chars = 0;
        for entry in self.filtered() {
            maximum_message_chars = maximum_message_chars.max(entry.message.chars().count());
            if (requested_start..requested_end).contains(&total) {
                entries.push(entry);
            }
            if viewport > 0 {
                if trailing.len() == viewport {
                    trailing.pop_front();
                }
                trailing.push_back(entry);
            }
            total += 1;
        }
        let selection = requested_selection.min(total.saturating_sub(1));
        let start = if requested_selection == selection {
            requested_start.min(total)
        } else {
            entries = trailing.into_iter().collect();
            total.saturating_sub(entries.len())
        };
        LogWindow {
            entries,
            start,
            total,
            selection,
            maximum_horizontal_offset: maximum_message_chars.saturating_sub(1),
        }
    }
    pub fn is_bookmarked(&self, id: u64) -> bool {
        self.bookmarks.contains(&id)
    }
    pub fn toggle_selected_bookmark(&mut self) -> bool {
        let Some(id) = self.selected().map(|entry| entry.id) else {
            return false;
        };
        if !self.bookmarks.remove(&id) {
            self.bookmarks.insert(id);
        }
        true
    }
    pub fn select_bookmark(&mut self, forward: bool) -> bool {
        let retained = self
            .entries
            .iter()
            .filter(|entry| self.bookmarks.contains(&entry.id))
            .map(|entry| entry.id)
            .collect::<Vec<_>>();
        if retained.is_empty() {
            return false;
        }
        let current = self.selected().map(|entry| entry.id);
        let position =
            current.and_then(|id| retained.iter().position(|candidate| *candidate == id));
        let target = if forward {
            retained[(position.map_or(0, |position| position + 1)) % retained.len()]
        } else {
            retained[position.map_or(retained.len() - 1, |position| {
                position.checked_sub(1).unwrap_or(retained.len() - 1)
            })]
        };
        self.jump_to(target)
    }
    pub fn jump_to(&mut self, id: u64) -> bool {
        if !self.entries.iter().any(|entry| entry.id == id) {
            return false;
        }
        self.jump_target = Some(id);
        self.follow = false;
        self.paused_len = Some(self.entries.len());
        let selection = self
            .filtered()
            .position(|entry| entry.id == id)
            .unwrap_or(0);
        let count = self.visible_count();
        self.selection = selection;
        self.scroll_offset = count.saturating_sub(selection.saturating_add(1));
        true
    }
    pub(crate) fn is_important(&self, entry: &LogEntry) -> bool {
        entry.protected
            || self.bookmarks.contains(&entry.id)
            || matches!(entry.severity, Severity::Warning | Severity::Error)
    }
    pub(crate) fn record_drop(&mut self, entry: &LogEntry) {
        self.dropped += 1;
        match entry.severity {
            Severity::Warning => self.dropped_warnings += 1,
            Severity::Error => self.dropped_errors += 1,
            Severity::Trace | Severity::Info => {}
        }
    }
    pub(crate) fn clamp_selection(&mut self) {
        self.selection = self
            .selection
            .min(self.filtered().count().saturating_sub(1));
        self.scroll_offset = self
            .filtered()
            .count()
            .saturating_sub(self.selection.saturating_add(1));
    }

    pub(crate) fn reconcile_selection(&mut self, selected_id: Option<u64>) {
        let retained_selection = selected_id
            .and_then(|selected_id| self.filtered().position(|entry| entry.id == selected_id));
        if let Some(selection) = retained_selection {
            self.selection = selection;
        }
        self.clamp_selection();
    }
}
pub(crate) fn diagnostic_for_entry(entry: &LogEntry) -> DiagnosticInfo {
    let summary = entry
        .message
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("Diagnostic without a message")
        .trim()
        .chars()
        .take(120)
        .collect();
    let mut event_metadata = vec![
        ("severity".into(), format!("{:?}", entry.severity)),
        ("protected".into(), entry.protected.to_string()),
    ];
    if let Some(build) = entry.build.as_ref() {
        event_metadata.push(("build".into(), build.clone()));
    }
    if let Some(path) = entry.path.as_ref() {
        event_metadata.push(("source".into(), path.display().to_string()));
    }
    let mut suggestions = vec!["Inspect the matching retained log context.".into()];
    if entry.path.is_some() {
        suggestions.push("Open the source log and inspect surrounding output.".into());
    }
    if entry.recipe.is_some() {
        suggestions.push("Inspect the recipe task and its metadata.".into());
    }
    DiagnosticInfo {
        category: match entry.severity {
            Severity::Warning => "BitBake warning",
            Severity::Error => "BitBake error",
            Severity::Trace | Severity::Info => "Build diagnostic",
        }
        .into(),
        summary,
        event_metadata,
        suggestions,
    }
}
