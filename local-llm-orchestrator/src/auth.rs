//! API-key authentication and role/namespace authorization.
//!
//! Keys are compared by SHA-256 digest in constant time, so configs can hold
//! `key_sha256` instead of the secret. With no keys configured the server is
//! in single-user local mode: every caller is an anonymous admin (and the
//! server refuses to bind beyond loopback, see `Config::check_bind`).

use sha2::{Digest, Sha256};

use crate::config::{Role, ServerConfig};
use crate::error::{Error, Result};

#[derive(Debug, Clone, PartialEq)]
pub struct Principal {
    pub name: String,
    pub role: Role,
    pub namespaces: Vec<String>,
}

impl Principal {
    pub fn local_admin() -> Self {
        Self {
            name: "local".into(),
            role: Role::Admin,
            namespaces: vec![],
        }
    }

    pub fn is_admin(&self) -> bool {
        self.role == Role::Admin
    }

    pub fn can_access(&self, namespace: &str) -> bool {
        self.is_admin()
            || self.namespaces.iter().any(|p| match p.strip_suffix('*') {
                Some(prefix) => namespace.starts_with(prefix),
                None => p == namespace,
            })
    }

    pub fn require_namespace(&self, namespace: &str) -> Result<()> {
        if self.can_access(namespace) {
            Ok(())
        } else {
            audit(self, "denied", &format!("namespace={namespace}"));
            Err(Error::Forbidden(format!(
                "key `{}` may not access namespace `{namespace}`",
                self.name
            )))
        }
    }

    pub fn require_admin(&self, action: &str) -> Result<()> {
        if self.is_admin() {
            Ok(())
        } else {
            audit(self, "denied", action);
            Err(Error::Forbidden(format!(
                "`{action}` requires an admin key"
            )))
        }
    }
}

struct KeyEntry {
    digest: [u8; 32],
    principal: Principal,
}

pub struct Authenticator {
    keys: Vec<KeyEntry>,
}

pub fn sha256(data: &[u8]) -> [u8; 32] {
    Sha256::digest(data).into()
}

pub fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn from_hex(s: &str) -> Option<[u8; 32]> {
    let mut out = [0u8; 32];
    if s.len() != 64 {
        return None;
    }
    for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
        out[i] = u8::from_str_radix(std::str::from_utf8(chunk).ok()?, 16).ok()?;
    }
    Some(out)
}

fn ct_eq(a: &[u8; 32], b: &[u8; 32]) -> bool {
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

impl Authenticator {
    pub fn new(cfg: &ServerConfig) -> Self {
        let mut keys = Vec::new();
        if let Some(k) = &cfg.api_key {
            keys.push(KeyEntry {
                digest: sha256(k.as_bytes()),
                principal: Principal {
                    name: "api_key".into(),
                    role: Role::Admin,
                    namespaces: vec![],
                },
            });
        }
        for k in &cfg.keys {
            let digest = match (&k.key_sha256, &k.key) {
                (Some(h), _) => from_hex(&h.to_ascii_lowercase()),
                (None, Some(plain)) => Some(sha256(plain.as_bytes())),
                (None, None) => None,
            };
            if let Some(digest) = digest {
                keys.push(KeyEntry {
                    digest,
                    principal: Principal {
                        name: k.name.clone(),
                        role: k.role.clone(),
                        namespaces: k.namespaces.clone(),
                    },
                });
            }
        }
        Self { keys }
    }

    pub fn enabled(&self) -> bool {
        !self.keys.is_empty()
    }

    /// Resolve a bearer token. Every key is compared so timing does not
    /// reveal which (if any) matched.
    pub fn authenticate(&self, bearer: Option<&str>) -> Result<Principal> {
        if !self.enabled() {
            return Ok(Principal::local_admin());
        }
        let Some(token) = bearer.filter(|t| !t.is_empty()) else {
            return Err(Error::Unauthorized);
        };
        let d = sha256(token.as_bytes());
        let mut found = None;
        for k in &self.keys {
            if ct_eq(&d, &k.digest) && found.is_none() {
                found = Some(k.principal.clone());
            }
        }
        found.ok_or(Error::Unauthorized)
    }
}

/// Generate a new random API key: `llmo_` + 256 bits of hex.
pub fn generate_key() -> String {
    let a = uuid::Uuid::new_v4();
    let b = uuid::Uuid::new_v4();
    let mut bytes = Vec::with_capacity(32);
    bytes.extend_from_slice(a.as_bytes());
    bytes.extend_from_slice(b.as_bytes());
    // Hash to whiten the UUID version/variant bits.
    format!("llmo_{}", to_hex(&sha256(&bytes)))
}

/// Structured audit event (target `llmo::audit`), for privileged or
/// destructive actions and denials.
pub fn audit(who: &Principal, action: &str, detail: &str) {
    tracing::info!(target: "llmo::audit", principal = %who.name, action, detail, "audit");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ApiKeyConfig, ServerConfig};

    fn cfg(keys: Vec<ApiKeyConfig>) -> ServerConfig {
        ServerConfig {
            keys,
            ..Default::default()
        }
    }

    #[test]
    fn local_mode_is_admin() {
        let a = Authenticator::new(&ServerConfig::default());
        assert!(a.authenticate(None).unwrap().is_admin());
    }

    #[test]
    fn hashed_and_plain_keys() {
        let secret = generate_key();
        let a = Authenticator::new(&cfg(vec![
            ApiKeyConfig {
                name: "ops".into(),
                key_sha256: Some(to_hex(&sha256(secret.as_bytes())).to_uppercase()),
                key: None,
                role: Role::Admin,
                namespaces: vec![],
            },
            ApiKeyConfig {
                name: "alice".into(),
                key_sha256: None,
                key: Some("alice-0123456789abcdef".into()),
                role: Role::User,
                namespaces: vec!["alice".into(), "team-*".into()],
            },
        ]));
        assert_eq!(a.authenticate(Some(&secret)).unwrap().name, "ops");
        let alice = a.authenticate(Some("alice-0123456789abcdef")).unwrap();
        assert!(alice.can_access("alice") && alice.can_access("team-red"));
        assert!(!alice.can_access("bob") && !alice.can_access("alicex"));
        assert!(alice.require_admin("x").is_err());
        assert!(matches!(
            a.authenticate(Some("nope")),
            Err(Error::Unauthorized)
        ));
        assert!(matches!(a.authenticate(None), Err(Error::Unauthorized)));
    }

    #[test]
    fn generated_keys_are_unique_and_long() {
        let (a, b) = (generate_key(), generate_key());
        assert_ne!(a, b);
        assert_eq!(a.len(), 5 + 64);
    }
}
