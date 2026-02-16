//! Identity mining and verification.
//!
//! This module provides proof-of-work identity mining similar to koru-protocol.
//! The difficulty is tuned to be accessible (seconds, not minutes) while still
//! preventing spam.

use chrono::Utc;
use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};

use crate::auth::types::{Identity, IdentityUserData};

/// Default difficulty for identity mining (4 leading hex zeros).
///
/// This requires on average 65,536 hashes, which takes ~0.1-1s on modern hardware.
/// Tuned to be fast enough for user experience but slow enough to prevent spam.
pub const DEFAULT_DIFFICULTY: u8 = 4;

/// Minimum allowed difficulty (2 leading hex zeros).
pub const MIN_DIFFICULTY: u8 = 2;

/// Maximum allowed difficulty (8 leading hex zeros).
pub const MAX_DIFFICULTY: u8 = 8;

/// Result of identity mining.
#[derive(Debug, Clone)]
pub struct MinedIdentity {
    /// The mined identity
    pub identity: Identity,
    /// The secret key (keep secure!)
    pub secret_key: Vec<u8>,
    /// Number of hashes computed
    pub hashes_computed: u64,
    /// Time taken to mine
    pub duration_ms: u64,
}

/// Mine a new identity with proof-of-work.
///
/// # Arguments
/// * `user_data` - User profile data
/// * `difficulty` - Number of leading hex zeros required (default: 4)
///
/// # Returns
/// A `MinedIdentity` containing the identity and secret key.
///
/// # Example
/// ```rust,ignore
/// use koru_delta::auth::identity::{mine_identity, DEFAULT_DIFFICULTY};
/// use koru_delta::auth::types::IdentityUserData;
///
/// # async fn example() {
/// let user_data = IdentityUserData {
///     display_name: Some("Alice".to_string()),
///     bio: Some("Developer".to_string()),
///     avatar_hash: None,
///     metadata: Default::default(),
/// };
///
/// let mined = mine_identity(user_data, DEFAULT_DIFFICULTY).await;
/// println!("Mined identity: {}", mined.identity.public_key);
/// println!("Hashes computed: {}", mined.hashes_computed);
/// # }
/// ```
pub async fn mine_identity(user_data: IdentityUserData, difficulty: u8) -> MinedIdentity {
    let difficulty = difficulty.clamp(MIN_DIFFICULTY, MAX_DIFFICULTY);
    #[cfg(not(target_arch = "wasm32"))]
    let start_time = std::time::Instant::now();

    // Generate Ed25519 keypair
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();
    let public_key = bs58::encode(verifying_key.as_bytes()).into_string();
    let secret_key = signing_key.to_bytes().to_vec();

    let created_at = Utc::now();
    let mut nonce: u64 = 0;
    let mut hashes_computed: u64 = 0;

    // Mine: find nonce such that hash has required leading zeros
    let proof_hash = loop {
        let hash = compute_pow_hash(&public_key, &user_data, nonce, created_at);
        hashes_computed += 1;

        if count_leading_hex_zeros(&hash) >= difficulty {
            break hash;
        }

        nonce += 1;

        // Yield periodically to avoid blocking async runtime (native only)
        #[cfg(not(target_arch = "wasm32"))]
        if hashes_computed.is_multiple_of(10000) {
            tokio::task::yield_now().await;
        }
    };

    #[cfg(not(target_arch = "wasm32"))]
    let duration_ms = start_time.elapsed().as_millis() as u64;
    #[cfg(target_arch = "wasm32")]
    let duration_ms = 0; // Timing not available on WASM

    let identity = Identity {
        public_key,
        user_data,
        nonce,
        difficulty,
        proof_hash: hex::encode(&proof_hash),
        created_at,
    };

    MinedIdentity {
        identity,
        secret_key,
        hashes_computed,
        duration_ms,
    }
}

/// Synchronous version of identity mining for non-async contexts.
#[cfg(not(target_arch = "wasm32"))]
pub fn mine_identity_sync(user_data: IdentityUserData, difficulty: u8) -> MinedIdentity {
    let difficulty = difficulty.clamp(MIN_DIFFICULTY, MAX_DIFFICULTY);
    let start_time = std::time::Instant::now();

    // Generate Ed25519 keypair
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();
    let public_key = bs58::encode(verifying_key.as_bytes()).into_string();
    let secret_key = signing_key.to_bytes().to_vec();

    let created_at = Utc::now();
    let mut nonce: u64 = 0;
    let mut hashes_computed: u64 = 0;

    // Mine: find nonce such that hash has required leading zeros
    let proof_hash = loop {
        let hash = compute_pow_hash(&public_key, &user_data, nonce, created_at);
        hashes_computed += 1;

        if count_leading_hex_zeros(&hash) >= difficulty {
            break hash;
        }

        nonce += 1;
    };

    let duration_ms = start_time.elapsed().as_millis() as u64;

    let identity = Identity {
        public_key,
        user_data,
        nonce,
        difficulty,
        proof_hash: hex::encode(&proof_hash),
        created_at,
    };

    MinedIdentity {
        identity,
        secret_key,
        hashes_computed,
        duration_ms,
    }
}

/// Compute the proof-of-work hash.
fn compute_pow_hash(
    public_key: &str,
    user_data: &IdentityUserData,
    nonce: u64,
    created_at: chrono::DateTime<Utc>,
) -> Vec<u8> {
    let data = serde_json::json!({
        "public_key": public_key,
        "user_data": user_data,
        "nonce": nonce,
        "created_at": created_at,
    });

    let mut hasher = Sha256::new();
    hasher.update(serde_json::to_vec(&data).unwrap());
    hasher.finalize().to_vec()
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

/// Verify an identity's proof-of-work.
pub fn verify_identity_pow(identity: &Identity) -> bool {
    // Recompute hash with provided nonce
    let computed_hash = compute_pow_hash(
        &identity.public_key,
        &identity.user_data,
        identity.nonce,
        identity.created_at,
    );

    // Verify hash matches claimed proof_hash
    let claimed_hash = match hex::decode(&identity.proof_hash) {
        Ok(h) => h,
        Err(_) => return false,
    };

    if computed_hash != claimed_hash {
        return false;
    }

    // Verify difficulty
    count_leading_hex_zeros(&computed_hash) >= identity.difficulty
}

/// Estimate mining time based on current hardware.
/// Returns approximate hashes per second.
#[cfg(not(target_arch = "wasm32"))]
pub fn estimate_hash_rate() -> u64 {
    let start = std::time::Instant::now();
    let test_iterations = 100_000;

    let mut hasher = Sha256::new();
    for i in 0..test_iterations {
        hasher.update((i as u64).to_le_bytes());
        let _ = hasher.finalize_reset();
    }

    let elapsed = start.elapsed().as_secs_f64();
    (test_iterations as f64 / elapsed) as u64
}

/// Estimate time to mine at given difficulty.
/// Returns approximate milliseconds.
#[cfg(not(target_arch = "wasm32"))]
pub fn estimate_mining_time_ms(difficulty: u8) -> u64 {
    let hash_rate = estimate_hash_rate();
    let expected_hashes = 16u64.pow(difficulty as u32); // 16^n for hex
    (expected_hashes as f64 / hash_rate as f64 * 1000.0) as u64
}

/// Sign a message with an identity's secret key.
pub fn sign_message(
    secret_key: &[u8],
    message: &[u8],
) -> Result<Vec<u8>, crate::auth::types::AuthError> {
    let key_bytes: [u8; 32] = secret_key
        .try_into()
        .map_err(|_| crate::auth::types::AuthError::InvalidKeyFormat)?;

    let signing_key = SigningKey::from_bytes(&key_bytes);
    let signature = signing_key.sign(message);

    Ok(signature.to_bytes().to_vec())
}

/// Sign a message as a base58 string.
pub fn sign_message_base58(
    secret_key: &[u8],
    message: &[u8],
) -> Result<String, crate::auth::types::AuthError> {
    let signature = sign_message(secret_key, message)?;
    Ok(bs58::encode(&signature).into_string())
}

/// Verify a signature with a public key.
pub fn verify_signature(
    public_key: &str,
    message: &[u8],
    signature: &[u8],
) -> Result<bool, crate::auth::types::AuthError> {
    use ed25519_dalek::{Signature, VerifyingKey};
    use std::convert::TryFrom;

    // Decode public key
    let public_key_bytes = bs58::decode(public_key)
        .into_vec()
        .map_err(|_| crate::auth::types::AuthError::InvalidKeyFormat)?;

    let verifying_key = VerifyingKey::try_from(&public_key_bytes[..])
        .map_err(|_| crate::auth::types::AuthError::InvalidKeyFormat)?;

    // Decode signature
    let signature = Signature::from_slice(signature)
        .map_err(|_| crate::auth::types::AuthError::InvalidSignature)?;

    // Verify
    Ok(verifying_key.verify_strict(message, &signature).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mine_identity_sync() {
        let user_data = IdentityUserData {
            display_name: Some("Test".to_string()),
            ..Default::default()
        };

        let mined = mine_identity_sync(user_data, 2); // Low difficulty for fast test

        assert!(mined.hashes_computed > 0);
        assert!(verify_identity_pow(&mined.identity));
        assert!(mined.duration_ms < 5000); // Should complete quickly
    }

    #[test]
    fn test_verify_identity_pow_invalid() {
        let user_data = IdentityUserData {
            display_name: Some("Test".to_string()),
            ..Default::default()
        };

        let mut mined = mine_identity_sync(user_data, 2);

        // Tamper with the nonce
        mined.identity.nonce += 1;

        assert!(!verify_identity_pow(&mined.identity));
    }

    #[test]
    fn test_sign_and_verify() {
        let user_data = IdentityUserData::default();
        let mined = mine_identity_sync(user_data, 2);

        let message = b"hello world";
        let signature = sign_message(&mined.secret_key, message).unwrap();

        assert!(verify_signature(&mined.identity.public_key, message, &signature).unwrap());

        // Wrong message should fail
        let wrong_message = b"goodbye world";
        assert!(!verify_signature(&mined.identity.public_key, wrong_message, &signature).unwrap());
    }

    #[tokio::test]
    async fn test_mine_identity_async() {
        let user_data = IdentityUserData {
            display_name: Some("Async Test".to_string()),
            ..Default::default()
        };

        let mined = mine_identity(user_data, 2).await;

        assert!(mined.hashes_computed > 0);
        assert!(verify_identity_pow(&mined.identity));
    }
}
