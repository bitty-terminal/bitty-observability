/// The read-only observation sink and its bounded, detachable subscription.
///
/// The accepted `W-71` contract names the sink `ObservationSink` and requires
/// the seam to expose no control, mutation, or handle-passing path. A
/// subscription is bounded, versioned, and detachable.
use crate::capability::{
    AuthorizationError, AuthorizationGate, ObservabilityCapability, ObserverOrigin,
};
use crate::event::{DropReport, Observation};

/// A version of the observation contract.
///
/// The concrete values are owned by the accepted contract; this crate
/// advertises and consumes ranges rather than fixed values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContractVersion {
    /// Major version; renaming or changing a record's meaning advances it.
    pub major: u16,

    /// Minor version; an ignorable additive record or attribute advances it.
    pub minor: u16,
}

impl ContractVersion {
    /// Creates a contract version.
    pub const fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }
}

/// An inclusive contract version range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContractRange {
    /// The lowest compatible version.
    pub min: ContractVersion,

    /// The highest compatible version.
    pub max: ContractVersion,
}

impl ContractRange {
    /// Creates an inclusive range.
    pub const fn new(min: ContractVersion, max: ContractVersion) -> Self {
        Self { min, max }
    }

    /// Returns the intersection of two ranges, or `None` when they do not
    /// overlap.
    ///
    /// Core attaches an observer only on a non-empty intersection and fails
    /// closed otherwise, so `None` means the observer must be refused.
    pub fn intersect(&self, other: &Self) -> Option<Self> {
        let min = self.min.max(other.min);
        let max = self.max.min(other.max);
        if min <= max {
            Some(Self { min, max })
        } else {
            None
        }
    }
}

/// A bounded, detachable subscription to the observation record stream.
///
/// The handle carries only the granted capability, the observer origin, the
/// negotiated contract version, and the in-flight record bound. It exposes no
/// Core handle and no mutation or control method.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subscription {
    id: u64,
    capability: ObservabilityCapability,
    origin: ObserverOrigin,
    version: ContractVersion,
    max_records_in_flight: usize,
    active: bool,
}

impl Subscription {
    /// Creates an active subscription.
    pub fn new(
        id: u64,
        capability: ObservabilityCapability,
        origin: ObserverOrigin,
        version: ContractVersion,
        max_records_in_flight: usize,
    ) -> Self {
        Self {
            id,
            capability,
            origin,
            version,
            max_records_in_flight,
            active: true,
        }
    }

    /// Returns the subscription identifier.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Returns the capability granted to this subscription.
    pub fn capability(&self) -> ObservabilityCapability {
        self.capability
    }

    /// Returns the observer origin.
    pub fn origin(&self) -> ObserverOrigin {
        self.origin
    }

    /// Returns the negotiated contract version.
    pub fn contract_version(&self) -> ContractVersion {
        self.version
    }

    /// Returns the maximum number of records in flight for this observer.
    pub fn max_records_in_flight(&self) -> usize {
        self.max_records_in_flight
    }

    /// Returns whether the subscription is still active.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Detaches the subscription; the observer receives no further records.
    pub fn detach(&mut self) {
        self.active = false;
    }
}

/// A read-only receiver of bounded observation records.
///
/// An observer implements this trait to receive records. The trait exposes no
/// method that can mutate Core state, and no PTY, GPU, window, input, or
/// capability handle is passed through it. The exact method shape is subject to
/// the accepted contract.
pub trait ObservationSink {
    /// Receives one bounded, redacted observation record.
    fn on_observation(&mut self, observation: &Observation);

    /// Receives explicit accounting for records dropped or truncated before
    /// delivery. The default implementation ignores the report.
    fn on_drop(&mut self, _report: &DropReport) {}

    /// Receives notice that the subscription was detached. The default
    /// implementation ignores it.
    fn on_detached(&mut self) {}
}

/// Authorizes and negotiates a subscription through the default-deny gate.
///
/// Returns the bounded subscription on success, or the authorization or
/// version failure. The version check fails closed: a range with no
/// intersection with `core_range` is refused.
///
/// # Errors
///
/// Returns [`SubscriptionError::Authorization`] when the gate refuses the
/// observer, or [`SubscriptionError::IncompatibleVersion`] when the version
/// ranges do not overlap.
pub fn subscribe(
    gate: &AuthorizationGate,
    id: u64,
    capability: ObservabilityCapability,
    origin: ObserverOrigin,
    observer_range: ContractRange,
    core_range: ContractRange,
    max_records_in_flight: usize,
) -> Result<Subscription, SubscriptionError> {
    gate.authorize(capability, origin)?;
    let version = observer_range
        .intersect(&core_range)
        .map(|range| range.max)
        .ok_or(SubscriptionError::IncompatibleVersion)?;
    Ok(Subscription::new(
        id,
        capability,
        origin,
        version,
        max_records_in_flight,
    ))
}

/// Why a subscription request was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SubscriptionError {
    /// The default-deny gate refused the observer.
    Authorization(AuthorizationError),

    /// The observer and Core contract ranges do not intersect.
    IncompatibleVersion,
}

impl From<AuthorizationError> for SubscriptionError {
    fn from(error: AuthorizationError) -> Self {
        Self::Authorization(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn core_range() -> ContractRange {
        ContractRange::new(ContractVersion::new(0, 1), ContractVersion::new(0, 1))
    }

    #[test]
    fn intersecting_ranges_attach() {
        let mut gate = AuthorizationGate::default_deny();
        gate.grant(ObservabilityCapability::DebugTrace);

        let subscription = subscribe(
            &gate,
            7,
            ObservabilityCapability::DebugTrace,
            ObserverOrigin::InProcess,
            ContractRange::new(ContractVersion::new(0, 0), ContractVersion::new(0, 3)),
            core_range(),
            16,
        )
        .expect("compatible observer should attach");

        assert_eq!(subscription.contract_version(), ContractVersion::new(0, 1));
        assert_eq!(subscription.max_records_in_flight(), 16);
        assert!(subscription.is_active());
    }

    #[test]
    fn incompatible_ranges_fail_closed() {
        let mut gate = AuthorizationGate::default_deny();
        gate.grant(ObservabilityCapability::DebugTrace);

        let result = subscribe(
            &gate,
            7,
            ObservabilityCapability::DebugTrace,
            ObserverOrigin::InProcess,
            ContractRange::new(ContractVersion::new(1, 0), ContractVersion::new(2, 0)),
            core_range(),
            16,
        );

        assert_eq!(result, Err(SubscriptionError::IncompatibleVersion));
    }

    #[test]
    fn ungranted_observer_is_refused_by_the_gate() {
        let gate = AuthorizationGate::default_deny();

        let result = subscribe(
            &gate,
            7,
            ObservabilityCapability::DebugTrace,
            ObserverOrigin::OutOfProcess,
            ContractRange::new(ContractVersion::new(0, 1), ContractVersion::new(0, 1)),
            core_range(),
            16,
        );

        assert_eq!(
            result,
            Err(SubscriptionError::Authorization(
                AuthorizationError::MissingCapability(ObservabilityCapability::DebugTrace)
            ))
        );
    }

    #[test]
    fn detach_deactivates_subscription() {
        let mut subscription = Subscription::new(
            1,
            ObservabilityCapability::DebugInspect,
            ObserverOrigin::InProcess,
            ContractVersion::new(0, 1),
            4,
        );
        subscription.detach();
        assert!(!subscription.is_active());
    }
}
