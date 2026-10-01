/// Observable event types and interfaces.
use std::collections::BTreeMap;

/// Event kind identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EventKind(String);

impl EventKind {
    /// Creates a new event kind.
    pub fn new(kind: impl Into<String>) -> Self {
        Self(kind.into())
    }

    /// Returns the event kind as a string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// An observable event with metadata.
#[derive(Debug, Clone)]
pub struct ObservableEvent {
    /// Event kind (e.g., "terminal.opened", "plugin.activated").
    pub kind: EventKind,

    /// Event sequence number (monotonic).
    pub sequence: u64,

    /// Event payload (string-keyed values).
    pub payload: BTreeMap<String, serde_json::Value>,
}

impl ObservableEvent {
    /// Creates a new observable event.
    pub fn new(kind: EventKind, sequence: u64) -> Self {
        Self {
            kind,
            sequence,
            payload: BTreeMap::new(),
        }
    }

    /// Creates an event with payload.
    pub fn with_payload(
        kind: EventKind,
        sequence: u64,
        payload: BTreeMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            kind,
            sequence,
            payload,
        }
    }
}
