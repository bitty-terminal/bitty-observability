/// Bounded trace buffer management.
///
/// Every bound is fixed and attributable: a record-count bound, a per-record
/// byte bound, and a total in-memory byte budget. Non-critical records are
/// dropped oldest-first, truncations are explicit, and an inert buffer (no
/// observer attached) stores nothing.
use bitty_observability_api::{
    DEFAULT_MAX_RECORD_BYTES, DEFAULT_MAX_RECORDS, DEFAULT_MAX_TOTAL_BYTES, DropReport, Observation,
};
use std::collections::VecDeque;

/// Fixed bounds for a trace record buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferLimits {
    /// Maximum number of records held in flight.
    pub max_records: usize,

    /// Maximum encoded size of a single record, in bytes.
    pub max_record_bytes: usize,

    /// Maximum total in-memory budget for buffered records, in bytes.
    pub max_total_bytes: usize,
}

impl BufferLimits {
    /// Creates explicit buffer bounds.
    pub const fn new(max_records: usize, max_record_bytes: usize, max_total_bytes: usize) -> Self {
        Self {
            max_records,
            max_record_bytes,
            max_total_bytes,
        }
    }

    /// Creates bounds for an inert buffer: nothing is stored.
    pub const fn inert() -> Self {
        Self {
            max_records: 0,
            max_record_bytes: 0,
            max_total_bytes: 0,
        }
    }
}

impl Default for BufferLimits {
    fn default() -> Self {
        Self {
            max_records: DEFAULT_MAX_RECORDS,
            max_record_bytes: DEFAULT_MAX_RECORD_BYTES,
            max_total_bytes: DEFAULT_MAX_TOTAL_BYTES,
        }
    }
}

/// The result of pushing one record into a bounded buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushOutcome {
    /// Stored intact.
    Stored,

    /// Stored after evicting the oldest record.
    EvictedOldest,

    /// Stored after truncating the record to the record-size bound.
    Truncated,

    /// Stored after evicting the oldest record and truncating this one.
    EvictedAndTruncated,

    /// Not stored because the record could not fit even truncated.
    Rejected,

    /// Not stored because the buffer is inert (no observer attached).
    Inert,
}

/// Ring buffer for bounded observation records.
pub struct TraceBuffer {
    records: VecDeque<Observation>,
    limits: BufferLimits,
    bytes: usize,
    report: DropReport,
}

impl TraceBuffer {
    /// Creates a buffer with the given fixed bounds.
    pub fn new(limits: BufferLimits) -> Self {
        let capacity = limits.max_records.min(1024);
        Self {
            records: VecDeque::with_capacity(capacity),
            limits,
            bytes: 0,
            report: DropReport::default(),
        }
    }

    /// Creates an inert buffer that stores nothing.
    ///
    /// With no observer attached, the buffer is inert and no record is kept.
    pub fn inert() -> Self {
        Self::new(BufferLimits::inert())
    }

    /// Pushes a record, dropping the oldest records when a bound is exceeded
    /// and truncating the record when it exceeds the record-size bound.
    ///
    /// The returned outcome makes the eviction or truncation explicit.
    pub fn push(&mut self, mut record: Observation) -> PushOutcome {
        if self.limits.max_records == 0 {
            return PushOutcome::Inert;
        }

        let mut truncated = false;
        if record.size_hint() > self.limits.max_record_bytes {
            if !record.enforce_size_bound(self.limits.max_record_bytes) {
                self.report.rejected = self.report.rejected.saturating_add(1);
                return PushOutcome::Rejected;
            }
            truncated = true;
            self.report.truncated = self.report.truncated.saturating_add(1);
        }

        let record_bytes = record.size_hint();
        if record_bytes > self.limits.max_total_bytes {
            self.report.rejected = self.report.rejected.saturating_add(1);
            return PushOutcome::Rejected;
        }

        let mut evicted = false;
        while self.records.len() >= self.limits.max_records
            || self.bytes.saturating_add(record_bytes) > self.limits.max_total_bytes
        {
            match self.records.pop_front() {
                Some(oldest) => {
                    self.bytes = self.bytes.saturating_sub(oldest.size_hint());
                    self.report.dropped = self.report.dropped.saturating_add(1);
                    evicted = true;
                }
                None => break,
            }
        }

        self.records.push_back(record);
        self.bytes = self.bytes.saturating_add(record_bytes);

        match (evicted, truncated) {
            (true, true) => PushOutcome::EvictedAndTruncated,
            (true, false) => PushOutcome::EvictedOldest,
            (false, true) => PushOutcome::Truncated,
            (false, false) => PushOutcome::Stored,
        }
    }

    /// Drains all records and returns them with the accumulated drop report.
    pub fn drain(&mut self) -> (Vec<Observation>, DropReport) {
        let records: Vec<_> = self.records.drain(..).collect();
        self.bytes = 0;
        let report = std::mem::take(&mut self.report);
        (records, report)
    }

    /// Returns the accumulated drop report without draining the buffer.
    pub fn report(&self) -> DropReport {
        self.report
    }

    /// Returns the number of buffered records.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Returns whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Returns the current in-memory byte usage of the buffered records.
    pub fn bytes(&self) -> usize {
        self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitty_observability_api::{AttributeValue, EventKind};

    fn record(topic: &str, sequence: u64) -> Observation {
        Observation::new(EventKind::new(topic), sequence)
    }

    fn record_with_padding(topic: &str, sequence: u64, padding: usize) -> Observation {
        Observation::new(EventKind::new(topic), sequence)
            .with_attribute("padding", AttributeValue::Text("x".repeat(padding)))
    }

    #[test]
    fn ring_buffer_drops_oldest() {
        let mut buffer = TraceBuffer::new(BufferLimits::new(
            2,
            DEFAULT_MAX_RECORD_BYTES,
            DEFAULT_MAX_TOTAL_BYTES,
        ));

        assert_eq!(buffer.push(record("a", 1)), PushOutcome::Stored);
        assert_eq!(buffer.push(record("b", 2)), PushOutcome::Stored);
        assert_eq!(buffer.push(record("c", 3)), PushOutcome::EvictedOldest);

        let (records, report) = buffer.drain();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].kind.as_str(), "b");
        assert_eq!(records[1].kind.as_str(), "c");
        assert_eq!(report.dropped, 1);
    }

    #[test]
    fn oversized_record_is_truncated_explicitly() {
        let mut buffer = TraceBuffer::new(BufferLimits::new(8, 64, DEFAULT_MAX_TOTAL_BYTES));

        let outcome = buffer.push(record_with_padding("big", 1, 256));

        assert_eq!(outcome, PushOutcome::Truncated);
        let (records, report) = buffer.drain();
        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0].status(),
            bitty_observability_api::RecordStatus::Truncated
        );
        assert_eq!(report.truncated, 1);
    }

    #[test]
    fn inert_buffer_stores_nothing() {
        let mut buffer = TraceBuffer::inert();

        let outcome = buffer.push(record("a", 1));

        assert_eq!(outcome, PushOutcome::Inert);
        assert!(buffer.is_empty());
        assert!(buffer.drain().1.is_empty());
    }

    #[test]
    fn total_budget_evicts_until_the_record_fits() {
        let mut buffer = TraceBuffer::new(BufferLimits::new(8, 512, 128));

        let recoverable = record_with_padding("a", 1, 16);
        let recoverable_bytes = recoverable.size_hint();
        assert_eq!(buffer.push(recoverable), PushOutcome::Stored);
        assert_eq!(buffer.bytes(), recoverable_bytes);

        let larger = record_with_padding("b", 2, recoverable_bytes + 20);
        assert_eq!(buffer.push(larger), PushOutcome::EvictedOldest);
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer.report().dropped, 1);
    }

    #[test]
    fn record_larger_than_total_budget_is_rejected() {
        let mut buffer = TraceBuffer::new(BufferLimits::new(8, 512, 128));

        assert_eq!(
            buffer.push(record_with_padding("huge", 1, 200)),
            PushOutcome::Rejected
        );
        assert!(buffer.is_empty());
        assert_eq!(buffer.report().rejected, 1);
    }
}
