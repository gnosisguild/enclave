use chrono::Utc;
use parking_lot::Mutex;
use serde::Serialize;
use std::collections::VecDeque;
use std::fmt;
use std::sync::Arc;
use tracing::field::{Field, Visit};
use tracing::Subscriber;
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

#[derive(Clone, Debug, Serialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
}

pub type SharedLogBuffer = Arc<Mutex<VecDeque<LogEntry>>>;

pub struct DashboardTracingLayer {
    buffer: SharedLogBuffer,
    max_entries: usize,
}

impl DashboardTracingLayer {
    pub fn new(max_entries: usize) -> (Self, SharedLogBuffer) {
        let buffer = Arc::new(Mutex::new(VecDeque::with_capacity(max_entries)));
        let handle = Arc::clone(&buffer);
        (
            Self {
                buffer,
                max_entries,
            },
            handle,
        )
    }
}

struct MessageVisitor {
    message: String,
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            // Use the alternate Debug format which, for &str values forwarded
            // through tracing macros, produces the string without surrounding
            // quotes (via the Display impl). We trim any remaining quotes as a
            // safety net.
            let formatted = format!("{:?}", value);
            self.message = formatted
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .unwrap_or(&formatted)
                .to_string();
        } else if self.message.is_empty() {
            self.message = format!("{}={:?}", field.name(), value);
        } else {
            self.message
                .push_str(&format!(" {}={:?}", field.name(), value));
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        } else if self.message.is_empty() {
            self.message = format!("{}={}", field.name(), value);
        } else {
            self.message
                .push_str(&format!(" {}={}", field.name(), value));
        }
    }
}

impl<S: Subscriber> Layer<S> for DashboardTracingLayer {
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let mut visitor = MessageVisitor {
            message: String::new(),
        };
        event.record(&mut visitor);

        let entry = LogEntry {
            timestamp: Utc::now().to_rfc3339(),
            level: metadata.level().to_string(),
            target: metadata.target().to_string(),
            message: visitor.message,
        };

        let mut buf = self.buffer.lock();
        if buf.len() >= self.max_entries {
            buf.pop_front();
        }
        buf.push_back(entry);
    }
}
