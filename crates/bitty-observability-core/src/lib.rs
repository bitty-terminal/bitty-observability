#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Bitty observability core - shared implementation.
//!
//! This crate implements the bounded buffer and redaction behavior the accepted
//! Core observability contract (`W-71`) requires of an observer:
//! - fixed record, in-flight, and total in-memory bounds with explicit drops;
//! - redaction at emission with sensitive-by-default attribute typing;
//! - event filtering and a bounded trace buffer.
//!
//! It holds no authority: it never widens the Core authorization gate, never
//! bypasses redaction, and adds no network or persistence behavior.

/// Bounded trace buffer management.
pub mod buffer;
/// Event filtering and pattern matching.
pub mod filter;
/// Payload redaction utilities.
pub mod redaction;

pub use buffer::{BufferLimits, PushOutcome, TraceBuffer};
pub use filter::EventFilter;
pub use redaction::{
    DEFAULT_DENIED_FIELDS, Sensitivity, classify_field, is_captured_by_default, redact_observation,
    redact_payload,
};
