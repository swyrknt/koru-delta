//! Storage adapter for auth data using CausalStorage.
//!
//! This module provides the bridge between auth types and CausalStorage,
//! storing identities, capabilities, and revocations as distinctions.

use std::sync::Arc;


use crate::auth::types::{AuthError, Capability, Identity, Revocation};
use crate::storage::CausalStorage;

/// Namespace for auth-related distinctions.
pub const AUTH_NAMESPACE: &str = "_auth";

/// Storage adapter for auth data.
pub struct AuthStorageAdapter {
    storage: Arc<CausalStorage>,
}

impl AuthStorageAdapter {
    /// Create a new auth storage adapter.
    pub fn new(storage: Arc<CausalStorage>) -> Self {
        Self { storage }
    }

    // =========================================================================
    // Identity Operations
    // =========================================================================

    /// Store an identity as a distinction.
    pub fn store_identity(&self, identity: &Identity) -> Result<(), AuthError> {
        let key = identity_key(&identity.public_key);
        let value = serde_json::to_value(identity)?;

        self.storage
            .put(AUTH_NAMESPACE, &key, value)
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        Ok(())
    }

    /// Get an identity by public key.
    pub fn get_identity(&self, public_key: &str) -> Result<Option<Identity>, AuthError> {
        let key = identity_key(public_key);

        match self.storage.get(AUTH_NAMESPACE, &key) {
            Ok(versioned) => {
                let identity: Identity = serde_json::from_value(versioned.value.as_ref().clone())?;
                Ok(Some(identity))
            }
            Err(crate::DeltaError::KeyNotFound { .. }) => Ok(None),
            Err(e) => Err(AuthError::Storage(e.to_string())),
        }
    }

    /// Check if an identity exists.
    pub fn identity_exists(&self, public_key: &str) -> Result<bool, AuthError> {
        self.get_identity(public_key).map(|i| i.is_some())
    }

    /// Update an identity's user data.
    /// This creates a new version while preserving history.
    pub fn update_identity(&self, identity: &Identity) -> Result<(), AuthError> {
        let key = identity_key(&identity.public_key);
        let value = serde_json::to_value(identity)?;

        self.storage
            .put(AUTH_NAMESPACE, &key, value)
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        Ok(())
    }

    /// Get the history of an identity.
    pub fn get_identity_history(&self, public_key: &str) -> Result<Vec<Identity>, AuthError> {
        let key = identity_key(public_key);

        let history = self
            .storage
            .history(AUTH_NAMESPACE, &key)
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        let mut identities = Vec::new();
        for entry in history {
            let identity: Identity = serde_json::from_value(entry.value.clone())?;
            identities.push(identity);
        }

        Ok(identities)
    }

    // =========================================================================
    // Capability Operations
    // =========================================================================

    /// Store a capability as a distinction.
    pub fn store_capability(&self, capability: &Capability) -> Result<String, AuthError> {
        let key = capability_key(&capability.id);
        let value = serde_json::to_value(capability)?;

        self.storage
            .put(AUTH_NAMESPACE, &key, value)
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        Ok(key)
    }

    /// Get a capability by ID.
    pub fn get_capability(&self, capability_id: &str) -> Result<Option<Capability>, AuthError> {
        let key = capability_key(capability_id);

        match self.storage.get(AUTH_NAMESPACE, &key) {
            Ok(versioned) => {
                let cap: Capability = serde_json::from_value((*versioned.value).clone())?;
                Ok(Some(cap))
            }
            Err(crate::DeltaError::KeyNotFound { .. }) => Ok(None),
            Err(e) => Err(AuthError::Storage(e.to_string())),
        }
    }

    /// List all capabilities for a grantee (identity receiving permissions).
    pub fn list_capabilities_for_grantee(
        &self,
        grantee: &str,
    ) -> Result<Vec<Capability>, AuthError> {
        // Query all capabilities in auth namespace
        let all = self.list_all_capabilities()?;

        let filtered: Vec<Capability> = all
            .into_iter()
            .filter(|cap| cap.grantee == grantee)
            .collect();

        Ok(filtered)
    }

    /// List all capabilities granted by a granter.
    pub fn list_capabilities_by_granter(
        &self,
        granter: &str,
    ) -> Result<Vec<Capability>, AuthError> {
        let all = self.list_all_capabilities()?;

        let filtered: Vec<Capability> = all
            .into_iter()
            .filter(|cap| cap.granter == granter)
            .collect();

        Ok(filtered)
    }

    /// List all capabilities in storage.
    pub fn list_all_capabilities(&self) -> Result<Vec<Capability>, AuthError> {
        self.list_by_prefix("capability:")
    }

    /// Query capabilities matching a resource pattern.
    pub fn query_capabilities_for_resource(
        &self,
        namespace: &str,
        key: &str,
    ) -> Result<Vec<Capability>, AuthError> {
        let all = self.list_all_capabilities()?;

        let matching: Vec<Capability> = all
            .into_iter()
            .filter(|cap| cap.resource_pattern.matches(namespace, key))
            .collect();

        Ok(matching)
    }

    // =========================================================================
    // Revocation Operations
    // =========================================================================

    /// Store a revocation as a distinction.
    pub fn store_revocation(&self, revocation: &Revocation) -> Result<String, AuthError> {
        let key = revocation_key(&revocation.capability_id);
        let value = serde_json::to_value(revocation)?;

        self.storage
            .put(AUTH_NAMESPACE, &key, value)
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        Ok(key)
    }

    /// Get a revocation by capability ID.
    pub fn get_revocation(
        &self,
        capability_id: &str,
    ) -> Result<Option<Revocation>, AuthError> {
        let key = revocation_key(capability_id);

        match self.storage.get(AUTH_NAMESPACE, &key) {
            Ok(versioned) => {
                let rev: Revocation = serde_json::from_value((*versioned.value).clone())?;
                Ok(Some(rev))
            }
            Err(crate::DeltaError::KeyNotFound { .. }) => Ok(None),
            Err(e) => Err(AuthError::Storage(e.to_string())),
        }
    }

    /// Check if a capability is revoked.
    pub fn is_capability_revoked(&self, capability_id: &str) -> Result<bool, AuthError> {
        self.get_revocation(capability_id).map(|r| r.is_some())
    }

    /// List all revocations.
    pub fn list_all_revocations(&self) -> Result<Vec<Revocation>, AuthError> {
        self.list_revocations_by_prefix("revocation:")
    }

    /// List revocations for a specific granter.
    pub fn list_revocations_by_granter(
        &self,
        granter: &str,
    ) -> Result<Vec<Revocation>, AuthError> {
        let all = self.list_all_revocations()?;

        let filtered: Vec<Revocation> = all
            .into_iter()
            .filter(|rev| rev.revoked_by == granter)
            .collect();

        Ok(filtered)
    }

    // =========================================================================
    // Authorization Helpers
    // =========================================================================

    /// Get all active (non-revoked) capabilities for an identity.
    pub fn get_active_capabilities(
        &self,
        identity_key: &str,
    ) -> Result<Vec<Capability>, AuthError> {
        let all = self.list_capabilities_for_grantee(identity_key)?;
        let mut active = Vec::new();

        for cap in all {
            // Skip if revoked
            if self.is_capability_revoked(&cap.id)? {
                continue;
            }

            // Skip if expired
            if cap.is_expired() {
                continue;
            }

            active.push(cap);
        }

        Ok(active)
    }

    // =========================================================================
    // Private Helpers
    // =========================================================================

    fn list_by_prefix<T: serde::de::DeserializeOwned>(
        &self,
        prefix: &str,
    ) -> Result<Vec<T>, AuthError> {
        // Scan all items in auth namespace and filter by prefix
        let items = self
            .storage
            .scan_collection(AUTH_NAMESPACE);

        let mut results = Vec::new();
        for (key, versioned) in items {
            if key.starts_with(prefix) {
                match serde_json::from_value::<T>((*versioned.value).clone()) {
                    Ok(item) => results.push(item),
                    Err(_) => continue, // Skip invalid entries
                }
            }
        }

        Ok(results)
    }

    fn list_revocations_by_prefix(
        &self,
        prefix: &str,
    ) -> Result<Vec<Revocation>, AuthError> {
        self.list_by_prefix(prefix)
    }
}

/// Create storage key for an identity.
fn identity_key(public_key: &str) -> String {
    format!("identity:{}", public_key)
}

/// Create storage key for a capability.
fn capability_key(capability_id: &str) -> String {
    format!("capability:{}", capability_id)
}

/// Create storage key for a revocation.
fn revocation_key(capability_id: &str) -> String {
    format!("revocation:{}", capability_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::capability::create_capability;
    use crate::auth::identity::mine_identity_sync;
    use crate::auth::types::{IdentityUserData, Permission, ResourcePattern};

    fn create_test_storage() -> Arc<CausalStorage> {
        Arc::new(CausalStorage::new(
            std::sync::Arc::new(koru_lambda_core::DistinctionEngine::new()),
        ))
    }

    #[test]
    fn test_store_and_get_identity() {
        let storage = create_test_storage();
        let adapter = AuthStorageAdapter::new(storage);

        let mined = mine_identity_sync(IdentityUserData::default(), 2);

        // Store
        adapter.store_identity(&mined.identity).unwrap();

        // Get
        let retrieved = adapter.get_identity(&mined.identity.public_key).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().public_key, mined.identity.public_key);

        // Exists
        assert!(adapter.identity_exists(&mined.identity.public_key).unwrap());
        assert!(!adapter.identity_exists("nonexistent").unwrap());
    }

    #[test]
    fn test_identity_history() {
        let storage = create_test_storage();
        let adapter = AuthStorageAdapter::new(storage);

        let mut mined = mine_identity_sync(IdentityUserData::default(), 2);

        // Store initial
        adapter.store_identity(&mined.identity).unwrap();

        // Update
        mined.identity.user_data.display_name = Some("Updated".to_string());
        adapter.update_identity(&mined.identity).unwrap();

        // Get history
        let history = adapter.get_identity_history(&mined.identity.public_key).unwrap();
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_store_and_get_capability() {
        let storage = create_test_storage();
        let adapter = AuthStorageAdapter::new(storage);

        let (granter, secret_key) = {
            let mined = mine_identity_sync(IdentityUserData::default(), 2);
            (mined.identity, mined.secret_key)
        };
        let grantee = {
            let mined = mine_identity_sync(IdentityUserData::default(), 2);
            mined.identity.public_key
        };

        // Store granter first
        adapter.store_identity(&granter).unwrap();

        let cap = create_capability(
            &granter,
            &secret_key,
            &grantee,
            ResourcePattern::Exact("test:resource".to_string()),
            Permission::Read,
            None,
        )
        .unwrap();

        // Store
        let key = adapter.store_capability(&cap).unwrap();
        assert!(key.contains(&cap.id));

        // Get
        let retrieved = adapter.get_capability(&cap.id).unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, cap.id);
        assert_eq!(retrieved.granter, granter.public_key);
    }

    #[test]
    fn test_list_capabilities() {
        let storage = create_test_storage();
        let adapter = AuthStorageAdapter::new(storage);

        let (granter, secret_key) = {
            let mined = mine_identity_sync(IdentityUserData::default(), 2);
            (mined.identity, mined.secret_key)
        };
        let grantee1 = {
            let mined = mine_identity_sync(IdentityUserData::default(), 2);
            adapter.store_identity(&mined.identity).unwrap();
            mined.identity.public_key
        };
        let grantee2 = {
            let mined = mine_identity_sync(IdentityUserData::default(), 2);
            adapter.store_identity(&mined.identity).unwrap();
            mined.identity.public_key
        };

        adapter.store_identity(&granter).unwrap();

        // Create capabilities
        let cap1 = create_capability(
            &granter,
            &secret_key,
            &grantee1,
            ResourcePattern::Exact("test:resource1".to_string()),
            Permission::Read,
            None,
        )
        .unwrap();

        let cap2 = create_capability(
            &granter,
            &secret_key,
            &grantee1,
            ResourcePattern::Exact("test:resource2".to_string()),
            Permission::Write,
            None,
        )
        .unwrap();

        let cap3 = create_capability(
            &granter,
            &secret_key,
            &grantee2,
            ResourcePattern::Exact("test:resource3".to_string()),
            Permission::Admin,
            None,
        )
        .unwrap();

        adapter.store_capability(&cap1).unwrap();
        adapter.store_capability(&cap2).unwrap();
        adapter.store_capability(&cap3).unwrap();

        // List by grantee
        let grantee1_caps = adapter.list_capabilities_for_grantee(&grantee1).unwrap();
        assert_eq!(grantee1_caps.len(), 2);

        let grantee2_caps = adapter.list_capabilities_for_grantee(&grantee2).unwrap();
        assert_eq!(grantee2_caps.len(), 1);

        // List by granter
        let granter_caps = adapter.list_capabilities_by_granter(&granter.public_key).unwrap();
        assert_eq!(granter_caps.len(), 3);
    }

    #[test]
    fn test_revocation() {
        let storage = create_test_storage();
        let adapter = AuthStorageAdapter::new(storage);

        let (granter, secret_key) = {
            let mined = mine_identity_sync(IdentityUserData::default(), 2);
            (mined.identity, mined.secret_key)
        };
        let grantee = {
            let mined = mine_identity_sync(IdentityUserData::default(), 2);
            adapter.store_identity(&mined.identity).unwrap();
            mined.identity.public_key
        };

        adapter.store_identity(&granter).unwrap();

        let cap = create_capability(
            &granter,
            &secret_key,
            &grantee,
            ResourcePattern::Exact("test:resource".to_string()),
            Permission::Read,
            None,
        )
        .unwrap();

        adapter.store_capability(&cap).unwrap();

        // Not revoked initially
        assert!(!adapter.is_capability_revoked(&cap.id).unwrap());

        // Create and store revocation
        let revocation =
            crate::auth::capability::create_revocation(&cap, &secret_key, None).unwrap();
        adapter.store_revocation(&revocation).unwrap();

        // Now revoked
        assert!(adapter.is_capability_revoked(&cap.id).unwrap());

        // Get revocation
        let retrieved = adapter.get_revocation(&cap.id).unwrap();
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_active_capabilities() {
        let storage = create_test_storage();
        let adapter = AuthStorageAdapter::new(storage);

        let (granter, secret_key) = {
            let mined = mine_identity_sync(IdentityUserData::default(), 2);
            (mined.identity, mined.secret_key)
        };
        let grantee = {
            let mined = mine_identity_sync(IdentityUserData::default(), 2);
            adapter.store_identity(&mined.identity).unwrap();
            mined.identity.public_key
        };

        adapter.store_identity(&granter).unwrap();

        // Create two capabilities
        let cap1 = create_capability(
            &granter,
            &secret_key,
            &grantee,
            ResourcePattern::Exact("test:resource1".to_string()),
            Permission::Read,
            None,
        )
        .unwrap();

        let cap2 = create_capability(
            &granter,
            &secret_key,
            &grantee,
            ResourcePattern::Exact("test:resource2".to_string()),
            Permission::Write,
            None,
        )
        .unwrap();

        adapter.store_capability(&cap1).unwrap();
        adapter.store_capability(&cap2).unwrap();

        // Both active
        let active = adapter.get_active_capabilities(&grantee).unwrap();
        assert_eq!(active.len(), 2);

        // Revoke one
        let revocation =
            crate::auth::capability::create_revocation(&cap1, &secret_key, None).unwrap();
        adapter.store_revocation(&revocation).unwrap();

        // Only one active
        let active = adapter.get_active_capabilities(&grantee).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, cap2.id);
    }
}
