//! Best-effort scrubbing of secrets and personal identifiers before text is
//! persisted. This is a safety net, not a guarantee.

use std::sync::LazyLock;

use regex::Regex;

static RULES: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    [
        (
            r"-----BEGIN [A-Z ]*PRIVATE KEY-----[\s\S]*?-----END [A-Z ]*PRIVATE KEY-----",
            "[REDACTED_PRIVATE_KEY]",
        ),
        (
            r"(?i)\bbearer\s+[A-Za-z0-9._~+/=-]{16,}",
            "Bearer [REDACTED_TOKEN]",
        ),
        (r"\b(?:sk|pk|rk)-[A-Za-z0-9_-]{16,}", "[REDACTED_KEY]"),
        (r"\bgh[pousr]_[A-Za-z0-9]{30,}\b", "[REDACTED_KEY]"),
        (r"\bxox[abpr]-[A-Za-z0-9-]{10,}\b", "[REDACTED_KEY]"),
        (r"\bAKIA[0-9A-Z]{16}\b", "[REDACTED_KEY]"),
        (
            r"\beyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\b",
            "[REDACTED_JWT]",
        ),
        (
            r"(?i)\b(password|passwd|pwd|secret|api[_-]?key|token)(\s*[:=]\s*)\S+",
            "$1$2[REDACTED]",
        ),
        (
            r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}",
            "[REDACTED_EMAIL]",
        ),
    ]
    .into_iter()
    .map(|(re, rep)| (Regex::new(re).expect("valid redaction regex"), rep))
    .collect()
});

pub fn redact(text: &str) -> String {
    let mut out = text.to_string();
    for (re, rep) in RULES.iter() {
        if re.is_match(&out) {
            out = re.replace_all(&out, *rep).into_owned();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrubs_common_secrets() {
        let s = redact(
            "mail me at jane.doe@example.com, key sk-abcdefghijklmnop1234, \
             password: hunter2 and Authorization: Bearer abcdefghijklmnopqrstuvwxyz",
        );
        assert!(!s.contains("jane.doe"));
        assert!(!s.contains("sk-abcdefghijklmnop1234"));
        assert!(!s.contains("hunter2"));
        assert!(!s.contains("abcdefghijklmnopqrstuvwxyz"));
        assert!(s.contains("[REDACTED_EMAIL]"));
        assert!(s.contains("password: [REDACTED]"));
    }

    #[test]
    fn scrubs_private_keys() {
        let s = redact("-----BEGIN RSA PRIVATE KEY-----\nMIIabc\n-----END RSA PRIVATE KEY-----");
        assert_eq!(s, "[REDACTED_PRIVATE_KEY]");
    }

    #[test]
    fn leaves_normal_text_alone() {
        let t = "I like hiking in the Alps and writing Rust.";
        assert_eq!(redact(t), t);
    }
}
