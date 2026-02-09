//! Core types for self-sovereign authentication.
//!
//! This module defines the fundamental data structures for identity, sessions,
//! capabilities, and authorization in the KoruDelta auth system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use serde_json::Value as JsonValue;

/// A mined identity stored as a distinction.
///
/// Identity is self-sovereign: users generate their own keys and mine their
/// identity distinction. The proof-of-work prevents spam while remaining
/// accessible to regular users.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Identity {
    /// Public key (base58 encoded Ed25519 key)
    pub public_key: String,

    /// User data (name, bio, avatar hash, etc.)
    pub user_data: IdentityUserData,

    /// Proof-of-work: nonce that makes hash start with difficulty zeros
    pub nonce: u64,

    /// Difficulty level (number of leading hex zeros required)
    pub difficulty: u8,

    /// Hash of (public_key + user_data + nonce) - must start with difficulty zeros
    pub proof_hash: String,

    /// Timestamp of mining
    pub created_at: DateTime<Utc>,
}

/// User data associated with an identity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct IdentityUserData {
    /// Display name (e.g., "Alice")
    pub display_name: Option<String>,

    /// Bio or description
    pub bio: Option<String>,

    /// Hash of avatar image (for content-addressed lookup)
    pub avatar_hash: Option<String>,

    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, JsonValue>,
}

impl Identity {
    /// Compute the hash for proof-of-work verification.
    pub fn compute_hash(&self) -> Vec<u8> {
        use sha2::{Digest, Sha256};

        let data = serde_json::json!({
            "public_key": &self.public_key,
            "user_data": &self.user_data,
            "nonce": self.nonce,
            "created_at": self.created_at,
        });

        let mut hasher = Sha256::new();
        hasher.update(serde_json::to_vec(&data).unwrap());
        hasher.finalize().to_vec()
    }

    /// Verify the proof-of-work is valid.
    pub fn verify_pow(&self) -> bool {
        let hash = self.compute_hash();
        let proof_hash = match hex::decode(&self.proof_hash) {
            Ok(h) => h,
            Err(_) => return false,
        };

        // Verify hash matches
        if hash != proof_hash {
            return false;
        }

        // Verify difficulty
        count_leading_hex_zeros(&hash) >= self.difficulty
    }
}

/// Count leading hex zeros in a byte array.
fn count_leading_hex_zeros(bytes: &[u8]) -> u8 {
    let mut count = 0u8;
    for byte in bytes {
        let high = byte >> 4;
        let low = byte & 0x0f;

        if high == 0 {
            count += 1;
        } else {
            return count;
        }

        if low == 0 {
            count += 1;
        } else {
            return count;
        }
    }
    count
}

/// Ephemeral challenge for authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Challenge {
    /// Public key of identity being challenged
    pub identity_key: String,

    /// Random bytes (base58 encoded)
    pub challenge: String,

    /// Timestamp when challenge was created
    pub created_at: DateTime<Utc>,

    /// Expiry time
    pub expires_at: DateTime<Utc>,
}

impl Challenge {
    /// Check if the challenge has expired.
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

/// Authenticated session derived from challenge-response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Session ID (derived from master key)
    pub session_id: String,

    /// Identity public key
    pub identity_key: String,

    /// When session was created
    pub created_at: DateTime<Utc>,

    /// Session expiry
    pub expires_at: DateTime<Utc>,

    /// Capabilities granted to this session (stored as keys)
    pub capabilities: Vec<CapabilityRef>,
}

impl Session {
    /// Check if the session has expired.
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

/// Reference to a capability within a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityRef {
    /// Storage key of the capability distinction
    pub capability_key: String,

    /// Resource pattern this capability applies to
    pub resource_pattern: ResourcePattern,

    /// Permission level
    pub permission: Permission,
}

/// A capability grants permissions to an identity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Capability {
    /// Unique identifier for this capability
    pub id: String,

    /// Who granted this capability
    pub granter: String,

    /// Who receives this capability
    pub grantee: String,

    /// What namespace:key patterns this applies to
    pub resource_pattern: ResourcePattern,

    /// What permissions are granted
    pub permission: Permission,

    /// When capability was created
    pub created_at: DateTime<Utc>,

    /// Optional expiry
    pub expires_at: Option<DateTime<Utc>>,

    /// Signature by granter (base58 encoded)
    pub signature: String,
}

impl Capability {
    /// Check if the capability has expired.
    pub fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(expiry) => Utc::now() > expiry,
            None => false,
        }
    }

    /// Verify the capability signature.
    pub fn verify_signature(&self) -> Result<bool, AuthError> {
        use ed25519_dalek::{Signature, VerifyingKey};
        use std::convert::TryFrom;

        // Decode granter's public key
        let public_key_bytes = bs58::decode(&self.granter)
            .into_vec()
            .map_err(|_| AuthError::InvalidKeyFormat)?;

        let verifying_key = VerifyingKey::try_from(&public_key_bytes[..])
            .map_err(|_| AuthError::InvalidKeyFormat)?;

        // Decode signature
        let signature_bytes = bs58::decode(&self.signature)
            .into_vec()
            .map_err(|_| AuthError::InvalidSignature)?;

        let signature =
            Signature::from_slice(&signature_bytes).map_err(|_| AuthError::InvalidSignature)?;

        // Create message to verify
        let message = self.signature_message();

        // Verify
        Ok(verifying_key.verify_strict(&message, &signature).is_ok())
    }

    /// Create the message that should be signed.
    fn signature_message(&self) -> Vec<u8> {
        format!(
            "capability_grant:{}/{}->{}/{}/{}/{}",
            self.id,
            self.granter,
            self.grantee,
            self.resource_pattern,
            self.permission.as_str(),
            self.created_at.timestamp()
        )
        .into_bytes()
    }
}

/// Resource pattern for capability matching.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ResourcePattern {
    /// Exact key: "users:alice:profile"
    Exact(String),
    /// Pattern with single wildcard: "users:alice:*"
    Wildcard { prefix: String },
    /// Entire namespace: "users"
    Namespace(String),
}

impl ResourcePattern {
    /// Check if a resource matches this pattern.
    pub fn matches(&self, namespace: &str, key: &str) -> bool {
        match self {
            ResourcePattern::Exact(pattern) => {
                let full = format!("{}:{}", namespace, key);
                pattern == &full
            }
            ResourcePattern::Wildcard { prefix } => {
                let full = format!("{}:{}", namespace, key);
                full.starts_with(prefix)
            }
            ResourcePattern::Namespace(ns) => ns == namespace,
        }
    }
}

impl std::fmt::Display for ResourcePattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourcePattern::Exact(s) => write!(f, "{}", s),
            ResourcePattern::Wildcard { prefix } => write!(f, "{}*", prefix),
            ResourcePattern::Namespace(ns) => write!(f, "{}:**", ns),
        }
    }
}

/// Permission levels.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    /// Read-only access
    Read,
    /// Read and write access
    Write,
    /// Full access including capability granting
    Admin,
}

impl Permission {
    /// Check if this permission includes another.
    pub fn includes(&self, other: Permission) -> bool {
        matches!(
            (self, other),
            (Permission::Admin, _)
                | (Permission::Write, Permission::Read)
                | (Permission::Write, Permission::Write)
                | (Permission::Read, Permission::Read)
        )
    }

    /// String representation for signatures.
    pub fn as_str(&self) -> &'static str {
        match self {
            Permission::Read => "read",
            Permission::Write => "write",
            Permission::Admin => "admin",
        }
    }
}

/// Revocation of a capability via tombstone distinction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Revocation {
    /// The capability ID being revoked
    pub capability_id: String,

    /// Who is revoking (must be granter)
    pub revoked_by: String,

    /// When revoked
    pub revoked_at: DateTime<Utc>,

    /// Reason (optional)
    pub reason: Option<String>,

    /// Signature by revoker
    pub signature: String,
}

/// Auth errors.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Identity not found: {0}")]
    IdentityNotFound(String),

    #[error("Identity already exists: {0}")]
    IdentityExists(String),

    #[error("Invalid proof-of-work")]
    InvalidProofOfWork,

    #[error("Invalid key format")]
    InvalidKeyFormat,

    #[error("Challenge expired or not found")]
    ChallengeExpired,

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Session expired or not found")]
    SessionExpired,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Capability not found: {0}")]
    CapabilityNotFound(String),

    #[error("Capability revoked")]
    CapabilityRevoked,

    #[error("Insufficient permissions")]
    InsufficientPermissions,

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Storage error: {0}")]
    Storage(String),
}

impl From<crate::DeltaError> for AuthError {
    fn from(err: crate::DeltaError) -> Self {
        AuthError::Storage(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_leading_hex_zeros() {
        // 0x00 = 0,0 (both digits zero) = 2 zeros
        // 0x01 = 0,1 (high digit zero) = 1 zero
        // Total: 2 + 1 = 3
        assert_eq!(count_leading_hex_zeros(&[0x00, 0x01]), 3);
        // 0x00, 0x00 = 2 + 2 = 4 zeros, 0x12 stops at first non-zero (1)
        assert_eq!(count_leading_hex_zeros(&[0x00, 0x00, 0x12]), 4);
        // 0x01 = 0,1 - stops at first non-zero (1)
        assert_eq!(count_leading_hex_zeros(&[0x01, 0x00]), 1);
        // 0x00 = 0,0 = 2 zeros, 0x0f = 0,15 (f is non-zero) = 1 zero, total 3
        assert_eq!(count_leading_hex_zeros(&[0x00, 0x0f]), 3);
    }

    #[test]
    fn test_permission_includes() {
        assert!(Permission::Read.includes(Permission::Read));
        assert!(!Permission::Read.includes(Permission::Write));
        assert!(!Permission::Read.includes(Permission::Admin));

        assert!(Permission::Write.includes(Permission::Read));
        assert!(Permission::Write.includes(Permission::Write));
        assert!(!Permission::Write.includes(Permission::Admin));

        assert!(Permission::Admin.includes(Permission::Read));
        assert!(Permission::Admin.includes(Permission::Write));
        assert!(Permission::Admin.includes(Permission::Admin));
    }

    #[test]
    fn test_resource_pattern_matching() {
        let exact = ResourcePattern::Exact("users:alice:profile".to_string());
        assert!(exact.matches("users", "alice:profile"));
        assert!(!exact.matches("users", "alice:settings"));
        assert!(!exact.matches("other", "alice:profile"));

        let wildcard = ResourcePattern::Wildcard {
            prefix: "users:alice:".to_string(),
        };
        assert!(wildcard.matches("users", "alice:profile"));
        assert!(wildcard.matches("users", "alice:settings"));
        assert!(!wildcard.matches("users", "bob:profile"));

        let ns = ResourcePattern::Namespace("users".to_string());
        assert!(ns.matches("users", "anything"));
        assert!(ns.matches("users", "alice:profile"));
        assert!(!ns.matches("other", "anything"));
    }

    #[test]
    fn test_identity_verify_pow() {
        use chrono::TimeZone;

        // Create a valid identity with known hash
        let identity = Identity {
            public_key: "test".to_string(),
            user_data: IdentityUserData::default(),
            nonce: 0,
            difficulty: 0,
            proof_hash: hex::encode(Identity::compute_hash(&Identity {
                public_key: "test".to_string(),
                user_data: IdentityUserData::default(),
                nonce: 0,
                difficulty: 0,
                proof_hash: String::new(),
                created_at: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            })),
            created_at: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
        };

        assert!(identity.verify_pow());
    }
}
