//! Auth manager - high-level interface for authentication.
//!
//! This module provides the main AuthManager that coordinates identity,
//! session, capability, and verification operations.

use std::sync::Arc;

use crate::auth::capability::{create_capability, create_revocation, CapabilityManager};
use crate::auth::identity::{mine_identity_sync, verify_identity_pow};
use crate::auth::session::{create_session_token, SessionManager};
use crate::auth::storage::{AuthStorageAdapter, AUTH_NAMESPACE};
use crate::auth::types::{
    AuthError, Capability, CapabilityRef, Identity, IdentityUserData, Permission, ResourcePattern,
    Revocation, Session,
};
use crate::auth::verification::{verify_challenge_response, ChallengeStore};
use crate::storage::CausalStorage;

/// Configuration for the auth system.
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// Difficulty for identity mining (default: 4)
    pub identity_difficulty: u8,

    /// Challenge TTL in seconds (default: 300 = 5 min)
    pub challenge_ttl_seconds: i64,

    /// Session TTL in seconds (default: 86400 = 24 hours)
    pub session_ttl_seconds: i64,

    /// Whether to persist sessions (default: false)
    pub persist_sessions: bool,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            identity_difficulty: 4,
            challenge_ttl_seconds: 300,
            session_ttl_seconds: 86400,
            persist_sessions: false,
        }
    }
}

/// High-level authentication manager.
///
/// Coordinates all auth operations: identity management, challenge-response,
/// session management, and capability-based authorization.
pub struct AuthManager {
    /// Storage adapter for auth data
    storage: AuthStorageAdapter,

    /// In-memory challenge store
    challenges: ChallengeStore,

    /// In-memory session manager
    sessions: SessionManager,

    /// Capability manager (caches capabilities from storage)
    capabilities: CapabilityManager,

    /// Configuration
    config: AuthConfig,
}

impl AuthManager {
    /// Create a new auth manager.
    pub fn new(storage: Arc<CausalStorage>) -> Self {
        Self::with_config(storage, AuthConfig::default())
    }

    /// Create a new auth manager with custom config.
    pub fn with_config(storage: Arc<CausalStorage>, config: AuthConfig) -> Self {
        Self {
            storage: AuthStorageAdapter::new(storage),
            challenges: ChallengeStore::with_ttl(config.challenge_ttl_seconds),
            sessions: SessionManager::with_ttl(config.session_ttl_seconds),
            capabilities: CapabilityManager::new(),
            config,
        }
    }

    // ========================================================================
    // Identity Operations
    // ========================================================================

    /// Mine and register a new identity.
    ///
    /// This is the synchronous version for convenience.
    /// For async contexts, use `mine_identity` and `register_identity` separately.
    pub fn create_identity(
        &self,
        user_data: IdentityUserData,
    ) -> Result<(Identity, Vec<u8>), AuthError> {


        let mined = mine_identity_sync(user_data, self.config.identity_difficulty);

        // Store the identity
        self.register_identity(&mined.identity)?;

        Ok((mined.identity, mined.secret_key))
    }

    /// Register an existing identity.
    pub fn register_identity(&self, identity: &Identity) -> Result<(), AuthError> {
        // Verify proof-of-work
        if !verify_identity_pow(identity) {
            return Err(AuthError::InvalidProofOfWork);
        }

        // Check if identity already exists
        if self.storage.identity_exists(&identity.public_key)? {
            return Err(AuthError::IdentityExists(identity.public_key.clone()));
        }

        // Store it
        self.storage.store_identity(identity)?;

        Ok(())
    }

    /// Get an identity by public key.
    pub fn get_identity(&self, public_key: &str) -> Result<Option<Identity>, AuthError> {
        self.storage.get_identity(public_key)
    }

    /// Update an identity's user data.
    pub fn update_identity(
        &self,
        identity: &Identity,
        _secret_key: &[u8], // Required for future signature verification
    ) -> Result<(), AuthError> {
        // TODO: Verify signature of update with secret_key
        self.storage.update_identity(identity)
    }

    /// Get identity history.
    pub fn get_identity_history(&self, public_key: &str) -> Result<Vec<Identity>, AuthError> {
        self.storage.get_identity_history(public_key)
    }

    // ========================================================================
    // Challenge-Response Authentication
    // ========================================================================

    /// Create a challenge for an identity.
    ///
    /// Returns the challenge string that must be signed by the identity.
    pub fn create_challenge(&self, public_key: &str) -> Result<String, AuthError> {
        // Verify identity exists
        if !self.storage.identity_exists(public_key)? {
            return Err(AuthError::IdentityNotFound(public_key.to_string()));
        }

        let challenge = self.challenges.create_challenge(public_key);
        Ok(challenge.challenge)
    }

    /// Verify a challenge response and create a session.
    ///
    /// # Arguments
    /// * `public_key` - The identity's public key
    /// * `challenge` - The challenge string
    /// * `response` - The signed response (base58 encoded signature)
    ///
    /// # Returns
    /// Session ID on success.
    pub fn verify_and_create_session(
        &self,
        public_key: &str,
        challenge: &str,
        response: &str,
    ) -> Result<Session, AuthError> {
        // Verify challenge-response
        verify_challenge_response(&self.challenges, public_key, challenge, response)?;

        // Load capabilities for this identity
        let capabilities = self.storage.get_active_capabilities(public_key)?;
        let capability_refs: Vec<CapabilityRef> = capabilities
            .into_iter()
            .map(|cap| CapabilityRef {
                capability_key: format!("{}:capability:{}", AUTH_NAMESPACE, cap.id),
                resource_pattern: cap.resource_pattern,
                permission: cap.permission,
            })
            .collect();

        // Create session
        let (session, _keys) =
            self.sessions
                .create_session(public_key, challenge, capability_refs);

        Ok(session)
    }

    // ========================================================================
    // Session Operations
    // ========================================================================

    /// Validate a session.
    pub fn validate_session(&self, session_id: &str) -> Result<Session, AuthError> {
        self.sessions.validate_session(session_id)
    }

    /// Get session info.
    pub fn get_session(&self, session_id: &str) -> Result<Session, AuthError> {
        self.validate_session(session_id)
    }

    /// Revoke a session.
    pub fn revoke_session(&self, session_id: &str) -> Result<(), AuthError> {
        self.sessions.revoke_session(session_id)
    }

    /// Revoke all sessions for an identity.
    pub fn revoke_all_sessions(&self, identity_key: &str) -> usize {
        self.sessions.revoke_all_identity_sessions(identity_key)
    }

    /// Create a session token for stateless authentication.
    pub fn create_session_token(&self, session_id: &str) -> Result<String, AuthError> {
        let keys = self.sessions.get_session_keys(session_id)?;
        let timestamp = chrono::Utc::now();
        create_session_token(&keys, timestamp)
    }

    /// Cleanup expired challenges and sessions.
    pub fn cleanup(&self) -> (usize, usize) {
        let challenges_cleaned = self.challenges.cleanup_expired();
        let sessions_cleaned = self.sessions.cleanup_expired();
        (challenges_cleaned, sessions_cleaned)
    }

    // ========================================================================
    // Capability Operations
    // ========================================================================

    /// Grant a capability.
    ///
    /// # Arguments
    /// * `granter_key` - The granter's secret key (must be Admin on resource)
    /// * `grantee` - The public key receiving the capability
    /// * `resource_pattern` - What resources this applies to
    /// * `permission` - The level of access
    /// * `expires_at` - Optional expiration
    pub fn grant_capability(
        &mut self,
        granter_identity: &Identity,
        granter_secret_key: &[u8],
        grantee: &str,
        resource_pattern: ResourcePattern,
        permission: Permission,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<Capability, AuthError> {
        // Verify granter exists
        if !self.storage.identity_exists(&granter_identity.public_key)? {
            return Err(AuthError::IdentityNotFound(
                granter_identity.public_key.clone(),
            ));
        }

        // Verify grantee exists
        if !self.storage.identity_exists(grantee)? {
            return Err(AuthError::IdentityNotFound(grantee.to_string()));
        }

        // TODO: Verify granter has Admin permission on the resource pattern
        // This requires checking existing capabilities

        // Create capability
        let capability = create_capability(
            granter_identity,
            granter_secret_key,
            grantee,
            resource_pattern,
            permission,
            expires_at,
        )?;

        // Store it
        self.storage.store_capability(&capability)?;

        // Add to cache
        self.capabilities.add_capability(capability.clone());

        Ok(capability)
    }

    /// Revoke a capability.
    pub fn revoke_capability(
        &mut self,
        capability: &Capability,
        revoker_secret_key: &[u8],
        reason: Option<String>,
    ) -> Result<Revocation, AuthError> {
        // Create revocation
        let revocation = create_revocation(capability, revoker_secret_key, reason)?;

        // Store it
        self.storage.store_revocation(&revocation)?;

        // Add to cache
        self.capabilities.add_revocation(revocation.clone());

        Ok(revocation)
    }

    /// Authorize access to a resource.
    pub fn authorize(
        &self,
        identity_key: &str,
        namespace: &str,
        key: &str,
        required_permission: Permission,
    ) -> Result<CapabilityRef, AuthError> {
        // Get active capabilities from storage
        let capabilities = self.storage.get_active_capabilities(identity_key)?;

        // Get revocations
        let revocations = self.storage.list_all_revocations()?;

        // Check authorization
        crate::auth::capability::authorize(
            identity_key,
            namespace,
            key,
            required_permission,
            &capabilities,
            &revocations,
        )
    }

    /// Check if an identity has a permission on a resource.
    pub fn check_permission(
        &self,
        identity_key: &str,
        namespace: &str,
        key: &str,
        permission: Permission,
    ) -> bool {
        self.authorize(identity_key, namespace, key, permission).is_ok()
    }

    /// Get all capabilities for an identity.
    pub fn get_capabilities(&self, identity_key: &str) -> Result<Vec<Capability>, AuthError> {
        self.storage.get_active_capabilities(identity_key)
    }

    /// Get capabilities granted by an identity.
    pub fn get_granted_capabilities(
        &self,
        granter_key: &str,
    ) -> Result<Vec<Capability>, AuthError> {
        self.storage.list_capabilities_by_granter(granter_key)
    }

    /// Refresh capability cache from storage.
    pub fn refresh_capabilities(&mut self) -> Result<(), AuthError> {
        let all_capabilities = self.storage.list_all_capabilities()?;
        let all_revocations = self.storage.list_all_revocations()?;

        self.capabilities.load(all_capabilities, all_revocations);
        Ok(())
    }

    // ========================================================================
    // Utility
    // ========================================================================

    /// Get storage adapter for advanced operations.
    pub fn storage(&self) -> &AuthStorageAdapter {
        &self.storage
    }

    /// Get configuration.
    pub fn config(&self) -> &AuthConfig {
        &self.config
    }

    /// Get stats about auth state.
    pub fn stats(&self) -> AuthStats {
        AuthStats {
            active_challenges: self.challenges.len(),
            active_sessions: self.sessions.len(),
        }
    }
}

/// Auth statistics.
#[derive(Debug, Clone)]
pub struct AuthStats {
    pub active_challenges: usize,
    pub active_sessions: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_manager() -> AuthManager {
        let storage = Arc::new(CausalStorage::new(
            std::sync::Arc::new(koru_lambda_core::DistinctionEngine::new()),
        ));
        AuthManager::new(storage)
    }

    #[test]
    fn test_create_and_get_identity() {
        let mut manager = create_test_manager();

        let user_data = IdentityUserData {
            display_name: Some("Alice".to_string()),
            ..Default::default()
        };

        // Create identity
        let (identity, _secret_key) = manager.create_identity(user_data).unwrap();

        // Get it back
        let retrieved = manager.get_identity(&identity.public_key).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().public_key, identity.public_key);
    }

    #[test]
    fn test_duplicate_identity() {
        let mut manager = create_test_manager();

        let user_data = IdentityUserData::default();
        let (identity, _secret_key) = manager.create_identity(user_data.clone()).unwrap();

        // Try to register again
        let result = manager.register_identity(&identity);
        assert!(matches!(result, Err(AuthError::IdentityExists(_))));
    }

    #[test]
    fn test_full_auth_flow() {
        let mut manager = create_test_manager();

        // 1. Create identity
        let user_data = IdentityUserData::default();
        let (identity, secret_key) = manager.create_identity(user_data).unwrap();

        // 2. Create challenge
        let challenge = manager.create_challenge(&identity.public_key).unwrap();

        // 3. Create response (client-side)
        let response =
            crate::auth::verification::create_challenge_response(&secret_key, &challenge).unwrap();

        // 4. Verify and create session
        let session = manager
            .verify_and_create_session(&identity.public_key, &challenge, &response)
            .unwrap();

        assert!(!session.session_id.is_empty());
        assert_eq!(session.identity_key, identity.public_key);

        // 5. Validate session
        let validated = manager.validate_session(&session.session_id).unwrap();
        assert_eq!(validated.session_id, session.session_id);

        // 6. Revoke session
        manager.revoke_session(&session.session_id).unwrap();

        // Should fail now
        let result = manager.validate_session(&session.session_id);
        assert!(matches!(result, Err(AuthError::SessionExpired)));
    }

    #[test]
    fn test_capability_management() {
        let mut manager = create_test_manager();

        // Create granter
        let (granter, granter_key) = manager.create_identity(IdentityUserData::default()).unwrap();

        // Create grantee
        let (grantee, _grantee_key) = manager.create_identity(IdentityUserData::default()).unwrap();

        // Grant capability
        let cap = manager
            .grant_capability(
                &granter,
                &granter_key,
                &grantee.public_key,
                ResourcePattern::Exact("test:resource".to_string()),
                Permission::Read,
                None,
            )
            .unwrap();

        assert_eq!(cap.granter, granter.public_key);
        assert_eq!(cap.grantee, grantee.public_key);

        // Check authorization
        assert!(manager.check_permission(&grantee.public_key, "test", "resource", Permission::Read));
        assert!(!manager.check_permission(&grantee.public_key, "test", "resource", Permission::Write));

        // Get capabilities
        let caps = manager.get_capabilities(&grantee.public_key).unwrap();
        assert_eq!(caps.len(), 1);

        // Revoke capability
        manager.revoke_capability(&cap, &granter_key, None).unwrap();

        // Should no longer be authorized
        assert!(!manager.check_permission(&grantee.public_key, "test", "resource", Permission::Read));
    }

    #[test]
    fn test_authorize_without_capability() {
        let mut manager = create_test_manager();

        // Create identity but don't grant any capabilities
        let (identity, _secret_key) = manager.create_identity(IdentityUserData::default()).unwrap();

        // Should not be authorized for anything
        assert!(!manager.check_permission(&identity.public_key, "any", "resource", Permission::Read));
    }

    #[test]
    fn test_admin_permission() {
        let mut manager = create_test_manager();

        let (granter, granter_key) = manager.create_identity(IdentityUserData::default()).unwrap();
        let (grantee, _grantee_key) = manager.create_identity(IdentityUserData::default()).unwrap();

        // Grant Admin capability
        manager
            .grant_capability(
                &granter,
                &granter_key,
                &grantee.public_key,
                ResourcePattern::Namespace("test".to_string()),
                Permission::Admin,
                None,
            )
            .unwrap();

        // Admin includes Read, Write, Admin
        assert!(manager.check_permission(&grantee.public_key, "test", "anything", Permission::Read));
        assert!(manager.check_permission(&grantee.public_key, "test", "anything", Permission::Write));
        assert!(manager.check_permission(&grantee.public_key, "test", "anything", Permission::Admin));
    }

    #[test]
    fn test_cleanup() {
        let mut manager = create_test_manager();

        // Create identity and challenge
        let (identity, _secret_key) = manager.create_identity(IdentityUserData::default()).unwrap();
        let _challenge = manager.create_challenge(&identity.public_key).unwrap();

        assert_eq!(manager.stats().active_challenges, 1);

        // Cleanup shouldn't remove non-expired items
        let (challenges, sessions) = manager.cleanup();
        assert_eq!(challenges, 0);
        assert_eq!(sessions, 0);
        assert_eq!(manager.stats().active_challenges, 1);
    }

    #[test]
    fn test_invalid_challenge_response() {
        let manager = create_test_manager();

        let (identity, _secret_key) = manager.create_identity(IdentityUserData::default()).unwrap();
        let challenge = manager.create_challenge(&identity.public_key).unwrap();

        // Wrong response
        let result = manager.verify_and_create_session(&identity.public_key, &challenge, "invalid");
        assert!(matches!(result, Err(AuthError::InvalidSignature)));
    }

    #[test]
    fn test_nonexistent_identity() {
        let manager = create_test_manager();

        let result = manager.create_challenge("nonexistent_identity");
        assert!(matches!(result, Err(AuthError::IdentityNotFound(_))));
    }
}
