//! Bounded tracing capture for the interactive client's self-diagnostic view.

use std::{
    fmt::{self, Write as _},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError},
    },
    time::SystemTime,
};
use tracing::{Event, Subscriber, field::Visit};
use tracing_subscriber::{Layer, layer::Context};
use yoctui_model::{InternalLogLevel, InternalLogRecord};

const MAX_CAPTURED_EVENT_BYTES: usize = 64 * 1024;

#[derive(Clone)]
pub struct InternalTracingLayer {
    sender: SyncSender<InternalLogRecord>,
    dropped: Arc<AtomicUsize>,
}

pub struct InternalTracingCapture {
    receiver: Receiver<InternalLogRecord>,
    dropped: Arc<AtomicUsize>,
}

pub fn bounded_channel(capacity: usize) -> (InternalTracingLayer, InternalTracingCapture) {
    let (sender, receiver) = mpsc::sync_channel(capacity.max(1));
    let dropped = Arc::new(AtomicUsize::new(0));
    (
        InternalTracingLayer {
            sender,
            dropped: Arc::clone(&dropped),
        },
        InternalTracingCapture { receiver, dropped },
    )
}

impl InternalTracingCapture {
    pub fn drain(&mut self, maximum: usize) -> (Vec<InternalLogRecord>, usize) {
        let mut records = Vec::with_capacity(maximum.min(256));
        for _ in 0..maximum {
            match self.receiver.try_recv() {
                Ok(record) => records.push(record),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
        let dropped = self.dropped.swap(0, Ordering::Relaxed);
        (records, dropped)
    }
}

impl<S> Layer<S> for InternalTracingLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _context: Context<'_, S>) {
        let metadata = event.metadata();
        let mut visitor = DiagnosticVisitor::default();
        event.record(&mut visitor);
        let record = InternalLogRecord {
            id: 0,
            timestamp: SystemTime::now(),
            level: match *metadata.level() {
                tracing::Level::TRACE => InternalLogLevel::Trace,
                tracing::Level::DEBUG => InternalLogLevel::Debug,
                tracing::Level::INFO => InternalLogLevel::Info,
                tracing::Level::WARN => InternalLogLevel::Warning,
                tracing::Level::ERROR => InternalLogLevel::Error,
            },
            target: metadata.target().to_owned(),
            message: visitor.finish(),
        };
        if let Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) =
            self.sender.try_send(record)
        {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }
}

#[derive(Default)]
struct DiagnosticVisitor {
    message: Option<String>,
    fields: Vec<String>,
    retained_bytes: usize,
}

impl DiagnosticVisitor {
    fn record_value(&mut self, field: &tracing::field::Field, value: &str) {
        let value = if field.name() == "message" {
            value.to_owned()
        } else {
            format!("{}={value}", field.name())
        };
        let available = MAX_CAPTURED_EVENT_BYTES.saturating_sub(self.retained_bytes);
        let value = bounded_text(&value, available);
        self.retained_bytes = self.retained_bytes.saturating_add(value.len());
        if field.name() == "message" {
            self.message = Some(value);
        } else if !value.is_empty() {
            self.fields.push(value);
        }
    }

    fn record_debug_value(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        let mut rendered =
            BoundedString::new(MAX_CAPTURED_EVENT_BYTES.saturating_sub(self.retained_bytes));
        let _ = write!(&mut rendered, "{value:?}");
        self.record_value(field, &rendered.value);
    }

    fn finish(self) -> String {
        let value = match (self.message, self.fields.is_empty()) {
            (Some(message), true) => message,
            (Some(message), false) => format!("{message} · {}", self.fields.join(" · ")),
            (None, false) => self.fields.join(" · "),
            (None, true) => "event without fields".into(),
        };
        bounded_text(&value, MAX_CAPTURED_EVENT_BYTES)
    }
}

impl Visit for DiagnosticVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.record_value(field, value);
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        self.record_debug_value(field, value);
    }
}

struct BoundedString {
    value: String,
    maximum_bytes: usize,
}

impl BoundedString {
    fn new(maximum_bytes: usize) -> Self {
        Self {
            value: String::with_capacity(maximum_bytes.min(1024)),
            maximum_bytes,
        }
    }
}

impl fmt::Write for BoundedString {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        let available = self.maximum_bytes.saturating_sub(self.value.len());
        let mut keep = available.min(value.len());
        while keep > 0 && !value.is_char_boundary(keep) {
            keep -= 1;
        }
        self.value.push_str(&value[..keep]);
        Ok(())
    }
}

fn bounded_text(value: &str, maximum_bytes: usize) -> String {
    if value.len() <= maximum_bytes {
        return value.to_owned();
    }
    let marker = " [truncated]";
    let marker = if marker.len() <= maximum_bytes {
        marker
    } else {
        ""
    };
    let mut keep = maximum_bytes.saturating_sub(marker.len()).min(value.len());
    while keep > 0 && !value.is_char_boundary(keep) {
        keep -= 1;
    }
    format!("{}{}", &value[..keep], marker)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::prelude::*;

    #[test]
    fn ux_internal_log_capture_is_typed_bounded_and_reports_channel_loss() {
        let (layer, mut capture) = bounded_channel(1);
        let subscriber = tracing_subscriber::registry().with(layer);
        tracing::subscriber::with_default(subscriber, || {
            tracing::warn!(request = 7, value = %"日".repeat(100_000), "adapter warning");
            tracing::error!("overflowed event");
        });
        let (records, dropped) = capture.drain(8);
        assert_eq!(records.len(), 1);
        assert_eq!(dropped, 1);
        assert_eq!(records[0].level, InternalLogLevel::Warning);
        assert!(records[0].target.contains("internal_tracing"));
        assert!(records[0].message.contains("adapter warning"));
        assert!(records[0].message.len() <= MAX_CAPTURED_EVENT_BYTES);
        assert!(!records[0].message.contains('\u{fffd}'));
    }
}
