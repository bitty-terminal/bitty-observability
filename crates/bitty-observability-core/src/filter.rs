/// Event filtering utilities.
///
/// Event filter for matching event kinds against patterns.
#[derive(Debug, Clone)]
pub struct EventFilter {
    pattern: String,
    is_prefix: bool,
}

impl EventFilter {
    /// Creates a new event filter.
    ///
    /// Patterns ending with `*` are treated as prefix matches.
    /// Other patterns require exact match.
    pub fn new(pattern: impl Into<String>) -> Self {
        let pattern = pattern.into();
        let is_prefix = pattern.ends_with('*');

        Self { pattern, is_prefix }
    }

    /// Checks if an event kind matches this filter.
    pub fn matches(&self, kind: &str) -> bool {
        if self.is_prefix {
            let prefix = &self.pattern[..self.pattern.len() - 1];
            kind.starts_with(prefix)
        } else {
            kind == self.pattern
        }
    }

    /// Returns the pattern string.
    pub fn pattern(&self) -> &str {
        &self.pattern
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match() {
        let filter = EventFilter::new("terminal.opened");
        assert!(filter.matches("terminal.opened"));
        assert!(!filter.matches("terminal.closed"));
        assert!(!filter.matches("terminal.opened.foo"));
    }

    #[test]
    fn prefix_match() {
        let filter = EventFilter::new("terminal.*");
        assert!(filter.matches("terminal.opened"));
        assert!(filter.matches("terminal.closed"));
        assert!(filter.matches("terminal.foo.bar"));
        assert!(!filter.matches("plugin.activated"));
    }
}
