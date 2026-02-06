//! Self-sovereign authentication via distinctions.
//!
//! This module implements self-sovereign identity and capability-based
//! authorization using KoruDelta's distinction system. Identity is mined
//! as a distinction, sessions are established via challenge-response,
//! and capabilities are stored as distinctions with cryptographic proofs.
//!
//! # Core Concepts
//!
//! ## Identity
//! An identity is a mined distinction containing:
//! - Ed25519 public key
//! - User data (name, bio, etc.)
//! - Proof-of-work (prevents spam)
//!
//! Users generate their own keys and mine their identity. The difficulty
//! is tuned to be accessible (~1 second on modern hardware) while
//! preventing mass registration attacks.
//!
//! ## Challenge-Response Authentication
//! To authenticate:
//! 1. Client requests a challenge from the server
//! 2. Server generates random 32-byte challenge (5 minute TTL)
//! 3. Client signs challenge with their private key
//! 4. Server verifies signature against stored identity
//! 5. Server creates session with derived encryption keys
//!
//! ## Sessions
//! Sessions are ephemeral by default (24 hour TTL). Each session has:
//! - Session ID (derived from HKDF)
//! - Encryption key (for client-server communication)
//! - Authentication key (for session validation)
//! - Capability references (what the session can access)
//!
//! ## Capabilities
//! Capabilities are grants of permission from one identity to another:
//! - Granter: identity giving permission
//! - Grantee: identity receiving permission
//! - Resource pattern: what the permission applies to
//! - Permission level: Read, Write, or Admin
//!
//! Capabilities are signed by the granter and stored as distinctions.
//! They can be revoked via tombstone distinctions.
//!
//! ## Resource Patterns
//! - Exact: `users:alice:profile` - matches exactly
//! - Wildcard: `users:alice:*` - matches any key under prefix
//! - Namespace: `users` - matches entire namespace
//!
//! # Example Usage
//!
//! ```rust,ignore
//! use koru_delta::auth::{AuthManager, AuthConfig, IdentityUserData};
//! use koru_delta::storage::CausalStorage;
//! use std::sync::Arc;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create storage
//! let storage = Arc::new(CausalStorage::new(
//!     std::sync::Arc::new(koru_lambda_core::DistinctionEngine::new())
//! ));
//!
//! // Create auth manager
//! let auth = AuthManager::new(storage);
//!
//! // Create identity
//! let user_data = IdentityUserData {
//!     display_name: Some("Alice".to_string()),
//!     bio: Some("Developer".to_string()),
//!     ..Default::default()
//! };
//! let (identity, secret_key) = auth.create_identity(user_data)?;
//!
//! // Authenticate
//! let challenge = auth.create_challenge(&identity.public_key)?;
//! let response = koru_delta::auth::create_challenge_response(&secret_key, &challenge)?;
//! let session = auth.verify_and_create_session(&identity.public_key, &challenge, &response)?;
//!
//! // Validate session
//! let session = auth.validate_session(&session.session_id)?;
//! # Ok(())
//! # }
//! ```
//!
//! # Storage Layout
//!
//! Auth data is stored in the `_auth` namespace:
//! - `_auth:identity:{public_key}` - Identity distinctions
//! - `_auth:capability:{id}` - Capability grants
//! - `_auth:revocation:{capability_id}` - Capability revocations
//!
//! This allows auth state to:
//! - Be versioned (history preserved)
//! - Be reconciled (synced between nodes)
//! - Be queried via causal graph

// Core types
pub mod types;

// Sub-modules
mod capability;
mod identity;
mod manager;
mod session;
mod storage;
mod verification;

// HTTP module (requires http feature)
#[cfg(all(not(target_arch = "wasm32"), feature = "http"))]
pub mod http;

// Public exports from sub-modules
pub use capability::{
    authorize, check_permission, create_capability, create_revocation, CapabilityManager,
};
pub use identity::{
    estimate_hash_rate, estimate_mining_time_ms, mine_identity, mine_identity_sync,
    sign_message, sign_message_base58, verify_identity_pow, verify_signature,
    DEFAULT_DIFFICULTY, MAX_DIFFICULTY, MIN_DIFFICULTY,
};
pub use manager::{AuthConfig, AuthManager, AuthStats};
pub use session::{
    create_session_token, derive_session_keys, validate_session_token, SessionManager,
    DEFAULT_SESSION_TTL_SECONDS, MAX_SESSION_TTL_SECONDS,
};
pub use storage::{AuthStorageAdapter, AUTH_NAMESPACE};
pub use types::{
    AuthError, Capability, CapabilityRef, Challenge, Identity, IdentityUserData, Permission,
    ResourcePattern, Revocation, Session,
};
pub use verification::{
    create_challenge_response, verify_challenge_response, ChallengeStore,
    DEFAULT_CHALLENGE_TTL_SECONDS,
};

// HTTP exports (requires http feature)
#[cfg(all(not(target_arch = "wasm32"), feature = "http"))]
pub use http::{
    auth_layer, auth_routes, protected_routes, AuthContext, AuthorizeRequest, AuthorizeResponse,
    CapabilityInfo, CapabilityResponse, ChallengeRequest, ChallengeResponse, extract_auth_context,
    GrantCapabilityRequest, RegisterRequest, RegisterResponse, require_auth_context, SessionInfo,
    SessionResponse, ValidateSessionRequest, ValidateSessionResponse, VerifyRequest,
};

use crate::storage::CausalStorage;
use std::sync::Arc;

/// Initialize the auth system with default configuration.
///
/// # Example
/// ```rust,ignore
/// use koru_delta::auth;
/// use koru_delta::storage::CausalStorage;
/// use std::sync::Arc;
///
/// # fn example() {
/// # let storage = Arc::new(CausalStorage::new(
/// #     std::sync::Arc::new(koru_lambda_core::DistinctionEngine::new())
/// # ));
/// let auth = auth::init(storage);
/// # }
/// ```
pub fn init(storage: Arc<CausalStorage>) -> AuthManager {
    AuthManager::new(storage)
}

/// Initialize the auth system with custom configuration.
///
/// # Example
/// ```rust,ignore
/// use koru_delta::auth::{self, AuthConfig};
/// use koru_delta::storage::CausalStorage;
/// use std::sync::Arc;
///
/// # fn example() {
/// # let storage = Arc::new(CausalStorage::new(
/// #     std::sync::Arc::new(koru_lambda_core::DistinctionEngine::new())
/// # ));
/// let config = AuthConfig {
///     identity_difficulty: 3,  // Easier mining
///     challenge_ttl_seconds: 600,  // 10 minutes
///     session_ttl_seconds: 3600,   // 1 hour
///     persist_sessions: true,
/// };
/// let auth = auth::init_with_config(storage, config);
/// # }
/// ```
pub fn init_with_config(storage: Arc<CausalStorage>, config: AuthConfig) -> AuthManager {
    AuthManager::with_config(storage, config)
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    fn create_test_auth() -> AuthManager {
        let storage = Arc::new(CausalStorage::new(
            std::sync::Arc::new(koru_lambda_core::DistinctionEngine::new()),
        ));
        AuthManager::new(storage)
    }

    #[test]
    fn test_full_auth_flow() {
        let mut auth = create_test_auth();

        // 1. Create identity
        let user_data = IdentityUserData {
            display_name: Some("Test User".to_string()),
            bio: Some("Testing auth flow".to_string()),
            ..Default::default()
        };

        let (identity, secret_key) = auth.create_identity(user_data).unwrap();
        assert!(!identity.public_key.is_empty());
        assert!(identity.verify_pow());

        // 2. Get challenge
        let challenge_str = auth.create_challenge(&identity.public_key).unwrap();
        assert!(!challenge_str.is_empty());

        // 3. Sign challenge (client-side)
        let response = create_challenge_response(&secret_key, &challenge_str).unwrap();

        // 4. Verify and create session
        let session = auth
            .verify_and_create_session(&identity.public_key, &challenge_str, &response)
            .unwrap();
        assert!(!session.session_id.is_empty());
        assert_eq!(session.identity_key, identity.public_key);

        // 5. Validate session
        let validated = auth.validate_session(&session.session_id).unwrap();
        assert_eq!(validated.session_id, session.session_id);

        // 6. Create another identity for capability testing
        let (grantee, _grantee_key) = auth.create_identity(IdentityUserData::default()).unwrap();

        // 7. Grant capability
        let cap = auth
            .grant_capability(
                &identity,
                &secret_key,
                &grantee.public_key,
                ResourcePattern::Exact("test:data".to_string()),
                Permission::Read,
                None,
            )
            .unwrap();

        // 8. Verify capability authorization
        assert!(auth.check_permission(&grantee.public_key, "test", "data", Permission::Read));
        assert!(!auth.check_permission(&grantee.public_key, "test", "data", Permission::Write));

        // 9. Revoke capability
        auth.revoke_capability(&cap, &secret_key, Some("Test revocation".to_string()))
            .unwrap();

        // 10. Verify revocation
        assert!(!auth.check_permission(&grantee.public_key, "test", "data", Permission::Read));

        // 11. Cleanup
        auth.revoke_session(&session.session_id).unwrap();
        let result = auth.validate_session(&session.session_id);
        assert!(matches!(result, Err(AuthError::SessionExpired)));
    }

    #[test]
    fn test_init_functions() {
        let storage = Arc::new(CausalStorage::new(
            std::sync::Arc::new(koru_lambda_core::DistinctionEngine::new()),
        ));

        let auth1 = init(storage.clone());
        let auth2 = init_with_config(
            storage,
            AuthConfig {
                identity_difficulty: 3,
                ..Default::default()
            },
        );

        assert_eq!(auth1.config().identity_difficulty, 4); // default
        assert_eq!(auth2.config().identity_difficulty, 3); // custom
    }

    #[test]
    fn test_auth_error_display() {
        let err = AuthError::IdentityNotFound("test".to_string());
        assert!(err.to_string().contains("test"));

        let err = AuthError::Unauthorized;
        assert_eq!(err.to_string(), "Unauthorized");
    }
}
