/// Observability capability definitions.
///
/// Observability capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ObservabilityCapability {
    /// Inspect plugin state, commands, events, and grants.
    DebugInspect,

    /// Enable bounded event tracing.
    DebugTrace,

    /// Control plugin lifecycle (reload, suspend, resume).
    DebugControl,
}

impl ObservabilityCapability {
    /// Returns the capability identifier string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DebugInspect => "debug.inspect",
            Self::DebugTrace => "debug.trace",
            Self::DebugControl => "debug.control",
        }
    }

    /// Parses a capability identifier string.
    ///
    /// Note: Does not implement `FromStr` trait to avoid confusion with standard library trait.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "debug.inspect" => Some(Self::DebugInspect),
            "debug.trace" => Some(Self::DebugTrace),
            "debug.control" => Some(Self::DebugControl),
            _ => None,
        }
    }
}
