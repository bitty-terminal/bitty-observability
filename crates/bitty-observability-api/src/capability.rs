/// Observability capability definitions and the default-deny authorization gate.
///
/// The read-only observation seam of the accepted `W-71` contract recognizes
/// exactly two capability scopes, matching the accepted DevTools vocabulary:
/// structured inspection and subscription to the bounded record stream. The
/// control scope is deliberately absent, because the seam is read-only and
/// control is not part of observability.
use std::collections::BTreeSet;

/// Observability capability definitions.
///
/// Each variant is an independently granted scope. The exact token spelling is
/// owned by the security corpus and remains subject to the accepted contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum ObservabilityCapability {
    /// Inspect structured state, commands, events, and grants
    /// (DevTools `debug.inspect`).
    DebugInspect,

    /// Subscribe to the bounded observation record stream
    /// (DevTools `debug.trace`).
    DebugTrace,
}

impl ObservabilityCapability {
    /// Returns the capability identifier string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DebugInspect => "debug.inspect",
            Self::DebugTrace => "debug.trace",
        }
    }

    /// Parses a capability identifier string.
    ///
    /// Returns `None` for any unknown token, including `debug.control`: the
    /// control scope is not an observability capability.
    ///
    /// Note: Does not implement `FromStr` trait to avoid confusion with standard library trait.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "debug.inspect" => Some(Self::DebugInspect),
            "debug.trace" => Some(Self::DebugTrace),
            _ => None,
        }
    }
}

/// Where an observer reaches Core from.
///
/// The accepted contract requires an explicit capability for every observer,
/// and additionally explicit consent when the observer reaches Core from
/// outside the process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObserverOrigin {
    /// A Core-owned subsystem observing its own mechanisms; no external
    /// consent is required.
    InProcess,

    /// An observer outside the process; explicit consent is required in
    /// addition to the capability.
    OutOfProcess,
}

/// Why the authorization gate refused an observer.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AuthorizationError {
    /// The required capability was not granted.
    MissingCapability(ObservabilityCapability),

    /// The observer is out-of-process and no explicit consent was recorded.
    ConsentRequired,
}

/// Default-deny authorization gate for the observation seam.
///
/// The gate starts closed: no capability is granted and no consent is
/// recorded. Compilation, connection, or configuration alone grants nothing.
/// There is no control scope; only inspect and trace can be granted, each
/// independently.
#[derive(Debug, Clone, Default)]
pub struct AuthorizationGate {
    granted: BTreeSet<ObservabilityCapability>,
    consented: bool,
}

impl AuthorizationGate {
    /// Creates a gate with no grants and no consent (the default-deny state).
    pub fn default_deny() -> Self {
        Self::default()
    }

    /// Returns whether `capability` has been explicitly granted.
    pub fn is_granted(&self, capability: ObservabilityCapability) -> bool {
        self.granted.contains(&capability)
    }

    /// Grants one capability scope; other scopes remain ungranted.
    pub fn grant(&mut self, capability: ObservabilityCapability) {
        self.granted.insert(capability);
    }

    /// Revokes one capability scope.
    pub fn revoke(&mut self, capability: ObservabilityCapability) {
        self.granted.remove(&capability);
    }

    /// Returns whether explicit consent for an out-of-process observer is
    /// recorded.
    pub fn consented(&self) -> bool {
        self.consented
    }

    /// Records explicit consent for out-of-process observers.
    pub fn grant_consent(&mut self) {
        self.consented = true;
    }

    /// Withdraws explicit consent for out-of-process observers.
    pub fn revoke_consent(&mut self) {
        self.consented = false;
    }

    /// Authorizes an observer for `capability` at the given `origin`.
    ///
    /// Fails closed: a missing capability or a missing consent for an
    /// out-of-process observer returns an error.
    ///
    /// # Errors
    ///
    /// Returns [`AuthorizationError::MissingCapability`] when the scope is not
    /// granted, and [`AuthorizationError::ConsentRequired`] when an
    /// out-of-process observer lacks explicit consent.
    pub fn authorize(
        &self,
        capability: ObservabilityCapability,
        origin: ObserverOrigin,
    ) -> Result<(), AuthorizationError> {
        if !self.is_granted(capability) {
            return Err(AuthorizationError::MissingCapability(capability));
        }
        if origin == ObserverOrigin::OutOfProcess && !self.consented {
            return Err(AuthorizationError::ConsentRequired);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_is_default_deny() {
        let gate = AuthorizationGate::default_deny();

        assert!(!gate.is_granted(ObservabilityCapability::DebugInspect));
        assert!(!gate.is_granted(ObservabilityCapability::DebugTrace));
        assert_eq!(
            gate.authorize(
                ObservabilityCapability::DebugInspect,
                ObserverOrigin::InProcess
            ),
            Err(AuthorizationError::MissingCapability(
                ObservabilityCapability::DebugInspect
            ))
        );
    }

    #[test]
    fn inspect_and_trace_grants_are_separate() {
        let mut gate = AuthorizationGate::default_deny();
        gate.grant(ObservabilityCapability::DebugInspect);

        assert!(
            gate.authorize(
                ObservabilityCapability::DebugInspect,
                ObserverOrigin::InProcess
            )
            .is_ok()
        );
        assert_eq!(
            gate.authorize(
                ObservabilityCapability::DebugTrace,
                ObserverOrigin::InProcess
            ),
            Err(AuthorizationError::MissingCapability(
                ObservabilityCapability::DebugTrace
            ))
        );
    }

    #[test]
    fn out_of_process_observer_needs_consent() {
        let mut gate = AuthorizationGate::default_deny();
        gate.grant(ObservabilityCapability::DebugTrace);

        assert_eq!(
            gate.authorize(
                ObservabilityCapability::DebugTrace,
                ObserverOrigin::OutOfProcess
            ),
            Err(AuthorizationError::ConsentRequired)
        );

        gate.grant_consent();
        assert!(
            gate.authorize(
                ObservabilityCapability::DebugTrace,
                ObserverOrigin::OutOfProcess
            )
            .is_ok()
        );
    }

    #[test]
    fn control_scope_is_not_an_observability_capability() {
        assert_eq!(ObservabilityCapability::parse("debug.control"), None);
        assert_eq!(
            ObservabilityCapability::parse("debug.inspect"),
            Some(ObservabilityCapability::DebugInspect)
        );
        assert_eq!(
            ObservabilityCapability::parse("debug.trace"),
            Some(ObservabilityCapability::DebugTrace)
        );
    }
}
