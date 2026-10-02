/// Bounded observation records and their drop accounting.
///
/// The accepted `W-71` contract names the seam record `Observation`, carrying a
/// bounded kind, a bounded attribute set, and optional plugin or subsystem
/// attribution. Exact type and field spellings remain subject to the accepted
/// contract; only the shape is fixed here.
use std::collections::BTreeMap;

/// Maximum encoded length of an event kind or attribute key.
pub const MAX_KEY_LEN: usize = 128;

/// Maximum number of attributes carried by a single observation record.
pub const MAX_ATTRIBUTES: usize = 64;

/// Maximum length of a textual attribute value.
pub const MAX_ATTRIBUTE_TEXT_LEN: usize = 4096;

/// Default number of records allowed in flight for one observer.
pub const DEFAULT_MAX_RECORDS: usize = 1024;

/// Default maximum encoded size of a single observation record, in bytes.
pub const DEFAULT_MAX_RECORD_BYTES: usize = 8192;

/// Default total in-memory budget for buffered observation records, in bytes.
pub const DEFAULT_MAX_TOTAL_BYTES: usize = 4 * 1024 * 1024;

/// Approximate fixed overhead of an encoded observation record, in bytes.
const RECORD_OVERHEAD: usize = 24;

/// Truncates `value` to at most `max` bytes on a character boundary.
fn truncate_chars(value: String, max: usize) -> (String, bool) {
    if value.len() <= max {
        return (value, false);
    }
    let mut end = max;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    let mut truncated = value;
    truncated.truncate(end);
    (truncated, true)
}

/// Bounded event kind identifier.
///
/// Construction truncates an oversized input on a character boundary and
/// records that fact, so a truncation is never silent.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EventKind {
    value: String,
    truncated: bool,
}

impl EventKind {
    /// Creates a new event kind, bounded to [`MAX_KEY_LEN`] bytes.
    pub fn new(kind: impl Into<String>) -> Self {
        let (value, truncated) = truncate_chars(kind.into(), MAX_KEY_LEN);
        Self { value, truncated }
    }

    /// Returns the event kind as a string.
    pub fn as_str(&self) -> &str {
        &self.value
    }

    /// Returns whether the original kind exceeded the bound and was truncated.
    pub fn is_truncated(&self) -> bool {
        self.truncated
    }
}

/// A typed attribute value carried by an observation record.
///
/// The concrete wire encoding is parked by the accepted contract; this enum is
/// the in-memory shape only. [`AttributeValue::Redacted`] marks a field that
/// was redacted at emission instead of being handed to an observer in raw form.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum AttributeValue {
    /// An absent or null value.
    Null,

    /// A boolean value.
    Bool(bool),

    /// A signed integer value.
    Integer(i64),

    /// An unsigned integer value.
    Unsigned(u64),

    /// A floating-point value.
    Float(f64),

    /// A textual value, bounded to [`MAX_ATTRIBUTE_TEXT_LEN`] bytes.
    Text(String),

    /// A field redacted at emission; the raw value never leaves Core.
    Redacted,
}

impl From<bool> for AttributeValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<i64> for AttributeValue {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

impl From<u64> for AttributeValue {
    fn from(value: u64) -> Self {
        Self::Unsigned(value)
    }
}

impl From<f64> for AttributeValue {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<String> for AttributeValue {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<&str> for AttributeValue {
    fn from(value: &str) -> Self {
        Self::Text(value.to_string())
    }
}

/// Returns the encoded byte length of an attribute value.
fn value_len(value: &AttributeValue) -> usize {
    match value {
        AttributeValue::Null | AttributeValue::Bool(_) => 1,
        AttributeValue::Integer(_) | AttributeValue::Unsigned(_) | AttributeValue::Float(_) => 8,
        AttributeValue::Text(text) => text.len(),
        AttributeValue::Redacted => 8,
    }
}

/// Bounds a text value, reporting whether it was truncated.
fn bound_value(value: AttributeValue) -> (AttributeValue, bool) {
    match value {
        AttributeValue::Text(text) => {
            let (text, truncated) = truncate_chars(text, MAX_ATTRIBUTE_TEXT_LEN);
            (AttributeValue::Text(text), truncated)
        }
        other => (other, false),
    }
}

/// Optional attribution of an observation to a plugin or a Core subsystem.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Attribution {
    /// A named Core subsystem.
    Subsystem(String),

    /// A plugin identified by its id.
    Plugin(String),
}

impl Attribution {
    /// Creates a subsystem attribution, bounded to [`MAX_KEY_LEN`] bytes.
    pub fn subsystem(name: impl Into<String>) -> Self {
        Self::Subsystem(truncate_chars(name.into(), MAX_KEY_LEN).0)
    }

    /// Creates a plugin attribution, bounded to [`MAX_KEY_LEN`] bytes.
    pub fn plugin(id: impl Into<String>) -> Self {
        Self::Plugin(truncate_chars(id.into(), MAX_KEY_LEN).0)
    }

    /// Returns `"subsystem"` or `"plugin"`.
    pub fn scope(&self) -> &'static str {
        match self {
            Self::Subsystem(_) => "subsystem",
            Self::Plugin(_) => "plugin",
        }
    }

    /// Returns the attributed subsystem or plugin name.
    pub fn name(&self) -> &str {
        match self {
            Self::Subsystem(name) | Self::Plugin(name) => name,
        }
    }
}

/// Whether an observation record was stored intact or truncated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RecordStatus {
    /// The record carries every attribute it was built with.
    #[default]
    Complete,

    /// At least one attribute was dropped or truncated to respect a bound.
    Truncated,
}

/// A bounded, read-only observation record.
///
/// Candidate seam type named `Observation` by the accepted `W-71` contract; the
/// exact spelling remains subject to that contract. A record carries a bounded
/// kind, a bounded attribute set, and optional attribution. It never carries a
/// raw PTY byte stream, a secret, or a capability handle.
#[derive(Debug, Clone, PartialEq)]
pub struct Observation {
    /// Event kind.
    pub kind: EventKind,

    /// Monotonic event sequence number.
    pub sequence: u64,

    /// Monotonic timestamp in milliseconds.
    pub timestamp: u64,

    /// Bounded attribute set, keyed by field name.
    attributes: BTreeMap<String, AttributeValue>,

    /// Optional plugin or subsystem attribution.
    pub attribution: Option<Attribution>,

    status: RecordStatus,
}

impl Observation {
    /// Creates a record with an empty attribute set and a zero timestamp.
    pub fn new(kind: EventKind, sequence: u64) -> Self {
        Self::with_timestamp(kind, sequence, 0)
    }

    /// Creates a record with the given kind, sequence, and timestamp.
    pub fn with_timestamp(kind: EventKind, sequence: u64, timestamp: u64) -> Self {
        let status = if kind.is_truncated() {
            RecordStatus::Truncated
        } else {
            RecordStatus::Complete
        };
        Self {
            kind,
            sequence,
            timestamp,
            attributes: BTreeMap::new(),
            attribution: None,
            status,
        }
    }

    /// Sets the plugin or subsystem attribution.
    pub fn with_attribution(mut self, attribution: Attribution) -> Self {
        self.attribution = Some(attribution);
        self
    }

    /// Inserts one attribute, enforcing the attribute-count, key-length, and
    /// value-length bounds, and returns the resulting record status.
    ///
    /// When the attribute count is already at [`MAX_ATTRIBUTES`] and the key is
    /// new, the attribute is dropped and the record is marked truncated.
    pub fn insert_attribute(
        &mut self,
        key: impl Into<String>,
        value: AttributeValue,
    ) -> RecordStatus {
        let (key, key_truncated) = truncate_chars(key.into(), MAX_KEY_LEN);
        let (value, value_truncated) = bound_value(value);
        if key_truncated || value_truncated {
            self.status = RecordStatus::Truncated;
        }
        if self.attributes.len() >= MAX_ATTRIBUTES && !self.attributes.contains_key(&key) {
            self.status = RecordStatus::Truncated;
            return self.status;
        }
        self.attributes.insert(key, value);
        self.status
    }

    /// Builder form of [`Observation::insert_attribute`].
    pub fn with_attribute(mut self, key: impl Into<String>, value: AttributeValue) -> Self {
        self.insert_attribute(key, value);
        self
    }

    /// Returns the bounded attribute set.
    pub fn attributes(&self) -> &BTreeMap<String, AttributeValue> {
        &self.attributes
    }

    /// Returns whether the record is complete or truncated.
    pub fn status(&self) -> RecordStatus {
        self.status
    }

    /// Marks the record as truncated.
    pub fn mark_truncated(&mut self) {
        self.status = RecordStatus::Truncated;
    }

    /// Returns the approximate encoded byte size of the record.
    pub fn size_hint(&self) -> usize {
        let mut total = RECORD_OVERHEAD.saturating_add(self.kind.as_str().len());
        for (key, value) in &self.attributes {
            total = total
                .saturating_add(key.len())
                .saturating_add(value_len(value));
        }
        if let Some(attribution) = &self.attribution {
            total = total.saturating_add(attribution.name().len());
        }
        total
    }

    /// Enforces a maximum encoded byte size by dropping attributes (in key
    /// order) until the record fits.
    ///
    /// Returns `true` when the record fits within `max_bytes` after
    /// truncation, and `false` when it does not (the record cannot be stored).
    pub fn enforce_size_bound(&mut self, max_bytes: usize) -> bool {
        while self.size_hint() > max_bytes {
            if self.attributes.pop_last().is_none() {
                return self.size_hint() <= max_bytes;
            }
            self.status = RecordStatus::Truncated;
        }
        true
    }
}

/// Explicit accounting of observation data that did not survive buffering.
///
/// A drop or truncation is surfaced to the observer so it can distinguish
/// "no data" from "data dropped".
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DropReport {
    /// Whole records evicted (oldest first) to respect a record-count or total
    /// byte budget.
    pub dropped: u64,

    /// Records whose attributes were truncated to respect the record-size
    /// bound.
    pub truncated: u64,

    /// Records rejected outright because they could not fit any bound.
    pub rejected: u64,
}

impl DropReport {
    /// Returns the total number of records dropped or rejected.
    pub fn total(&self) -> u64 {
        self.dropped.saturating_add(self.rejected)
    }

    /// Returns whether any data was dropped, truncated, or rejected.
    pub fn is_empty(&self) -> bool {
        self.dropped == 0 && self.truncated == 0 && self.rejected == 0
    }

    /// Adds the counts of another report into this one.
    pub fn merge(&mut self, other: &Self) {
        self.dropped = self.dropped.saturating_add(other.dropped);
        self.truncated = self.truncated.saturating_add(other.truncated);
        self.rejected = self.rejected.saturating_add(other.rejected);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oversized_kind_is_truncated_and_flagged() {
        let kind = EventKind::new("x".repeat(MAX_KEY_LEN + 10));
        assert_eq!(kind.as_str().len(), MAX_KEY_LEN);
        assert!(kind.is_truncated());
    }

    #[test]
    fn oversize_record_truncates_attributes_explicitly() {
        let mut observation = Observation::new(EventKind::new("terminal.opened"), 1);
        for index in 0..8 {
            observation.insert_attribute(
                format!("field{index}"),
                AttributeValue::Text("v".repeat(64)),
            );
        }
        assert!(observation.size_hint() > 64);
        assert!(observation.enforce_size_bound(64));
        assert_eq!(observation.status(), RecordStatus::Truncated);
    }

    #[test]
    fn attribute_count_is_bounded() {
        let mut observation = Observation::new(EventKind::new("plugin.activated"), 1);
        for index in 0..MAX_ATTRIBUTES {
            observation.insert_attribute(format!("field{index}"), AttributeValue::Bool(true));
        }
        observation.insert_attribute("overflow", AttributeValue::Bool(true));

        assert_eq!(observation.attributes().len(), MAX_ATTRIBUTES);
        assert!(!observation.attributes().contains_key("overflow"));
        assert_eq!(observation.status(), RecordStatus::Truncated);
    }

    #[test]
    fn drop_report_merges_counts() {
        let mut left = DropReport {
            dropped: 1,
            truncated: 2,
            rejected: 0,
        };
        left.merge(&DropReport {
            dropped: 3,
            truncated: 0,
            rejected: 4,
        });

        assert_eq!(left.dropped, 4);
        assert_eq!(left.truncated, 2);
        assert_eq!(left.rejected, 4);
        assert_eq!(left.total(), 8);
        assert!(!left.is_empty());
    }
}
