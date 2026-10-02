/// Trace configuration and the trace record shape.
///
/// Observability is opt-in under the accepted `W-71` contract: the default
/// configuration is disabled, and the bounds mirror the observation-seam
/// limits so a trace implementation honors the same record and queue bounds.
use crate::event::{
    DEFAULT_MAX_RECORD_BYTES, DEFAULT_MAX_RECORDS, DEFAULT_MAX_TOTAL_BYTES, Observation,
};

/// A captured trace record.
///
/// The trace record is the same bounded, read-only [`Observation`] shape the
/// seam emits, so the trace layer cannot bypass the record bounds or the
/// redaction rules. The alias keeps the historical name available.
pub type TraceRecord = Observation;

/// Trace configuration.
///
/// `enabled` defaults to `false`: tracing is an explicit opt-in and writes
/// nothing by default. The bounds are the same fixed record, in-flight, and
/// total in-memory limits the observation seam uses.
#[derive(Debug, Clone)]
pub struct TraceConfig {
    /// Whether tracing is enabled. Defaults to `false` (opt-in).
    pub enabled: bool,

    /// Topic filter (e.g., "terminal.*").
    pub filter: Option<String>,

    /// Maximum number of records buffered in flight.
    pub max_events: usize,

    /// Maximum encoded size of a single record, in bytes.
    pub max_record_bytes: usize,

    /// Maximum total in-memory budget for buffered records, in bytes.
    pub max_total_bytes: usize,

    /// Maximum number of records in flight for one observer.
    pub max_records_in_flight: usize,
}

impl TraceConfig {
    /// Creates the disabled, bounded default configuration.
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            filter: None,
            max_events: DEFAULT_MAX_RECORDS,
            max_record_bytes: DEFAULT_MAX_RECORD_BYTES,
            max_total_bytes: DEFAULT_MAX_TOTAL_BYTES,
            max_records_in_flight: DEFAULT_MAX_RECORDS,
        }
    }
}

impl Default for TraceConfig {
    fn default() -> Self {
        Self::disabled()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_trace_config_is_disabled_and_bounded() {
        let config = TraceConfig::default();

        assert!(!config.enabled);
        assert_eq!(config.filter, None);
        assert_eq!(config.max_events, DEFAULT_MAX_RECORDS);
        assert_eq!(config.max_record_bytes, DEFAULT_MAX_RECORD_BYTES);
        assert_eq!(config.max_total_bytes, DEFAULT_MAX_TOTAL_BYTES);
    }

    #[test]
    fn trace_record_is_the_observation_shape() {
        let record: TraceRecord =
            TraceRecord::new(crate::event::EventKind::new("terminal.opened"), 1);

        assert_eq!(record.kind.as_str(), "terminal.opened");
    }
}
