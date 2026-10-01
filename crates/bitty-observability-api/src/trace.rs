/// Trace configuration and record types.
use std::collections::BTreeMap;

/// Trace configuration.
#[derive(Debug, Clone)]
pub struct TraceConfig {
    /// Whether tracing is enabled.
    pub enabled: bool,

    /// Topic filter (e.g., "terminal.*").
    pub filter: Option<String>,

    /// Maximum number of events to buffer.
    pub max_events: usize,
}

impl Default for TraceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            filter: None,
            max_events: 1000,
        }
    }
}

/// A captured trace record.
#[derive(Debug, Clone)]
pub struct TraceRecord {
    /// Event topic.
    pub topic: String,

    /// Event sequence number.
    pub sequence: u64,

    /// Monotonic timestamp (milliseconds).
    pub timestamp: u64,

    /// Event payload (may be redacted or truncated).
    pub payload: BTreeMap<String, serde_json::Value>,
}

impl TraceRecord {
    /// Creates a new trace record.
    pub fn new(
        topic: String,
        sequence: u64,
        timestamp: u64,
        payload: BTreeMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            topic,
            sequence,
            timestamp,
            payload,
        }
    }
}
