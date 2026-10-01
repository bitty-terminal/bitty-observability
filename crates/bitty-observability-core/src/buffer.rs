/// Trace buffer management.
use bitty_observability_api::TraceRecord;
use std::collections::VecDeque;

/// Ring buffer for trace records.
pub struct TraceBuffer {
    records: VecDeque<TraceRecord>,
    max_events: usize,
    dropped: u64,
}

impl TraceBuffer {
    /// Creates a new trace buffer with the specified capacity.
    pub fn new(max_events: usize) -> Self {
        Self {
            records: VecDeque::with_capacity(max_events.min(1024)),
            max_events,
            dropped: 0,
        }
    }

    /// Pushes a record, dropping the oldest if at capacity.
    pub fn push(&mut self, record: TraceRecord) {
        if self.records.len() >= self.max_events {
            self.records.pop_front();
            self.dropped = self.dropped.saturating_add(1);
        }
        self.records.push_back(record);
    }

    /// Drains all records and returns them with the dropped count.
    pub fn drain(&mut self) -> (Vec<TraceRecord>, u64) {
        let records: Vec<_> = self.records.drain(..).collect();
        let dropped = self.dropped;
        self.dropped = 0;
        (records, dropped)
    }

    /// Returns the number of buffered records.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Returns whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn ring_buffer_drops_oldest() {
        let mut buffer = TraceBuffer::new(2);

        buffer.push(TraceRecord::new("a".into(), 1, 100, BTreeMap::new()));
        buffer.push(TraceRecord::new("b".into(), 2, 200, BTreeMap::new()));
        buffer.push(TraceRecord::new("c".into(), 3, 300, BTreeMap::new()));

        let (records, dropped) = buffer.drain();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].topic, "b");
        assert_eq!(records[1].topic, "c");
        assert_eq!(dropped, 1);
    }
}
