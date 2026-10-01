#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Bitty observability API layer - pure types and traits.
//!
//! This crate provides the contract layer for observability:
//! - Trace and debug types
//! - Event observation interfaces
//! - Capability definitions
//!
//! Zero implementation dependencies. This is the only crate plugins
//! and cross-repo consumers depend on.

/// Capability definitions for observability features.
pub mod capability;
/// Event observation types and interfaces.
pub mod event;
/// Trace configuration and record types.
pub mod trace;

pub use capability::ObservabilityCapability;
pub use event::{EventKind, ObservableEvent};
pub use trace::{TraceConfig, TraceRecord};
