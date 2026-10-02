#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Bitty observability - the observation seam, redaction, and bounded buffering.
//!
//! This is the main facade crate that re-exports the API contract layer and the
//! shared implementation:
//! - `bitty-observability-api`: pure types, traits, and the default-deny gate
//! - `bitty-observability-core`: bounded buffers, filtering, and redaction
//!
//! # Architecture
//!
//! The observability system is split into layers:
//!
//! - **API layer**: zero-dependency contract types and traits
//! - **Core layer**: bounded buffer and emission-time redaction helpers
//! - **Runtime integration**: lives outside this repository and is not yet
//!   scheduled; nothing here is consumed by Bitty Core
//!
//! # Features
//!
//! - `default`: the core layers only; no optional feature is enabled

pub use bitty_observability_api as api;
pub use bitty_observability_core as core;

pub use api::{
    AttributeValue, Attribution, AuthorizationError, AuthorizationGate, ContractRange,
    ContractVersion, DropReport, EventKind, ObservabilityCapability, Observation, ObservationSink,
    ObserverOrigin, RecordStatus, Subscription, SubscriptionError, TraceConfig, TraceRecord,
    subscribe,
};
pub use core::{
    BufferLimits, DEFAULT_DENIED_FIELDS, EventFilter, PushOutcome, Sensitivity, TraceBuffer,
    classify_field, is_captured_by_default, redact_observation, redact_payload,
};
