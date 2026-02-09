//! Challenge-response authentication.
//!
//! This module implements the challenge-response protocol for establishing
//! authenticated sessions. It uses ephemeral challenges that must be signed
//! by the identity's private key.

use chrono::{Duration, Utc};
use dashmap::DashMap;
use rand::RngCore;

use crate::auth::identity::verify_signature;
use crate::auth::types::{AuthError, Challenge};

/// Default challenge TTL: 5 minutes.
pub const DEFAULT_CHALLENGE_TTL_SECONDS: i64 = 300;

/// Challenge store for in-memory challenge management.
pub struct ChallengeStore {
    /// Map of challenge key -> Challenge
    challenges: DashMap<String, Challenge>,
    /// TTL in seconds
    ttl_seconds: i64,
}

impl ChallengeStore {
    /// Create a new challenge store.
    pub fn new() -> Self {
        Self::with_ttl(DEFAULT_CHALLENGE_TTL_SECONDS)
    }

    /// Create a challenge store with custom TTL.
    pub fn with_ttl(ttl_seconds: i64) -> Self {
        Self {
            challenges: DashMap::new(),
            ttl_seconds,
        }
    }

    /// Create a new challenge for an identity.
    pub fn create_challenge(&self, identity_key: &str) -> Challenge {
        let mut challenge_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut challenge_bytes);
        let challenge = bs58::encode(&challenge_bytes).into_string();

        let created_at = Utc::now();
        let expires_at = created_at + Duration::seconds(self.ttl_seconds);

        let challenge_obj = Challenge {
            identity_key: identity_key.to_string(),
            challenge: challenge.clone(),
            created_at,
            expires_at,
        };

        let key = format!("{}:{}", identity_key, challenge);
        self.challenges.insert(key, challenge_obj.clone());

        challenge_obj
    }

    /// Verify a challenge exists and hasn't expired.
    pub fn verify_challenge(
        &self,
        identity_key: &str,
        challenge: &str,
    ) -> Result<Challenge, AuthError> {
        let key = format!("{}:{}", identity_key, challenge);

        match self.challenges.get(&key) {
            Some(challenge) => {
                if challenge.is_expired() {
                    // Remove expired challenge
                    drop(challenge);
                    self.challenges.remove(&key);
                    Err(AuthError::ChallengeExpired)
                } else {
                    Ok(challenge.clone())
                }
            }
            None => Err(AuthError::ChallengeExpired),
        }
    }

    /// Consume a challenge (removes it from store).
    pub fn consume_challenge(
        &self,
        identity_key: &str,
        challenge: &str,
    ) -> Result<Challenge, AuthError> {
        let key = format!("{}:{}", identity_key, challenge);

        match self.challenges.remove(&key) {
            Some((_, challenge)) => {
                if challenge.is_expired() {
                    Err(AuthError::ChallengeExpired)
                } else {
                    Ok(challenge)
                }
            }
            None => Err(AuthError::ChallengeExpired),
        }
    }

    /// Clean up expired challenges.
    pub fn cleanup_expired(&self) -> usize {
        let now = Utc::now();
        let mut removed = 0;

        self.challenges.retain(|_, challenge| {
            if challenge.expires_at < now {
                removed += 1;
                false
            } else {
                true
            }
        });

        removed
    }

    /// Get the number of active challenges.
    pub fn len(&self) -> usize {
        self.challenges.len()
    }

    /// Check if the store is empty.
    pub fn is_empty(&self) -> bool {
        self.challenges.is_empty()
    }
}

impl Default for ChallengeStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Verify a challenge-response authentication.
///
/// # Arguments
/// * `challenge_store` - The challenge store
/// * `identity_key` - The identity's public key
/// * `challenge` - The challenge string (base58)
/// * `response` - The signed response (signature, base58)
///
/// # Returns
/// `Ok(())` if verification succeeds, `Err(AuthError)` otherwise.
pub fn verify_challenge_response(
    challenge_store: &ChallengeStore,
    identity_key: &str,
    challenge: &str,
    response: &str,
) -> Result<(), AuthError> {
    // Consume the challenge (fails if expired or not found)
    let _challenge = challenge_store.consume_challenge(identity_key, challenge)?;

    // Decode the signature
    let signature = bs58::decode(response)
        .into_vec()
        .map_err(|_| AuthError::InvalidSignature)?;

    // Verify the signature
    let message = format!("challenge:{}", challenge);
    let valid = verify_signature(identity_key, message.as_bytes(), &signature)?;

    if valid {
        Ok(())
    } else {
        Err(AuthError::InvalidSignature)
    }
}

/// Create a response to a challenge (client-side).
///
/// # Arguments
/// * `secret_key` - The identity's secret key
/// * `challenge` - The challenge string
///
/// # Returns
/// The base58-encoded signature.
pub fn create_challenge_response(secret_key: &[u8], challenge: &str) -> Result<String, AuthError> {
    use crate::auth::identity::sign_message;

    let message = format!("challenge:{}", challenge);
    let signature = sign_message(secret_key, message.as_bytes())?;
    Ok(bs58::encode(&signature).into_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::identity::mine_identity_sync;
    use crate::auth::types::IdentityUserData;

    #[test]
    fn test_create_and_verify_challenge() {
        let store = ChallengeStore::new();
        let identity_key = "test_identity";

        // Create challenge
        let challenge = store.create_challenge(identity_key);
        assert_eq!(challenge.identity_key, identity_key);
        assert!(!challenge.challenge.is_empty());

        // Verify it exists
        let verified = store.verify_challenge(identity_key, &challenge.challenge);
        assert!(verified.is_ok());

        // Consume it
        let consumed = store.consume_challenge(identity_key, &challenge.challenge);
        assert!(consumed.is_ok());

        // Should fail to consume again
        let consumed_again = store.consume_challenge(identity_key, &challenge.challenge);
        assert!(matches!(consumed_again, Err(AuthError::ChallengeExpired)));
    }

    #[test]
    fn test_challenge_expiration() {
        let store = ChallengeStore::with_ttl(0); // 0 second TTL
        let identity_key = "test_identity";

        // Create challenge
        let challenge = store.create_challenge(identity_key);

        // Should be expired immediately (or very soon)
        std::thread::sleep(std::time::Duration::from_millis(10));

        let result = store.verify_challenge(identity_key, &challenge.challenge);
        assert!(matches!(result, Err(AuthError::ChallengeExpired)));
    }

    #[test]
    fn test_full_challenge_response() {
        let store = ChallengeStore::new();

        // Mine an identity
        let mined = mine_identity_sync(IdentityUserData::default(), 2);

        // Server: Create challenge
        let challenge = store.create_challenge(&mined.identity.public_key);

        // Client: Create response
        let response = create_challenge_response(&mined.secret_key, &challenge.challenge).unwrap();

        // Server: Verify response
        let result = verify_challenge_response(
            &store,
            &mined.identity.public_key,
            &challenge.challenge,
            &response,
        );
        assert!(result.is_ok());

        // Should fail to verify again (challenge consumed)
        let result2 = verify_challenge_response(
            &store,
            &mined.identity.public_key,
            &challenge.challenge,
            &response,
        );
        assert!(matches!(result2, Err(AuthError::ChallengeExpired)));
    }

    #[test]
    fn test_invalid_signature() {
        let store = ChallengeStore::new();

        // Mine two identities
        let mined1 = mine_identity_sync(IdentityUserData::default(), 2);
        let mined2 = mine_identity_sync(IdentityUserData::default(), 2);

        // Create challenge for identity 1
        let challenge = store.create_challenge(&mined1.identity.public_key);

        // Sign with identity 2's key
        let response = create_challenge_response(&mined2.secret_key, &challenge.challenge).unwrap();

        // Should fail verification
        let result = verify_challenge_response(
            &store,
            &mined1.identity.public_key,
            &challenge.challenge,
            &response,
        );
        assert!(matches!(result, Err(AuthError::InvalidSignature)));
    }

    #[test]
    fn test_cleanup_expired() {
        let store = ChallengeStore::with_ttl(0); // 0 second TTL

        // Create several challenges
        for i in 0..5 {
            store.create_challenge(&format!("identity_{}", i));
        }

        assert_eq!(store.len(), 5);

        // Wait a bit
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Cleanup
        let removed = store.cleanup_expired();
        assert_eq!(removed, 5);
        assert!(store.is_empty());
    }
}
