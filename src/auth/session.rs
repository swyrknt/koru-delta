//! Session management with HKDF key derivation.
//!
//! Sessions are created after successful challenge-response authentication.
//! Each session has derived encryption keys via HKDF-SHA256.

use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use sha2::{Digest, Sha256};

use crate::auth::types::{AuthError, CapabilityRef, Session};

/// Default session TTL: 24 hours.
pub const DEFAULT_SESSION_TTL_SECONDS: i64 = 86400;

/// Maximum session TTL: 30 days.
pub const MAX_SESSION_TTL_SECONDS: i64 = 2592000;

/// Size of derived keys in bytes.
pub const KEY_SIZE: usize = 32;

/// Derived session keys.
#[derive(Debug, Clone)]
pub struct SessionKeys {
    /// Encryption key for this session
    pub encryption_key: [u8; KEY_SIZE],
    /// Authentication key for this session
    pub auth_key: [u8; KEY_SIZE],
}

/// Session manager for in-memory session storage.
pub struct SessionManager {
    /// Map of session_id -> Session with keys
    sessions: DashMap<String, (Session, SessionKeys)>,
    /// Default TTL in seconds
    ttl_seconds: i64,
}

impl SessionManager {
    /// Create a new session manager.
    pub fn new() -> Self {
        Self::with_ttl(DEFAULT_SESSION_TTL_SECONDS)
    }

    /// Create a session manager with custom TTL.
    pub fn with_ttl(ttl_seconds: i64) -> Self {
        let ttl_seconds = ttl_seconds.min(MAX_SESSION_TTL_SECONDS);
        Self {
            sessions: DashMap::new(),
            ttl_seconds,
        }
    }

    /// Create a new session after successful authentication.
    ///
    /// # Arguments
    /// * `identity_key` - The authenticated identity's public key
    /// * `challenge` - The challenge that was used for authentication
    /// * `capabilities` - Capabilities granted to this session
    ///
    /// # Returns
    /// The session ID and session keys.
    pub fn create_session(
        &self,
        identity_key: &str,
        challenge: &str,
        capabilities: Vec<CapabilityRef>,
    ) -> (Session, SessionKeys) {
        let created_at = Utc::now();
        let expires_at = created_at + Duration::seconds(self.ttl_seconds);

        // Derive session keys
        let keys = derive_session_keys(identity_key, challenge);

        // Session ID is derived from auth key
        let session_id = bs58::encode(&keys.auth_key).into_string();

        let session = Session {
            session_id: session_id.clone(),
            identity_key: identity_key.to_string(),
            created_at,
            expires_at,
            capabilities,
        };

        self.sessions
            .insert(session_id.clone(), (session.clone(), keys.clone()));

        (session, keys)
    }

    /// Get a session by ID.
    pub fn get_session(&self, session_id: &str) -> Result<(Session, SessionKeys), AuthError> {
        match self.sessions.get(session_id) {
            Some(entry) => {
                let (session, keys) = entry.value().clone();
                if session.is_expired() {
                    drop(entry);
                    self.sessions.remove(session_id);
                    Err(AuthError::SessionExpired)
                } else {
                    Ok((session, keys))
                }
            }
            None => Err(AuthError::SessionExpired),
        }
    }

    /// Validate a session (check if it exists and is not expired).
    pub fn validate_session(&self, session_id: &str) -> Result<Session, AuthError> {
        self.get_session(session_id).map(|(session, _)| session)
    }

    /// Get session keys.
    pub fn get_session_keys(&self, session_id: &str) -> Result<SessionKeys, AuthError> {
        self.get_session(session_id).map(|(_, keys)| keys)
    }

    /// Revoke a session.
    pub fn revoke_session(&self, session_id: &str) -> Result<(), AuthError> {
        match self.sessions.remove(session_id) {
            Some(_) => Ok(()),
            None => Err(AuthError::SessionExpired),
        }
    }

    /// Revoke all sessions for an identity.
    pub fn revoke_all_identity_sessions(&self, identity_key: &str) -> usize {
        let mut removed = 0;

        self.sessions.retain(|_, (session, _)| {
            if session.identity_key == identity_key {
                removed += 1;
                false
            } else {
                true
            }
        });

        removed
    }

    /// Clean up expired sessions.
    pub fn cleanup_expired(&self) -> usize {
        let now = Utc::now();
        let mut removed = 0;

        self.sessions.retain(|_, (session, _)| {
            if session.expires_at < now {
                removed += 1;
                false
            } else {
                true
            }
        });

        removed
    }

    /// Get all active sessions for an identity.
    pub fn get_identity_sessions(&self, identity_key: &str) -> Vec<Session> {
        self.sessions
            .iter()
            .filter(|entry| entry.value().0.identity_key == identity_key)
            .map(|entry| entry.value().0.clone())
            .collect()
    }

    /// Get the number of active sessions.
    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    /// Check if the manager is empty.
    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Derive session keys using HKDF-SHA256.
///
/// # Arguments
/// * `identity_key` - The identity's public key
/// * `challenge` - The challenge that was authenticated
///
/// # Returns
/// Derived encryption and authentication keys.
pub fn derive_session_keys(identity_key: &str, challenge: &str) -> SessionKeys {
    use hkdf::Hkdf;

    // IKM (Input Keying Material): combination of identity and challenge
    let ikm = format!("{}:{}", identity_key, challenge);

    // Salt: hash of the IKM
    let salt = Sha256::digest(ikm.as_bytes());

    // HKDF extract
    let hkdf = Hkdf::<Sha256>::new(Some(&salt), ikm.as_bytes());

    // Expand to 64 bytes (2 keys)
    let mut okm = [0u8; 64];
    hkdf.expand(b"koru-session-v1", &mut okm)
        .expect("HKDF expand should not fail with valid parameters");

    SessionKeys {
        encryption_key: okm[0..32].try_into().unwrap(),
        auth_key: okm[32..64].try_into().unwrap(),
    }
}

/// Create a session token (session_id + signature).
/// This can be used for stateless session validation.
pub fn create_session_token(
    session_keys: &SessionKeys,
    timestamp: DateTime<Utc>,
) -> Result<String, AuthError> {
    use hmac::{Hmac, Mac};

    type HmacSha256 = Hmac<Sha256>;

    let message = format!("session:{}", timestamp.timestamp());

    let mut mac = HmacSha256::new_from_slice(&session_keys.auth_key)
        .map_err(|_| AuthError::InvalidKeyFormat)?;
    mac.update(message.as_bytes());
    let signature = mac.finalize().into_bytes();

    let token = format!(
        "{}.{}.{}",
        bs58::encode(&session_keys.auth_key).into_string(),
        timestamp.timestamp(),
        bs58::encode(&signature).into_string()
    );

    Ok(token)
}

/// Validate a session token.
pub fn validate_session_token(
    token: &str,
    max_age_seconds: i64,
) -> Result<(String, DateTime<Utc>), AuthError> {
    use hmac::{Hmac, Mac};

    type HmacSha256 = Hmac<Sha256>;

    // Parse token: session_id.timestamp.signature
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(AuthError::InvalidSignature);
    }

    let session_id = parts[0];
    let timestamp_secs: i64 = parts[1].parse().map_err(|_| AuthError::InvalidSignature)?;
    let signature = bs58::decode(parts[2])
        .into_vec()
        .map_err(|_| AuthError::InvalidSignature)?;

    let timestamp =
        DateTime::from_timestamp(timestamp_secs, 0).ok_or(AuthError::InvalidSignature)?;

    // Check age
    let age = Utc::now().signed_duration_since(timestamp);
    if age.num_seconds() > max_age_seconds {
        return Err(AuthError::SessionExpired);
    }

    // Verify signature
    let message = format!("session:{}", timestamp_secs);
    let auth_key = bs58::decode(session_id)
        .into_vec()
        .map_err(|_| AuthError::InvalidKeyFormat)?;

    let mut mac = HmacSha256::new_from_slice(&auth_key).map_err(|_| AuthError::InvalidKeyFormat)?;
    mac.update(message.as_bytes());

    mac.verify_slice(&signature)
        .map_err(|_| AuthError::InvalidSignature)?;

    Ok((session_id.to_string(), timestamp))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_session_keys() {
        let identity_key = "test_identity";
        let challenge = "test_challenge";

        let keys1 = derive_session_keys(identity_key, challenge);
        let keys2 = derive_session_keys(identity_key, challenge);

        // Same inputs should produce same keys
        assert_eq!(keys1.encryption_key, keys2.encryption_key);
        assert_eq!(keys1.auth_key, keys2.auth_key);

        // Different challenge should produce different keys
        let keys3 = derive_session_keys(identity_key, "different_challenge");
        assert_ne!(keys1.encryption_key, keys3.encryption_key);
    }

    #[test]
    fn test_session_manager() {
        let manager = SessionManager::new();
        let identity_key = "test_identity";
        let challenge = "test_challenge";

        // Create session
        let (session, keys) = manager.create_session(identity_key, challenge, vec![]);

        assert_eq!(session.identity_key, identity_key);
        assert!(!session.session_id.is_empty());

        // Validate session
        let validated = manager.validate_session(&session.session_id);
        assert!(validated.is_ok());

        // Get keys
        let retrieved_keys = manager.get_session_keys(&session.session_id);
        assert!(retrieved_keys.is_ok());
        assert_eq!(retrieved_keys.unwrap().encryption_key, keys.encryption_key);

        // Revoke session
        manager.revoke_session(&session.session_id).unwrap();

        // Should fail after revocation
        let after_revoke = manager.validate_session(&session.session_id);
        assert!(matches!(after_revoke, Err(AuthError::SessionExpired)));
    }

    #[test]
    fn test_session_expiration() {
        let manager = SessionManager::with_ttl(0); // 0 second TTL
        let identity_key = "test_identity";
        let challenge = "test_challenge";

        // Create session
        let (session, _) = manager.create_session(identity_key, challenge, vec![]);

        // Wait a bit
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Should be expired
        let result = manager.validate_session(&session.session_id);
        assert!(matches!(result, Err(AuthError::SessionExpired)));
    }

    #[test]
    fn test_cleanup_expired() {
        let manager = SessionManager::with_ttl(0);

        // Create several sessions
        for i in 0..5 {
            manager.create_session(&format!("identity_{}", i), "challenge", vec![]);
        }

        assert_eq!(manager.len(), 5);

        // Wait a bit
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Cleanup
        let removed = manager.cleanup_expired();
        assert_eq!(removed, 5);
        assert!(manager.is_empty());
    }

    #[test]
    fn test_revoke_all_identity_sessions() {
        let manager = SessionManager::new();
        let identity_key = "test_identity";

        // Create multiple sessions for same identity
        // Note: each session needs a unique challenge to get a unique session_id
        for i in 0..3 {
            manager.create_session(identity_key, &format!("challenge{}", i), vec![]);
        }

        // Create session for different identity
        manager.create_session("other_identity", "challenge_other", vec![]);

        assert_eq!(manager.len(), 4);

        // Revoke all for identity
        let removed = manager.revoke_all_identity_sessions(identity_key);
        assert_eq!(removed, 3);
        assert_eq!(manager.len(), 1);
    }

    #[test]
    fn test_session_token() {
        let identity_key = "test_identity";
        let challenge = "test_challenge";

        let keys = derive_session_keys(identity_key, challenge);
        let timestamp = Utc::now();

        // Create token
        let token = create_session_token(&keys, timestamp).unwrap();
        assert!(!token.is_empty());

        // Validate token
        let (session_id, validated_ts) = validate_session_token(&token, 60).unwrap();
        assert_eq!(session_id, bs58::encode(&keys.auth_key).into_string());
        assert_eq!(validated_ts.timestamp(), timestamp.timestamp());
    }

    #[test]
    fn test_session_token_expiration() {
        let identity_key = "test_identity";
        let challenge = "test_challenge";

        let keys = derive_session_keys(identity_key, challenge);
        let old_timestamp = Utc::now() - Duration::seconds(100);

        // Create token with old timestamp
        let token = create_session_token(&keys, old_timestamp).unwrap();

        // Should fail with max_age of 60 seconds
        let result = validate_session_token(&token, 60);
        assert!(matches!(result, Err(AuthError::SessionExpired)));
    }

    #[test]
    fn test_invalid_session_token() {
        // Invalid format
        let result = validate_session_token("invalid", 60);
        assert!(matches!(result, Err(AuthError::InvalidSignature)));

        // Wrong number of parts
        let result = validate_session_token("part1.part2", 60);
        assert!(matches!(result, Err(AuthError::InvalidSignature)));
    }
}
