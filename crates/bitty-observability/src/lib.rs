#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Bitty observability - metrics, tracing, and telemetry.
//!
//! This is the main facade crate that re-exports components from:
//! - `bitty-observability-api`: Core trait definitions
//! - `bitty-observability-core`: Shared implementation
//!
//! # Architecture
//!
//! The observability system is split into layers:
//!
//! - **API layer**: Pure trait definitions with zero dependencies
//! - **Core layer**: Shared utilities (filters, buffers, redaction)
//! - **Runtime integration**: Lives in bitty-runtime, uses the API
//!
//! # Features
//!
//! - `default`: Core functionality only

pub use bitty_observability_api as api;
pub use bitty_observability_core as core;

pub use api::{EventKind, ObservabilityCapability, ObservableEvent, TraceConfig, TraceRecord};
pub use core::{EventFilter, TraceBuffer};
