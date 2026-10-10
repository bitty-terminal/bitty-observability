#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Bitty observability API layer - pure types and traits.
//!
//! This crate provides the contract layer for the read-only observation seam of
//! the accepted Core observability contract (bitty-docs `W-71`):
//! - the bounded, read-only observation record and its attribute typing;
//! - the Core-owned host side of the seam (what Core pushes, bounds);
//! - the read-only sink trait and the bounded, detachable subscription;
//! - the default-deny authorization gate with separate inspect and subscribe
//!   grants;
//! - the versioned contract range used to attach an observer fail-closed.
//!
//! Zero dependencies. This is the only crate plugins and cross-repo consumers
//! depend on for API contracts. The exact trait, type, method, field, and token
//! spellings remain parked by the accepted contract; only the shape is fixed
//! here. Nothing in this crate claims Core integration.

/// Capability definitions and the default-deny authorization gate.
pub mod capability;
/// Bounded observation records, attribute typing, and drop accounting.
pub mod event;
/// The Core-owned observation host contract: what Core pushes, bounds.
pub mod host;
/// The read-only observation sink, subscription, and version negotiation.
pub mod sink;
/// Trace configuration and the trace record shape.
pub mod trace;

pub use capability::{
    AuthorizationError, AuthorizationGate, ObservabilityCapability, ObserverOrigin,
};
pub use event::{
    AttributeValue, Attribution, DEFAULT_MAX_RECORD_BYTES, DEFAULT_MAX_RECORDS,
    DEFAULT_MAX_TOTAL_BYTES, DropReport, EventKind, Observation, RecordStatus,
};
pub use host::ObservationHost;
pub use sink::{
    ContractRange, ContractVersion, ObservationSink, Subscription, SubscriptionError, subscribe,
};
pub use trace::{TraceConfig, TraceRecord};
