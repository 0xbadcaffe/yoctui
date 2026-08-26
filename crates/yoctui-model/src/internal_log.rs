//! Bounded Yoctui self-diagnostics, deliberately separate from BitBake logs.

use std::{collections::VecDeque, time::SystemTime};

pub const MAX_INTERNAL_LOG_TARGET_BYTES: usize = 512;
pub const MAX_INTERNAL_LOG_MESSAGE_BYTES: usize = 64 * 1024;
pub const MAX_INTERNAL_LOG_EXPORT_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LogWorkspaceView {
    #[default]
    BitBake,
    Yoctui,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InternalLogLevel {
    Trace,
    Debug,
    Info,
    Warning,
    Error,
}

impl InternalLogLevel {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Trace => "Trace",
            Self::Debug => "Debug",
            Self::Info => "Info",
            Self::Warning => "Warning",
            Self::Error => "Error",
        }
    }

    pub const fn next_filter(current: Option<Self>) -> Option<Self> {
        match current {
            None => Some(Self::Trace),
            Some(Self::Trace) => Some(Self::Debug),
            Some(Self::Debug) => Some(Self::Info),
            Some(Self::Info) => Some(Self::Warning),
            Some(Self::Warning) => Some(Self::Error),
            Some(Self::Error) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InternalLogRecord {
    pub id: u64,
    pub timestamp: SystemTime,
    pub level: InternalLogLevel,
    pub target: String,
    pub message: String,
}

impl InternalLogRecord {
    fn retained_bytes(&self) -> usize {
        self.target.len().saturating_add(self.message.len())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InternalLogWindow<'a> {
    pub entries: Vec<&'a InternalLogRecord>,
    pub start: usize,
    pub total: usize,
    pub selection: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InternalLogExport {
    pub content: String,
    pub included: usize,
    pub omitted: usize,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InternalLogState {
    pub entries: VecDeque<InternalLogRecord>,
    pub max_entries: usize,
    pub max_bytes: usize,
    pub retained_bytes: usize,
    pub evicted: usize,
    pub ingress_dropped: usize,
    pub follow: bool,
    pub paused_len: Option<usize>,
    pub level_filter: Option<InternalLogLevel>,
    pub target_filter: Option<String>,
    pub query: String,
    pub searching: bool,
    pub selection: usize,
    next_id: u64,
}

impl InternalLogState {
    pub fn new(max_entries: usize, max_bytes: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            max_entries,
            max_bytes,
            retained_bytes: 0,
            evicted: 0,
            ingress_dropped: 0,
            follow: true,
            paused_len: None,
            level_filter: None,
            target_filter: None,
            query: String::new(),
            searching: false,
            selection: 0,
            next_id: 1,
        }
    }

    pub fn insert(&mut self, mut record: InternalLogRecord) {
        let selected_id = (!self.follow)
            .then(|| self.selected().map(|entry| entry.id))
            .flatten();
        record.target = bounded_text(
            &record.target,
            MAX_INTERNAL_LOG_TARGET_BYTES.min(self.max_bytes),
            " [target truncated]",
        );
        let message_limit =
            MAX_INTERNAL_LOG_MESSAGE_BYTES.min(self.max_bytes.saturating_sub(record.target.len()));
        record.message = bounded_text(&record.message, message_limit, "\n[event truncated]");
        if self.max_entries == 0 || self.max_bytes == 0 {
            self.evicted = self.evicted.saturating_add(1);
            return;
        }
        if record.id == 0 {
            record.id = self.next_id;
            self.next_id = self.next_id.wrapping_add(1).max(1);
        }
        self.retained_bytes = self.retained_bytes.saturating_add(record.retained_bytes());
        self.entries.push_back(record);
        while self.entries.len() > self.max_entries || self.retained_bytes > self.max_bytes {
            let Some(evicted) = self.entries.pop_front() else {
                break;
            };
            self.retained_bytes = self.retained_bytes.saturating_sub(evicted.retained_bytes());
            self.paused_len = self.paused_len.map(|visible| visible.saturating_sub(1));
            self.evicted = self.evicted.saturating_add(1);
        }
        self.reconcile_selection(selected_id);
        if self.follow {
            self.selection = self.visible_count().saturating_sub(1);
        }
    }

    pub fn note_ingress_dropped(&mut self, count: usize) {
        self.ingress_dropped = self.ingress_dropped.saturating_add(count);
    }

    pub fn filtered(&self) -> impl Iterator<Item = &InternalLogRecord> {
        let query = self.query.to_lowercase();
        let visible_len = self.paused_len.unwrap_or(self.entries.len());
        self.entries.iter().take(visible_len).filter(move |entry| {
            self.level_filter.is_none_or(|level| entry.level == level)
                && self
                    .target_filter
                    .as_ref()
                    .is_none_or(|target| &entry.target == target)
                && (query.is_empty()
                    || entry.message.to_lowercase().contains(&query)
                    || entry.target.to_lowercase().contains(&query))
        })
    }

    pub fn selected(&self) -> Option<&InternalLogRecord> {
        self.filtered().nth(self.selection)
    }

    pub fn visible_count(&self) -> usize {
        self.filtered().count()
    }

    pub fn vertical_position(&self) -> Option<(usize, usize)> {
        let count = self.visible_count();
        (count > 0).then(|| (self.selection.min(count - 1) + 1, count))
    }

    pub fn window(&self, viewport: usize) -> InternalLogWindow<'_> {
        let total = self.visible_count();
        let selection = self.selection.min(total.saturating_sub(1));
        let end = selection.saturating_add(1).max(viewport).min(total);
        let start = end.saturating_sub(viewport);
        InternalLogWindow {
            entries: self.filtered().skip(start).take(viewport).collect(),
            start,
            total,
            selection,
        }
    }

    pub fn scroll(&mut self, delta: isize) {
        if self.follow {
            self.follow = false;
            self.paused_len = Some(self.entries.len());
        }
        self.selection = shifted_index(self.selection, delta, self.visible_count());
    }

    pub fn toggle_follow(&mut self) {
        self.follow = !self.follow;
        if self.follow {
            self.paused_len = None;
            self.selection = self.visible_count().saturating_sub(1);
        } else {
            self.paused_len = Some(self.entries.len());
        }
    }

    pub fn cycle_level_filter(&mut self) {
        let selected_id = self.selected().map(|entry| entry.id);
        self.level_filter = InternalLogLevel::next_filter(self.level_filter);
        self.reconcile_selection(selected_id);
    }

    pub fn cycle_target_filter(&mut self) {
        let selected_id = self.selected().map(|entry| entry.id);
        let mut targets = self
            .entries
            .iter()
            .map(|entry| entry.target.clone())
            .collect::<Vec<_>>();
        targets.sort();
        targets.dedup();
        self.target_filter = match self.target_filter.take() {
            None => targets.first().cloned(),
            Some(current) => targets
                .iter()
                .position(|target| target == &current)
                .and_then(|index| targets.get(index + 1))
                .cloned(),
        };
        self.reconcile_selection(selected_id);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.retained_bytes = 0;
        self.paused_len = None;
        self.selection = 0;
    }

    pub(crate) fn reconcile_selection(&mut self, selected_id: Option<u64>) {
        if let Some(selection) =
            selected_id.and_then(|id| self.filtered().position(|entry| entry.id == id))
        {
            self.selection = selection;
        }
        self.selection = self.selection.min(self.visible_count().saturating_sub(1));
    }
}

pub fn format_internal_log_export(logs: &InternalLogState) -> InternalLogExport {
    let total = logs.visible_count();
    let marker = "\n[internal diagnostic export truncated at 256 KiB]";
    let limit = MAX_INTERNAL_LOG_EXPORT_BYTES.saturating_sub(marker.len());
    let mut content = String::with_capacity(MAX_INTERNAL_LOG_EXPORT_BYTES.min(logs.retained_bytes));
    let header = format!(
        "Yoctui internal diagnostic export\nVisible entries: {total}\nRetention evicted: {}\nIngress dropped: {}\n\n",
        logs.evicted, logs.ingress_dropped
    );
    let mut truncated = !push_bounded(&mut content, &header, limit);
    let mut included = 0;
    if !truncated {
        for entry in logs.filtered() {
            let value = format!(
                "--- {} · {} · {} ---\n{}\n\n",
                entry.id,
                entry.level.label(),
                entry.target,
                entry.message
            );
            if !push_bounded(&mut content, &value, limit) {
                truncated = true;
                break;
            }
            included += 1;
        }
    }
    if truncated {
        append_marker(&mut content, marker, MAX_INTERNAL_LOG_EXPORT_BYTES);
    }
    InternalLogExport {
        content,
        included,
        omitted: total.saturating_sub(included),
        truncated,
    }
}

fn shifted_index(current: usize, delta: isize, count: usize) -> usize {
    if count == 0 {
        return 0;
    }
    current.saturating_add_signed(delta).min(count - 1)
}

fn bounded_text(value: &str, maximum_bytes: usize, marker: &str) -> String {
    if value.len() <= maximum_bytes {
        return value.to_owned();
    }
    let marker = if marker.len() <= maximum_bytes {
        marker
    } else {
        ""
    };
    let mut keep = maximum_bytes.saturating_sub(marker.len()).min(value.len());
    while keep > 0 && !value.is_char_boundary(keep) {
        keep -= 1;
    }
    let mut output = value[..keep].to_owned();
    output.push_str(marker);
    output
}

fn push_bounded(output: &mut String, value: &str, maximum_bytes: usize) -> bool {
    let available = maximum_bytes.saturating_sub(output.len());
    if value.len() <= available {
        output.push_str(value);
        return true;
    }
    let mut keep = available.min(value.len());
    while keep > 0 && !value.is_char_boundary(keep) {
        keep -= 1;
    }
    output.push_str(&value[..keep]);
    false
}

fn append_marker(output: &mut String, marker: &str, maximum_bytes: usize) {
    if marker.len() > maximum_bytes {
        return;
    }
    if output.len().saturating_add(marker.len()) > maximum_bytes {
        let mut keep = maximum_bytes - marker.len();
        while keep > 0 && !output.is_char_boundary(keep) {
            keep -= 1;
        }
        output.truncate(keep);
    }
    output.push_str(marker);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Action, App, Effect, LogEntry, LogWorkspaceView, Severity, update};
    use std::time::Duration;

    fn record(index: usize, level: InternalLogLevel, target: &str) -> InternalLogRecord {
        InternalLogRecord {
            id: 0,
            timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(index as u64),
            level,
            target: target.into(),
            message: format!("diagnostic-{index:04}-{}", "界".repeat(40)),
        }
    }

    #[test]
    fn ux_internal_log_state_is_bounded_filterable_and_viewport_only() {
        let mut logs = InternalLogState::new(128, 64 * 1024);
        for index in 0..10_000 {
            logs.insert(record(
                index,
                if index % 7 == 0 {
                    InternalLogLevel::Warning
                } else {
                    InternalLogLevel::Debug
                },
                if index % 2 == 0 {
                    "yoctui::runtime"
                } else {
                    "yoctui::adapter"
                },
            ));
        }
        assert_eq!(logs.entries.len(), 128);
        assert!(logs.retained_bytes <= logs.max_bytes);
        assert_eq!(logs.evicted, 10_000 - 128);
        assert_eq!(logs.window(6).entries.len(), 6);

        logs.level_filter = Some(InternalLogLevel::Warning);
        logs.target_filter = Some("yoctui::runtime".into());
        logs.query = "diagnostic".into();
        assert!(logs.filtered().all(|entry| {
            entry.level == InternalLogLevel::Warning && entry.target == "yoctui::runtime"
        }));
        logs.note_ingress_dropped(9);
        assert_eq!(logs.ingress_dropped, 9);
    }

    #[test]
    fn ux_internal_log_reducer_keeps_bitbake_authority_separate_and_export_bounded() {
        let mut app = App::new(64, 512 * 1024);
        let bitbake = LogEntry {
            id: 0,
            severity: Severity::Error,
            message: "BitBake domain failure".into(),
            recipe: Some("busybox".into()),
            task: Some("do_compile".into()),
            path: None,
            timestamp: SystemTime::UNIX_EPOCH,
            build: Some("core-image-minimal".into()),
            protected: true,
            diagnostic: None,
        };
        let _ = update(&mut app, Action::Log(bitbake));
        for index in 0..64 {
            let mut entry = record(index, InternalLogLevel::Info, "yoctui::client");
            entry.message = format!("{index:03}-{}", "構".repeat(30_000));
            let _ = update(&mut app, Action::InternalLog(entry));
        }
        assert_eq!(app.logs.entries.len(), 1);
        assert!(
            app.internal_logs
                .entries
                .iter()
                .all(|entry| !entry.message.contains("BitBake domain failure"))
        );

        let _ = update(&mut app, Action::CycleLogWorkspaceView);
        assert_eq!(app.log_workspace_view, LogWorkspaceView::Yoctui);
        let Some(Effect::CopyToClipboard(export)) = update(&mut app, Action::ExportInternalLogs)
        else {
            panic!("internal export must use the existing typed clipboard authority");
        };
        assert!(export.len() <= MAX_INTERNAL_LOG_EXPORT_BYTES);
        assert!(export.ends_with("[internal diagnostic export truncated at 256 KiB]"));

        let evicted = app.internal_logs.evicted;
        let _ = update(&mut app, Action::ClearInternalLogs);
        assert!(app.internal_logs.entries.is_empty());
        assert_eq!(app.internal_logs.evicted, evicted);
        assert_eq!(app.logs.entries.len(), 1);
    }
}
