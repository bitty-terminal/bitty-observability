/// Payload redaction utilities.
use std::collections::BTreeMap;

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
}
