//! Identity Agent - self-sovereign authentication via LCA synthesis.
//!
//! This module provides the IdentityAgent that coordinates identity,
//! session, capability, and verification operations through the unified
//! distinction field. All operations follow the LCA pattern:
//!
//! ```text
//! ΔNew = ΔLocal_Root ⊕ ΔAction_Data
//! ```
//!
//! # Core Concepts
//!
//! ## Identity as Distinction
//! An identity is mined as a distinction within the field, containing:
//! - Ed25519 public key (as distinction content)
//! - User data (synthesized into the identity)
//! - Proof-of-work (prevents spam, verified as distinction validity)
//!
//! ## Capability Chains
//! Capabilities form chains in the distinction graph:
//! - Each grant synthesizes granter → grantee with permission
//! - Revocations are tombstone distinctions
//! - Authorization traces paths through the graph

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use koru_lambda_core::{Canonicalizable, Distinction, DistinctionEngine};

use crate::actions::IdentityAction;
use crate::auth::capability::{create_capability, create_revocation, CapabilityManager};
use crate::auth::identity::{mine_identity_sync, verify_identity_pow};
use crate::auth::session::{create_session_token, SessionAgent};
use crate::auth::storage::AuthStorageAdapter;
use crate::auth::types::{
    AuthError, Capability, CapabilityRef, Identity, IdentityUserData, Permission, ResourcePattern,
    Revocation, Session,
};
use crate::auth::verification::{verify_challenge_response, ChallengeStore};
use crate::engine::{FieldHandle, SharedEngine};
use crate::roots::RootType;
use crate::storage::CausalStorage;

/// Configuration for the identity agent.
#[derive(Debug, Clone)]
pub struct IdentityConfig {
    /// Difficulty for identity mining (default: 4)
    pub identity_difficulty: u8,

    /// Challenge TTL in seconds (default: 300 = 5 min)
    pub challenge_ttl_seconds: i64,

    /// Session TTL in seconds (default: 86400 = 24 hours)
    pub session_ttl_seconds: i64,

    /// Whether to persist sessions (default: false)
    pub persist_sessions: bool,
}

impl Default for IdentityConfig {
    fn default() -> Self {
        Self {
            identity_difficulty: 4,
            challenge_ttl_seconds: 300,
            session_ttl_seconds: 86400,
            persist_sessions: false,
        }
    }
}

/// Identity Agent - self-sovereign authentication via LCA synthesis.
///
/// Coordinates all identity operations through the unified field:
/// - Identity management (mining, registration)
/// - Challenge-response authentication
/// - Session management
/// - Capability-based authorization
///
/// # LCA Architecture
///
/// All operations synthesize: `ΔNew = ΔLocal_Root ⊕ ΔIdentity_Action`
/// - Local root: RootType::Identity (canonical root for all identity)
/// - Identities: Synthesis of all known identities in the field
///
/// Note: Uses internal mutability (RefCell) to allow &self API while
/// maintaining LCA synthesis state.
pub struct IdentityAgent {
    /// Storage adapter for auth data
    storage: AuthStorageAdapter,

    /// In-memory challenge store
    challenges: ChallengeStore,

    /// In-memory session manager
    sessions: SessionAgent,

    /// Capability manager (caches capabilities from storage)
    capabilities: RwLock<CapabilityManager>,

    /// Configuration
    config: IdentityConfig,

    /// LCA: Local root distinction (Root: IDENTITY)
    local_root: RwLock<Distinction>,

    /// LCA: Synthesis of all identities
    identities: RwLock<Distinction>,

    /// LCA: Handle to the shared field
    field: FieldHandle,

    /// Statistics
    identities_mined: AtomicU64,
    sessions_created: AtomicU64,
    capabilities_granted: AtomicU64,
}

// IdentityAgent is !Sync due to RefCell, but we can still use it in single-threaded contexts
// For multi-threaded access, wrap in a Mutex or RwLock

impl IdentityAgent {
    /// Create a new identity agent.
    ///
    /// # LCA Pattern
    ///
    /// The agent initializes with:
    /// - `local_root` = RootType::Identity (from shared field roots)
    /// - `identities` = Synthesis of all identity distinctions
    /// - `field` = Handle to the unified distinction engine
    pub fn new(storage: Arc<CausalStorage>, shared_engine: &SharedEngine) -> Self {
        Self::with_config(storage, IdentityConfig::default(), shared_engine)
    }

    /// Create a new identity agent with custom config.
    ///
    /// # LCA Pattern
    ///
    /// The agent anchors to the IDENTITY root, which is synthesized
    /// from the primordial distinctions (d0, d1) in the shared field.
    pub fn with_config(
        storage: Arc<CausalStorage>,
        config: IdentityConfig,
        shared_engine: &SharedEngine,
    ) -> Self {
        let local_root = shared_engine.root(RootType::Identity).clone();
        let identities = shared_engine.root(RootType::Identity).clone();
        let field = FieldHandle::new(shared_engine);

        Self {
            storage: AuthStorageAdapter::new(storage),
            challenges: ChallengeStore::with_ttl(config.challenge_ttl_seconds),
            sessions: SessionAgent::with_ttl(shared_engine, config.session_ttl_seconds),
            capabilities: RwLock::new(CapabilityManager::new()),
            config,
            local_root: RwLock::new(local_root),
            identities: RwLock::new(identities),
            field,
            identities_mined: AtomicU64::new(0),
            sessions_created: AtomicU64::new(0),
            capabilities_granted: AtomicU64::new(0),
        }
    }

    // ========================================================================
    // LCA Synthesis Helpers
    // ========================================================================

    /// Internal synthesis helper.
    ///
    /// Performs the LCA synthesis: `ΔNew = ΔLocal_Root ⊕ ΔAction`
    fn synthesize_action_internal(&self, action: IdentityAction) -> Distinction {
        let engine = self.field.engine_arc();
        let action_distinction = action.to_canonical_structure(engine);
        let local_root = self.local_root.read().unwrap().clone();
        let new_root = engine.synthesize(&local_root, &action_distinction);
        *self.local_root.write().unwrap() = new_root.clone();
        new_root
    }

    // ========================================================================
    // Identity Operations
    // ========================================================================

    /// Mine and register a new identity.
    ///
    /// # LCA Pattern
    ///
    /// Mining synthesizes: `ΔNew = ΔLocal_Root ⊕ ΔMineIdentity_Action`
    /// then the identity is synthesized into the identities distinction.
    pub fn create_identity(&self, user_data: IdentityUserData) -> Result<(Identity, Vec<u8>), AuthError> {
        // Synthesize mine identity action
        let pow_json = serde_json::json!({
            "difficulty": self.config.identity_difficulty,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        let action = IdentityAction::MineIdentity {
            proof_of_work_json: pow_json,
        };
        let _ = self.synthesize_action_internal(action);

        let mined = mine_identity_sync(user_data, self.config.identity_difficulty);

        // Store the identity
        self.register_identity(&mined.identity)?;

        self.identities_mined.fetch_add(1, Ordering::SeqCst);

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

    /// Verify that an identity exists and has valid proof-of-work.
    ///
    /// This is a convenience method for checking identity validity.
    /// Returns true if the identity exists and passes verification.
    ///
    /// # Arguments
    ///
    /// * `public_key` - The public key of the identity to verify
    ///
    /// # Example
    ///
    /// ```ignore
    /// if auth.verify_identity("C4nCw...").await? {
    ///     println!("Identity is valid");
    /// }
    /// ```
    pub async fn verify_identity(&self, public_key: &str) -> Result<bool, AuthError> {
        match self.get_identity(public_key)? {
            Some(identity) => {
                // Verify the proof-of-work is still valid
                Ok(verify_identity_pow(&identity))
            }
            None => Ok(false),
        }
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
    /// # LCA Pattern
    ///
    /// Authentication synthesizes: `ΔNew = ΔLocal_Root ⊕ ΔAuthenticate_Action`
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
        // Synthesize authenticate action
        let action = IdentityAction::Authenticate {
            identity_id: public_key.to_string(),
            challenge: challenge.to_string(),
        };
        let _ = self.synthesize_action_internal(action);

        // Verify challenge-response
        verify_challenge_response(&self.challenges, public_key, challenge, response)?;

        // Load capabilities for this identity
        let capabilities = self.storage.get_active_capabilities(public_key)?;
        let capability_refs: Vec<CapabilityRef> = capabilities
            .into_iter()
            .map(|cap| crate::auth::capability::build_capability_ref(&cap))
            .collect();

        // Create session
        let (session, _keys) = self
            .sessions
            .create_session(public_key, challenge, capability_refs);

        self.sessions_created.fetch_add(1, Ordering::SeqCst);

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
    /// # LCA Pattern
    ///
    /// Granting synthesizes: `ΔNew = ΔLocal_Root ⊕ ΔGrantCapability_Action`
    ///
    /// # Arguments
    /// * `granter_key` - The granter's secret key (must be Admin on resource)
    /// * `grantee` - The public key receiving the capability
    /// * `resource_pattern` - What resources this applies to
    /// * `permission` - The level of access
    /// * `expires_at` - Optional expiration
    pub fn grant_capability(
        &self,
        granter_identity: &Identity,
        granter_secret_key: &[u8],
        grantee: &str,
        resource_pattern: ResourcePattern,
        permission: Permission,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<Capability, AuthError> {
        // Synthesize grant capability action
        let action = IdentityAction::GrantCapability {
            from_id: granter_identity.public_key.clone(),
            to_id: grantee.to_string(),
            permission: format!("{:?}", permission),
        };
        let _ = self.synthesize_action_internal(action);

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
        self.capabilities.write().unwrap().add_capability(capability.clone());

        self.capabilities_granted.fetch_add(1, Ordering::SeqCst);

        Ok(capability)
    }

    /// Revoke a capability.
    pub fn revoke_capability(
        &self,
        capability: &Capability,
        revoker_secret_key: &[u8],
        reason: Option<String>,
    ) -> Result<Revocation, AuthError> {
        // Create revocation
        let revocation = create_revocation(capability, revoker_secret_key, reason)?;

        // Store it
        self.storage.store_revocation(&revocation)?;

        // Add to cache
        self.capabilities.write().unwrap().add_revocation(revocation.clone());

        Ok(revocation)
    }

    /// Authorize access to a resource.
    ///
    /// # LCA Pattern
    ///
    /// Authorization synthesizes: `ΔNew = ΔLocal_Root ⊕ ΔVerifyAccess_Action`
    pub fn authorize(
        &self,
        identity_key: &str,
        namespace: &str,
        key: &str,
        required_permission: Permission,
    ) -> Result<CapabilityRef, AuthError> {
        // Synthesize verify access action
        let action = IdentityAction::VerifyAccess {
            identity_id: identity_key.to_string(),
            resource: format!("{}:{}", namespace, key),
        };
        let _ = self.synthesize_action_internal(action);

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
        self.authorize(identity_key, namespace, key, permission)
            .is_ok()
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
    pub fn refresh_capabilities(&self) -> Result<(), AuthError> {
        let all_capabilities = self.storage.list_all_capabilities()?;
        let all_revocations = self.storage.list_all_revocations()?;

        self.capabilities.write().unwrap().load(all_capabilities, all_revocations);
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
    pub fn config(&self) -> &IdentityConfig {
        &self.config
    }

    /// Get stats about identity state.
    pub fn stats(&self) -> IdentityStats {
        IdentityStats {
            active_challenges: self.challenges.len(),
            active_sessions: self.sessions.len(),
            identities_mined: self.identities_mined.load(Ordering::SeqCst),
            sessions_created: self.sessions_created.load(Ordering::SeqCst),
            capabilities_granted: self.capabilities_granted.load(Ordering::SeqCst),
        }
    }

    /// Get the identities distinction (synthesis of all identities).
    pub fn identities_distinction(&self) -> Distinction {
        self.identities.read().unwrap().clone()
    }
}

/// Identity statistics.
#[derive(Debug, Clone)]
pub struct IdentityStats {
    pub active_challenges: usize,
    pub active_sessions: usize,
    pub identities_mined: u64,
    pub sessions_created: u64,
    pub capabilities_granted: u64,
}

/// LCA Pattern Implementation for IdentityAgent
///
/// All operations follow the synthesis pattern:
/// ```text
/// ΔNew = ΔLocal_Root ⊕ ΔAction_Data
/// ```
///
/// Note: This implementation uses internal mutability (RwLock) to allow
/// the LCA pattern to work through &self methods, enabling use in
/// multi-threaded contexts like HTTP handlers.
impl IdentityAgent {
    /// Get the current local root distinction.
    pub fn get_local_root(&self) -> Distinction {
        self.local_root.read().unwrap().clone()
    }

    /// Update the local root (used by LCA synthesis).
    pub fn update_local_root(&self, new_root: Distinction) {
        *self.local_root.write().unwrap() = new_root;
    }

    /// Synthesize an action with the current local root.
    ///
    /// Follows the LCA pattern: `ΔNew = ΔLocal_Root ⊕ ΔAction`
    pub fn synthesize_action(
        &self,
        action: IdentityAction,
        engine: &Arc<DistinctionEngine>,
    ) -> Distinction {
        let action_distinction = action.to_canonical_structure(engine);
        let local_root = self.local_root.read().unwrap().clone();
        let new_root = engine.synthesize(&local_root, &action_distinction);
        *self.local_root.write().unwrap() = new_root.clone();
        new_root
    }
}

// ============================================================================
// LCA Pattern Verification
// ============================================================================

// Note: IdentityAgent follows the LCA pattern internally:
// - Has local_root (RootType::Identity)
// - All mutation operations synthesize: ΔNew = ΔLocal_Root ⊕ ΔAction
// - Uses interior mutability for ergonomic &self API
//
// The LocalCausalAgent trait is not implemented because the trait requires
// &mut self for synthesize_action, which would force an ergonomic regression
// on the public API. The architecture is followed; the trait is omitted.

// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::SharedEngine;

    fn create_test_manager() -> IdentityAgent {
        let shared_engine = SharedEngine::new();
        let storage = Arc::new(CausalStorage::new(Arc::clone(shared_engine.inner())));
        IdentityAgent::new(storage, &shared_engine)
    }

    #[test]
    fn test_create_and_get_identity() {
        let manager = create_test_manager();

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
        let manager = create_test_manager();

        let user_data = IdentityUserData::default();
        let (identity, _secret_key) = manager.create_identity(user_data.clone()).unwrap();

        // Try to register again
        let result = manager.register_identity(&identity);
        assert!(matches!(result, Err(AuthError::IdentityExists(_))));
    }

    #[test]
    fn test_full_auth_flow() {
        let manager = create_test_manager();

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
        let manager = create_test_manager();

        // Create granter
        let (granter, granter_key) = manager
            .create_identity(IdentityUserData::default())
            .unwrap();

        // Create grantee
        let (grantee, _grantee_key) = manager
            .create_identity(IdentityUserData::default())
            .unwrap();

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
        assert!(manager.check_permission(
            &grantee.public_key,
            "test",
            "resource",
            Permission::Read
        ));
        assert!(!manager.check_permission(
            &grantee.public_key,
            "test",
            "resource",
            Permission::Write
        ));

        // Get capabilities
        let caps = manager.get_capabilities(&grantee.public_key).unwrap();
        assert_eq!(caps.len(), 1);

        // Revoke capability
        manager.revoke_capability(&cap, &granter_key, None).unwrap();

        // Should no longer be authorized
        assert!(!manager.check_permission(
            &grantee.public_key,
            "test",
            "resource",
            Permission::Read
        ));
    }

    #[test]
    fn test_authorize_without_capability() {
        let manager = create_test_manager();

        // Create identity but don't grant any capabilities
        let (identity, _secret_key) = manager
            .create_identity(IdentityUserData::default())
            .unwrap();

        // Should not be authorized for anything
        assert!(!manager.check_permission(
            &identity.public_key,
            "any",
            "resource",
            Permission::Read
        ));
    }

    #[test]
    fn test_admin_permission() {
        let manager = create_test_manager();

        let (granter, granter_key) = manager
            .create_identity(IdentityUserData::default())
            .unwrap();
        let (grantee, _grantee_key) = manager
            .create_identity(IdentityUserData::default())
            .unwrap();

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
        assert!(manager.check_permission(
            &grantee.public_key,
            "test",
            "anything",
            Permission::Read
        ));
        assert!(manager.check_permission(
            &grantee.public_key,
            "test",
            "anything",
            Permission::Write
        ));
        assert!(manager.check_permission(
            &grantee.public_key,
            "test",
            "anything",
            Permission::Admin
        ));
    }

    #[test]
    fn test_cleanup() {
        let manager = create_test_manager();

        // Create identity and challenge
        let (identity, _secret_key) = manager
            .create_identity(IdentityUserData::default())
            .unwrap();
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

        let (identity, _secret_key) = manager
            .create_identity(IdentityUserData::default())
            .unwrap();
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

    #[test]
    fn test_stats_tracking() {
        let manager = create_test_manager();

        // Initial stats
        let stats = manager.stats();
        assert_eq!(stats.identities_mined, 0);
        assert_eq!(stats.sessions_created, 0);
        assert_eq!(stats.capabilities_granted, 0);

        // Create identity
        let (identity, secret_key) = manager
            .create_identity(IdentityUserData::default())
            .unwrap();
        assert_eq!(manager.stats().identities_mined, 1);

        // Create session
        let challenge = manager.create_challenge(&identity.public_key).unwrap();
        let response =
            crate::auth::verification::create_challenge_response(&secret_key, &challenge).unwrap();
        let _session = manager
            .verify_and_create_session(&identity.public_key, &challenge, &response)
            .unwrap();
        assert_eq!(manager.stats().sessions_created, 1);

        // Grant capability
        let (grantee, _grantee_key) = manager
            .create_identity(IdentityUserData::default())
            .unwrap();
        manager
            .grant_capability(
                &identity,
                &secret_key,
                &grantee.public_key,
                ResourcePattern::Exact("test:resource".to_string()),
                Permission::Read,
                None,
            )
            .unwrap();
        assert_eq!(manager.stats().capabilities_granted, 1);
    }
}
