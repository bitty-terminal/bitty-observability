/// Payload redaction utilities.
///
/// The accepted `W-71` contract requires redaction at emission, sensitive by
/// default: a newly added attribute is sensitive until it is explicitly
/// classified otherwise, and raw PTY bytes, clipboard content, and raw
/// environment data are never captured by default.
use bitty_observability_api::{AttributeValue, Observation};
use std::collections::BTreeMap;

/// Field names that are never captured by default.
///
/// These carry raw PTY bytes, clipboard content, raw environment data, or raw
/// input, which the accepted contract excludes regardless of grant.
pub const DEFAULT_DENIED_FIELDS: &[&str] = &["pty.raw", "clipboard", "env.raw", "input.raw"];

/// Sensitivity classification for an observation attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Sensitivity {
    /// Sensitive until explicitly classified otherwise; redacted at emission.
    #[default]
    Sensitive,

    /// Explicitly classified as safe to emit.
    NonSensitive,
}

impl Sensitivity {
    /// Returns whether the field is sensitive and must be redacted.
    pub const fn is_sensitive(self) -> bool {
        matches!(self, Self::Sensitive)
    }
}

/// Returns whether a field is captured by default.
///
/// Fields in [`DEFAULT_DENIED_FIELDS`] are never captured, even when
/// allowlisted.
pub fn is_captured_by_default(field: &str) -> bool {
    !DEFAULT_DENIED_FIELDS.contains(&field)
}

/// Classifies a field name against the explicitly non-sensitive fields.
///
/// A field that is not explicitly classified is sensitive by default.
pub fn classify_field(field: &str, non_sensitive_fields: &[&str]) -> Sensitivity {
    if non_sensitive_fields.contains(&field) {
        Sensitivity::NonSensitive
    } else {
        Sensitivity::Sensitive
    }
}

/// Redacts an observation at emission.
///
/// Only explicitly non-sensitive fields keep their value; every other field is
/// replaced with [`AttributeValue::Redacted`], and fields that are never
/// captured by default are redacted regardless of classification. The raw
/// sensitive value never reaches an observer.
pub fn redact_observation(observation: &Observation, non_sensitive_fields: &[&str]) -> Observation {
    let mut redacted = Observation::with_timestamp(
        observation.kind.clone(),
        observation.sequence,
        observation.timestamp,
    );
    if let Some(attribution) = &observation.attribution {
        redacted = redacted.with_attribution(attribution.clone());
    }

    for (key, value) in observation.attributes() {
        let captured = is_captured_by_default(key)
            && classify_field(key, non_sensitive_fields) == Sensitivity::NonSensitive;
        if captured {
            redacted.insert_attribute(key.clone(), value.clone());
        } else {
            redacted.insert_attribute(key.clone(), AttributeValue::Redacted);
        }
    }

    redacted
}

/// Redacts sensitive fields from a payload.
///
/// Removes all fields except those in the allowlist.
pub fn redact_payload(
    payload: &BTreeMap<String, String>,
    allowed_fields: &[&str],
) -> BTreeMap<String, String> {
    let mut redacted = BTreeMap::new();

    for field in allowed_fields {
        if let Some(value) = payload.get(*field) {
            redacted.insert(field.to_string(), value.clone());
        }
    }

    if redacted.is_empty() && !payload.is_empty() {
        redacted.insert("redacted".to_string(), "true".to_string());
    }

    redacted
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitty_observability_api::EventKind;

    fn observation_with(pairs: &[(&str, &str)]) -> Observation {
        let mut observation = Observation::new(EventKind::new("terminal.input"), 1);
        for (key, value) in pairs {
            observation.insert_attribute(*key, AttributeValue::Text((*value).to_string()));
        }
        observation
    }

    #[test]
    fn keeps_allowed_fields() {
        let mut payload = BTreeMap::new();
        payload.insert("action".into(), "paste".into());
        payload.insert("text".into(), "secret".into());

        let redacted = redact_payload(&payload, &["action"]);

        assert_eq!(redacted.len(), 1);
        assert_eq!(redacted.get("action").unwrap(), "paste");
        assert!(!redacted.contains_key("text"));
    }

    #[test]
    fn adds_redacted_marker_when_all_removed() {
        let mut payload = BTreeMap::new();
        payload.insert("text".into(), "secret".into());

        let redacted = redact_payload(&payload, &[]);

        assert_eq!(redacted.len(), 1);
        assert_eq!(redacted.get("redacted").unwrap(), "true");
    }

    #[test]
    fn sensitive_fields_are_redacted_at_emission() {
        let observation = observation_with(&[("action", "paste"), ("token", "secret://abc")]);

        let redacted = redact_observation(&observation, &["action"]);

        assert_eq!(
            redacted.attributes().get("action"),
            Some(&AttributeValue::Text("paste".into()))
        );
        assert_eq!(
            redacted.attributes().get("token"),
            Some(&AttributeValue::Redacted)
        );
        assert!(!format!("{redacted:?}").contains("secret://abc"));
    }

    #[test]
    fn default_denied_fields_are_never_captured() {
        let observation = observation_with(&[
            ("pty.raw", "raw-bytes"),
            ("clipboard", "clipboard-secret"),
            ("env.raw", "API_KEY=leaked"),
        ]);

        let redacted = redact_observation(&observation, &["pty.raw", "clipboard", "env.raw"]);

        for key in ["pty.raw", "clipboard", "env.raw"] {
            assert_eq!(
                redacted.attributes().get(key),
                Some(&AttributeValue::Redacted)
            );
        }
        assert!(!format!("{redacted:?}").contains("leaked"));
        assert!(!format!("{redacted:?}").contains("clipboard-secret"));
    }

    #[test]
    fn unknown_fields_are_sensitive_by_default() {
        assert!(classify_field("new_field", &[]).is_sensitive());
        assert!(!classify_field("new_field", &["new_field"]).is_sensitive());
        assert!(!is_captured_by_default("pty.raw"));
        assert!(is_captured_by_default("terminal.opened"));
    }
}
