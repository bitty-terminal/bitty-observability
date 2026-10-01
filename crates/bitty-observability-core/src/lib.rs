#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Bitty observability core - shared implementation.
//!
//! This crate provides shared utilities and common implementation:
//! - Event filtering and matching
//! - Trace buffer management
//! - Payload redaction helpers
//!
//! Used by both the runtime and extension layers.

/// Trace buffer management with size bounds.
pub mod buffer;
/// Event filtering and pattern matching.
pub mod filter;
/// Payload redaction utilities.
pub mod redaction;

pub use buffer::TraceBuffer;
pub use filter::EventFilter;
