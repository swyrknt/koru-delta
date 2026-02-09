//! Capability-based authorization.
//!
//! Capabilities are grants of permission from one identity to another.
//! They are stored as distinctions and can be revoked via tombstone.

#[allow(unused_imports)]
use chrono::{DateTime, Duration, Utc};
use sha2::{Digest, Sha256};

use crate::auth::identity::sign_message;
use crate::auth::types::{
    AuthError, Capability, CapabilityRef, Identity, Permission, ResourcePattern, Revocation,
};

/// Default capability TTL: None (no expiration).
#[allow(dead_code)]
pub const DEFAULT_CAPABILITY_TTL_DAYS: i64 = 365;

/// Create a capability grant.
///
/// # Arguments
/// * `granter` - The identity granting the capability (must have secret key)
/// * `grantee` - The public key of the identity receiving the capability
/// * `resource_pattern` - What resources this applies to
/// * `permission` - The level of access granted
/// * `expires_at` - Optional expiration time
///
/// # Returns
/// The signed capability.
pub fn create_capability(
    granter_identity: &Identity,
    granter_secret_key: &[u8],
    grantee: &str,
    resource_pattern: ResourcePattern,
    permission: Permission,
    expires_at: Option<DateTime<Utc>>,
) -> Result<Capability, AuthError> {
    let id = generate_capability_id(granter_identity, grantee, &resource_pattern);
    let created_at = Utc::now();

    // Create unsigned capability for signing
    let capability_to_sign = Capability {
        id: id.clone(),
        granter: granter_identity.public_key.clone(),
        grantee: grantee.to_string(),
        resource_pattern: resource_pattern.clone(),
        permission,
        created_at,
        expires_at,
        signature: String::new(),
    };

    // Sign
    let message = create_capability_signature_message(&capability_to_sign);
    let signature = sign_message(granter_secret_key, &message)?;

    Ok(Capability {
        id,
        granter: granter_identity.public_key.clone(),
        grantee: grantee.to_string(),
        resource_pattern,
        permission,
        created_at,
        expires_at,
        signature: bs58::encode(&signature).into_string(),
    })
}

/// Generate a unique ID for a capability.
fn generate_capability_id(
    granter: &Identity,
    grantee: &str,
    resource_pattern: &ResourcePattern,
) -> String {
    let input = format!(
        "{}:{}:{}:{}",
        granter.public_key,
        grantee,
        resource_pattern,
        Utc::now().timestamp_nanos_opt().unwrap_or(0)
    );
    let hash = Sha256::digest(input.as_bytes());
    bs58::encode(&hash[..16]).into_string() // First 16 bytes = 128 bits
}

/// Create the message to sign for a capability.
fn create_capability_signature_message(cap: &Capability) -> Vec<u8> {
    format!(
        "capability_grant:{}/{}->{}/{}/{}/{}",
        cap.id,
        cap.granter,
        cap.grantee,
        cap.resource_pattern,
        cap.permission.as_str(),
        cap.created_at.timestamp()
    )
    .into_bytes()
}

/// Create a revocation for a capability.
///
/// # Arguments
/// * `capability` - The capability to revoke
/// * `revoker_secret_key` - Secret key of the revoker (must be granter)
/// * `reason` - Optional reason for revocation
///
/// # Returns
/// The signed revocation.
pub fn create_revocation(
    capability: &Capability,
    revoker_secret_key: &[u8],
    reason: Option<String>,
) -> Result<Revocation, AuthError> {
    let revoked_at = Utc::now();

    // Create message to sign
    let message = format!(
        "capability_revoke:{}/{}/{}",
        capability.id,
        capability.granter,
        revoked_at.timestamp()
    );

    // Sign
    let signature = sign_message(revoker_secret_key, message.as_bytes())?;

    Ok(Revocation {
        capability_id: capability.id.clone(),
        revoked_by: capability.granter.clone(),
        revoked_at,
        reason,
        signature: bs58::encode(&signature).into_string(),
    })
}

/// Check if a capability is revoked.
pub fn is_revoked(_capability: &Capability, revocations: &[Revocation]) -> bool {
    revocations
        .iter()
        .any(|r| r.capability_id == _capability.id)
}

/// Check authorization for a resource.
///
/// # Arguments
/// * `identity_key` - The identity attempting access
/// * `namespace` - The resource namespace
/// * `key` - The resource key
/// * `required_permission` - The permission required
/// * `capabilities` - List of capabilities to check
/// * `revocations` - List of active revocations
///
/// # Returns
/// `Ok(())` if authorized, `Err(AuthError)` otherwise.
pub fn authorize(
    identity_key: &str,
    namespace: &str,
    key: &str,
    required_permission: Permission,
    capabilities: &[Capability],
    revocations: &[Revocation],
) -> Result<CapabilityRef, AuthError> {
    // Find matching capabilities
    for cap in capabilities {
        // Skip if not for this identity
        if cap.grantee != identity_key {
            continue;
        }

        // Skip if revoked
        if is_revoked(cap, revocations) {
            continue;
        }

        // Skip if expired
        if cap.is_expired() {
            continue;
        }

        // Skip if permission insufficient
        if !cap.permission.includes(required_permission) {
            continue;
        }

        // Check resource pattern
        if cap.resource_pattern.matches(namespace, key) {
            return Ok(CapabilityRef {
                capability_key: format!("_auth:capability:{}", cap.id),
                resource_pattern: cap.resource_pattern.clone(),
                permission: cap.permission,
            });
        }
    }

    Err(AuthError::Unauthorized)
}

/// Check if an identity has a specific permission on a resource.
pub fn check_permission(
    identity_key: &str,
    namespace: &str,
    key: &str,
    permission: Permission,
    capabilities: &[Capability],
    revocations: &[Revocation],
) -> bool {
    authorize(
        identity_key,
        namespace,
        key,
        permission,
        capabilities,
        revocations,
    )
    .is_ok()
}

/// Build a capability reference for storage.
#[allow(dead_code)]
pub fn build_capability_ref(capability: &Capability) -> CapabilityRef {
    CapabilityRef {
        capability_key: format!("_auth:capability:{}", capability.id),
        resource_pattern: capability.resource_pattern.clone(),
        permission: capability.permission,
    }
}

/// Get the storage key for a capability.
#[allow(dead_code)]
pub fn capability_storage_key(capability: &Capability) -> String {
    format!("capability:{}", capability.id)
}

/// Get the storage key for a revocation.
#[allow(dead_code)]
pub fn revocation_storage_key(capability_id: &str) -> String {
    format!("revocation:{}", capability_id)
}

/// Capability manager for caching and querying.
pub struct CapabilityManager {
    /// Cached capabilities (loaded from storage)
    capabilities: Vec<Capability>,
    /// Cached revocations
    revocations: Vec<Revocation>,
}

impl CapabilityManager {
    /// Create a new capability manager.
    pub fn new() -> Self {
        Self {
            capabilities: Vec::new(),
            revocations: Vec::new(),
        }
    }

    /// Load capabilities from storage.
    pub fn load(&mut self, capabilities: Vec<Capability>, revocations: Vec<Revocation>) {
        self.capabilities = capabilities;
        self.revocations = revocations;
    }

    /// Add a capability.
    pub fn add_capability(&mut self, capability: Capability) {
        self.capabilities.push(capability);
    }

    /// Add a revocation.
    pub fn add_revocation(&mut self, revocation: Revocation) {
        self.revocations.push(revocation);
    }

    /// Authorize access to a resource.
    pub fn authorize(
        &self,
        identity_key: &str,
        namespace: &str,
        key: &str,
        required_permission: Permission,
    ) -> Result<CapabilityRef, AuthError> {
        authorize(
            identity_key,
            namespace,
            key,
            required_permission,
            &self.capabilities,
            &self.revocations,
        )
    }

    /// Check if an identity has a permission.
    pub fn check_permission(
        &self,
        identity_key: &str,
        namespace: &str,
        key: &str,
        permission: Permission,
    ) -> bool {
        check_permission(
            identity_key,
            namespace,
            key,
            permission,
            &self.capabilities,
            &self.revocations,
        )
    }

    /// Get all capabilities for an identity.
    pub fn get_identity_capabilities(&self, identity_key: &str) -> Vec<&Capability> {
        self.capabilities
            .iter()
            .filter(|c| c.grantee == identity_key && !is_revoked(c, &self.revocations))
            .collect()
    }

    /// Get all capabilities granted by an identity.
    pub fn get_granted_capabilities(&self, granter_key: &str) -> Vec<&Capability> {
        self.capabilities
            .iter()
            .filter(|c| c.granter == granter_key && !is_revoked(c, &self.revocations))
            .collect()
    }
}

impl Default for CapabilityManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::identity::mine_identity_sync;
    use crate::auth::types::IdentityUserData;

    fn create_test_granter() -> (Identity, Vec<u8>) {
        let mined = mine_identity_sync(IdentityUserData::default(), 2);
        (mined.identity, mined.secret_key)
    }

    fn create_test_grantee() -> String {
        let mined = mine_identity_sync(IdentityUserData::default(), 2);
        mined.identity.public_key
    }

    #[test]
    fn test_create_capability() {
        let (granter, secret_key) = create_test_granter();
        let grantee = create_test_grantee();

        let cap = create_capability(
            &granter,
            &secret_key,
            &grantee,
            ResourcePattern::Exact("test:resource".to_string()),
            Permission::Read,
            None,
        )
        .unwrap();

        assert_eq!(cap.granter, granter.public_key);
        assert_eq!(cap.grantee, grantee);
        assert_eq!(cap.permission, Permission::Read);
        assert!(!cap.signature.is_empty());

        // Verify signature
        assert!(cap.verify_signature().unwrap());
    }

    #[test]
    fn test_authorize_exact_match() {
        let (granter, secret_key) = create_test_granter();
        let grantee = create_test_grantee();

        let cap = create_capability(
            &granter,
            &secret_key,
            &grantee,
            ResourcePattern::Exact("test:resource".to_string()),
            Permission::Read,
            None,
        )
        .unwrap();

        // Authorized
        let result = authorize(
            &grantee,
            "test",
            "resource",
            Permission::Read,
            std::slice::from_ref(&cap),
            &[],
        );
        assert!(result.is_ok());

        // Wrong permission
        let result = authorize(
            &grantee,
            "test",
            "resource",
            Permission::Write,
            std::slice::from_ref(&cap),
            &[],
        );
        assert!(matches!(result, Err(AuthError::Unauthorized)));

        // Wrong resource
        let result = authorize(
            &grantee,
            "test",
            "other",
            Permission::Read,
            std::slice::from_ref(&cap),
            &[],
        );
        assert!(matches!(result, Err(AuthError::Unauthorized)));
    }

    #[test]
    fn test_authorize_with_admin() {
        let (granter, secret_key) = create_test_granter();
        let grantee = create_test_grantee();

        let cap = create_capability(
            &granter,
            &secret_key,
            &grantee,
            ResourcePattern::Namespace("test".to_string()),
            Permission::Admin,
            None,
        )
        .unwrap();

        // Admin includes Read, Write, Admin
        assert!(authorize(
            &grantee,
            "test",
            "anything",
            Permission::Read,
            std::slice::from_ref(&cap),
            &[]
        )
        .is_ok());
        assert!(authorize(
            &grantee,
            "test",
            "anything",
            Permission::Write,
            std::slice::from_ref(&cap),
            &[]
        )
        .is_ok());
        assert!(authorize(
            &grantee,
            "test",
            "anything",
            Permission::Admin,
            std::slice::from_ref(&cap),
            &[]
        )
        .is_ok());
    }

    #[test]
    fn test_revocation() {
        let (granter, secret_key) = create_test_granter();
        let grantee = create_test_grantee();

        let cap = create_capability(
            &granter,
            &secret_key,
            &grantee,
            ResourcePattern::Exact("test:resource".to_string()),
            Permission::Read,
            None,
        )
        .unwrap();

        // Initially authorized
        assert!(authorize(
            &grantee,
            "test",
            "resource",
            Permission::Read,
            std::slice::from_ref(&cap),
            &[]
        )
        .is_ok());

        // Create revocation
        let revocation = create_revocation(&cap, &secret_key, Some("Testing".to_string())).unwrap();

        // Now unauthorized
        assert!(matches!(
            authorize(
                &grantee,
                "test",
                "resource",
                Permission::Read,
                std::slice::from_ref(&cap),
                &[revocation]
            ),
            Err(AuthError::Unauthorized)
        ));
    }

    #[test]
    fn test_capability_expiration() {
        let (granter, secret_key) = create_test_granter();
        let grantee = create_test_grantee();

        // Create expired capability
        let expired_time = Utc::now() - Duration::hours(1);
        let cap = create_capability(
            &granter,
            &secret_key,
            &grantee,
            ResourcePattern::Exact("test:resource".to_string()),
            Permission::Read,
            Some(expired_time),
        )
        .unwrap();

        // Should be expired
        assert!(cap.is_expired());

        // Authorization should fail
        let result = authorize(&grantee, "test", "resource", Permission::Read, &[cap], &[]);
        assert!(matches!(result, Err(AuthError::Unauthorized)));
    }

    #[test]
    fn test_capability_manager() {
        let (granter, secret_key) = create_test_granter();
        let grantee = create_test_grantee();

        let cap = create_capability(
            &granter,
            &secret_key,
            &grantee,
            ResourcePattern::Namespace("test".to_string()),
            Permission::Write,
            None,
        )
        .unwrap();

        let mut manager = CapabilityManager::new();
        manager.add_capability(cap.clone());

        // Check permission
        assert!(manager.check_permission(&grantee, "test", "resource", Permission::Read));
        assert!(manager.check_permission(&grantee, "test", "resource", Permission::Write));
        assert!(!manager.check_permission(&grantee, "test", "resource", Permission::Admin));

        // Wrong identity
        assert!(!manager.check_permission("other_identity", "test", "resource", Permission::Read));

        // Get capabilities
        let caps = manager.get_identity_capabilities(&grantee);
        assert_eq!(caps.len(), 1);

        // Revoke
        let revocation = create_revocation(&cap, &secret_key, None).unwrap();
        manager.add_revocation(revocation);

        // Now unauthorized
        assert!(!manager.check_permission(&grantee, "test", "resource", Permission::Read));
    }

    #[test]
    fn test_wildcard_pattern() {
        let (granter, secret_key) = create_test_granter();
        let grantee = create_test_grantee();

        let cap = create_capability(
            &granter,
            &secret_key,
            &grantee,
            ResourcePattern::Wildcard {
                prefix: "users:alice:".to_string(),
            },
            Permission::Read,
            None,
        )
        .unwrap();

        assert!(authorize(
            &grantee,
            "users",
            "alice:profile",
            Permission::Read,
            std::slice::from_ref(&cap),
            &[]
        )
        .is_ok());
        assert!(authorize(
            &grantee,
            "users",
            "alice:settings",
            Permission::Read,
            std::slice::from_ref(&cap),
            &[]
        )
        .is_ok());
        assert!(matches!(
            authorize(
                &grantee,
                "users",
                "bob:profile",
                Permission::Read,
                std::slice::from_ref(&cap),
                &[]
            ),
            Err(AuthError::Unauthorized)
        ));
    }
}
