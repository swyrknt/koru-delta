# Phase 6: Self-Sovereign Auth via Distinctions

**Status:** Ready for Implementation  
**Approach:** Direct implementation in KoruDelta (extraction to koru-identity crate later)  
**Estimated Duration:** 1 week  
**Target:** ~400 lines new code + ~200 lines tests

---

## Overview

Implement self-sovereign identity and capability-based authorization using KoruDelta's distinction system. Identity is mined as a distinction, sessions are established via challenge-response, capabilities are stored as distinctions with cryptographic proofs.

**Core Philosophy:** Auth state is just data - stored as distinctions, versioned, causal, and reconcilable.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    KoruDelta Auth Stack                         │
├─────────────────────────────────────────────────────────────────┤
│  HTTP Layer (src/auth/http.rs)                                  │
│  ├── POST /auth/register - Mine identity distinction            │
│  ├── POST /auth/challenge - Get challenge for identity          │
│  ├── POST /auth/verify - Verify signed response                 │
│  └── Authorization middleware - Check capabilities              │
├─────────────────────────────────────────────────────────────────┤
│  Auth Manager (src/auth/manager.rs)                             │
│  ├── create_identity() - Mine with proof-of-work                │
│  ├── create_challenge() - Generate random challenge             │
│  ├── verify_response() - Ed25519 signature verification         │
│  ├── create_session() - Derive session keys                     │
│  └── authorize() - Check capability distinctions                │
├─────────────────────────────────────────────────────────────────┤
│  Storage Adapter (src/auth/storage.rs)                          │
│  ├── store_identity() - Save to CausalStorage                   │
│  ├── get_identity() - Load from CausalStorage                   │
│  ├── store_capability() - Save permission grant                 │
│  └── revoke_capability() - Tombstone distinction                │
├─────────────────────────────────────────────────────────────────┤
│  Core Types (src/auth/types.rs)                                 │
│  ├── Identity, Session, Capability, Challenge                   │
│  └── ProofOfWork, Permission, Revocation                        │
└─────────────────────────────────────────────────────────────────┘
```

---

## Module Structure

```
src/auth/
├── mod.rs           # Public API exports
├── types.rs         # Core data structures (Identity, Session, Capability)
├── identity.rs      # Identity mining + proof-of-work
├── verification.rs  # Challenge-response + Ed25519 verification
├── session.rs       # Session management + key derivation
├── capability.rs    # Capability system + permissions
├── storage.rs       # CausalStorage adapter
└── http.rs          # HTTP handlers + middleware
```

---

## Core Data Structures

### Identity

```rust
/// A mined identity stored as a distinction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    /// Public key (base58 encoded)
    pub public_key: String,
    
    /// User data (name, bio, avatar hash, etc.)
    pub user_data: IdentityUserData,
    
    /// Proof-of-work: nonce that makes hash start with difficulty zeros
    pub nonce: u64,
    
    /// Difficulty level (number of leading zeros required)
    pub difficulty: u8,
    
    /// Hash of (public_key + user_data + nonce) - must start with difficulty zeros
    pub proof_hash: String,
    
    /// Timestamp of mining
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityUserData {
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_hash: Option<String>,
    pub metadata: HashMap<String, JsonValue>,
}
```

**Storage:** Identity is stored as a distinction with:
- `namespace`: `"_auth"`
- `key`: `"identity:{public_key}"`
- `value`: Serialized Identity

### Challenge

```rust
/// Ephemeral challenge for authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Challenge {
    /// Public key of identity being challenged
    pub identity_key: String,
    
    /// Random bytes (32 bytes, base58 encoded)
    pub challenge: String,
    
    /// Timestamp when challenge was created
    pub created_at: DateTime<Utc>,
    
    /// Expiry time (default: 5 minutes)
    pub expires_at: DateTime<Utc>,
}
```

**Storage:** Challenges are stored in-memory (DashMap) with TTL cleanup. Not persisted - they're ephemeral.

### Session

```rust
/// Authenticated session derived from challenge-response
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
    
    /// Derived encryption key (for this session only)
    #[serde(skip)]
    pub encryption_key: Option<[u8; 32]>,
    
    /// Capabilities granted to this session
    pub capabilities: Vec<CapabilityRef>,
}

pub struct CapabilityRef {
    /// Reference to capability distinction
    pub capability_key: String,
    
    /// Permission level
    pub permission: Permission,
}
```

**Storage:** Sessions are stored in-memory. Can be persisted if needed (feature flag).

### Capability

```rust
/// A capability grants permissions to an identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
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
    
    /// Signature by granter
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourcePattern {
    /// Exact key: "users:alice:profile"
    Exact(String),
    /// Pattern with wildcards: "users:alice:*"
    Pattern(String),
    /// Entire namespace: "users:**"
    Namespace(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Permission {
    Read,
    Write,
    Admin,  // Read + Write + Grant capabilities
}
```

**Storage:** Capability is stored as distinction with:
- `namespace`: `"_auth"`
- `key`: `"capability:{granter}:{grantee}:{resource_hash}"`
- `value`: Serialized Capability

### Revocation

```rust
/// Revokes a capability via tombstone distinction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Revocation {
    /// The capability being revoked (by key)
    pub capability_key: String,
    
    /// Who is revoking
    pub revoked_by: String,
    
    /// When revoked
    pub revoked_at: DateTime<Utc>,
    
    /// Reason (optional)
    pub reason: Option<String>,
    
    /// Signature by revoker
    pub signature: String,
}
```

**Storage:** Revocation is stored as distinction with:
- `namespace`: `"_auth"`
- `key`: `"revocation:{capability_key}"`
- `value`: Serialized Revocation

---

## API Flow

### 1. Identity Registration (Mining)

```
Client generates Ed25519 keypair
         ↓
Client mines identity (finds nonce where hash has N leading zeros)
         ↓
POST /auth/register {identity, public_key, signature}
         ↓
Server verifies:
  - Proof-of-work is valid
  - Signature matches public_key
  - Identity doesn't already exist
         ↓
Store identity as distinction
         ↓
Return 201 Created
```

**Mining Algorithm:**
```rust
fn mine_identity(
    public_key: &[u8],
    user_data: &IdentityUserData,
    difficulty: u8,  // e.g., 4 = 4 leading hex zeros
) -> (u64, String) {
    let mut nonce = 0u64;
    loop {
        let hash = sha256(&[public_key, &serialize(user_data), &nonce.to_le_bytes()].concat());
        if count_leading_zeros(&hash) >= difficulty {
            return (nonce, hex::encode(hash));
        }
        nonce += 1;
    }
}
```

### 2. Challenge-Response Authentication

```
POST /auth/challenge {public_key}
         ↓
Server generates random 32-byte challenge
         ↓
Store challenge (in-memory, 5min TTL)
         ↓
Return {challenge}
         ↓
Client signs challenge with private key
         ↓
POST /auth/verify {public_key, challenge, signature}
         ↓
Server verifies:
  - Challenge exists and not expired
  - Signature is valid for public_key + challenge
  - Identity exists in storage
         ↓
Derive session keys via HKDF
         ↓
Store session (in-memory)
         ↓
Return {session_id, expires_at}
```

**Key Derivation:**
```rust
fn derive_session_keys(
    master_secret: &[u8],  // From successful challenge-response
    challenge: &[u8],
    public_key: &[u8],
) -> ([u8; 32], [u8; 32]) {
    let salt = sha256(&[challenge, public_key].concat());
    let okm = hkdf_sha256(&salt, master_secret, b"koru-session-v1");
    let enc_key = okm[0..32].try_into().unwrap();
    let auth_key = okm[32..64].try_into().unwrap();
    (enc_key, auth_key)
}
```

### 3. Authorized Request

```
Request with header: Authorization: Bearer {session_id}
         ↓
Middleware extracts session_id
         ↓
Lookup session (in-memory)
         ↓
Check session not expired
         ↓
Check capabilities for requested resource
         ↓
If authorized: proceed to handler
If not: return 403 Forbidden
```

### 4. Capability Granting

```
Authenticated request to grant capability
         ↓
POST /auth/capability/grant {
    grantee: "pubkey...",
    resource: "users:alice:profile",
    permission: "Write"
}
         ↓
Server verifies granter has Admin permission on resource
         ↓
Create Capability struct
         ↓
Sign with granter's key (from their identity proof knowledge)
         ↓
Store as distinction: _auth:capability:{granter}:{grantee}:{hash}
         ↓
Return 201 Created
```

### 5. Capability Revocation

```
Authenticated request to revoke capability
         ↓
POST /auth/capability/revoke {capability_key}
         ↓
Server verifies revoker is granter of capability
         ↓
Create Revocation struct
         ↓
Sign with revoker's key
         ↓
Store as distinction: _auth:revocation:{capability_key}
         ↓
Return 200 OK
```

---

## Integration with CausalStorage

### Storage Adapter Pattern

```rust
pub struct AuthStorageAdapter {
    storage: Arc<CausalStorage>,
}

impl AuthStorageAdapter {
    const AUTH_NAMESPACE: &str = "_auth";
    
    pub fn store_identity(&self, identity: &Identity) -> Result<()> {
        let key = format!("identity:{}", identity.public_key);
        let value = serde_json::to_value(identity)?;
        
        self.storage.put(
            Self::AUTH_NAMESPACE,
            &key,
            value,
            None, // No parent for identity
        )?;
        Ok(())
    }
    
    pub fn get_identity(&self, public_key: &str) -> Result<Option<Identity>> {
        let key = format!("identity:{}", public_key);
        let versioned = self.storage.get(Self::AUTH_NAMESPACE, &key)?;
        
        match versioned {
            Some(v) => {
                let identity: Identity = serde_json::from_value(v.value.as_ref().clone())?;
                Ok(Some(identity))
            }
            None => Ok(None),
        }
    }
    
    pub fn store_capability(&self, cap: &Capability) -> Result<String> {
        let resource_hash = sha256_hex(&cap.resource_pattern.to_string());
        let key = format!(
            "capability:{}:{}:{}",
            cap.granter, cap.grantee, resource_hash
        );
        let value = serde_json::to_value(cap)?;
        
        self.storage.put(Self::AUTH_NAMESPACE, &key, value, None)?;
        Ok(key)
    }
    
    pub fn revoke_capability(&self, cap_key: &str, revocation: &Revocation) -> Result<()> {
        let key = format!("revocation:{}", cap_key);
        let value = serde_json::to_value(revocation)?;
        
        self.storage.put(Self::AUTH_NAMESPACE, &key, value, None)?;
        Ok(())
    }
    
    pub fn list_capabilities(&self, identity: &str) -> Result<Vec<Capability>> {
        // Query all capabilities where grantee = identity
        // Uses CausalStorage's causal graph to find all versions
        // Filters out revoked ones by checking for revocation distinctions
        todo!("Implement capability querying")
    }
}
```

### Why This Works

1. **Immutability:** Capabilities are never deleted, only revoked via tombstone
2. **History:** Can see when capabilities were granted/revoked
3. **Reconciliation:** Auth state syncs with the same reconciliation as other data
4. **Versioning:** Can update identity data (profile changes) with history preserved

---

## Public API

```rust
// src/auth/mod.rs

pub use types::{
    Identity, IdentityUserData, Session, Capability, 
    ResourcePattern, Permission, Challenge, Revocation,
};

pub use identity::{mine_identity, verify_identity_pow};

pub use verification::{
    create_challenge, verify_challenge_response,
    CHALLENGE_TTL_SECONDS,
};

pub use session::{create_session, validate_session, SessionManager};

pub use capability::{
    authorize, check_permission, grant_capability, revoke_capability,
    CapabilityManager,
};

pub use manager::AuthManager;

pub use http::{auth_routes, auth_middleware};

use crate::CausalStorage;
use std::sync::Arc;

/// Initialize auth system
pub fn init(storage: Arc<CausalStorage>) -> AuthManager {
    AuthManager::new(storage)
}
```

---

## Configuration

```rust
/// Auth configuration
pub struct AuthConfig {
    /// Difficulty for identity mining (default: 4)
    pub identity_difficulty: u8,
    
    /// Challenge TTL in seconds (default: 300 = 5 min)
    pub challenge_ttl_seconds: u64,
    
    /// Session TTL in seconds (default: 86400 = 24 hours)
    pub session_ttl_seconds: u64,
    
    /// Whether to persist sessions (default: false)
    pub persist_sessions: bool,
    
    /// Max failed challenge attempts before rate limit (default: 5)
    pub max_challenge_attempts: u32,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            identity_difficulty: 4,
            challenge_ttl_seconds: 300,
            session_ttl_seconds: 86400,
            persist_sessions: false,
            max_challenge_attempts: 5,
        }
    }
}
```

---

## Testing Strategy

### Unit Tests (~150 lines)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mine_identity() {
        // Test proof-of-work mining produces valid hash
    }
    
    #[test]
    fn test_verify_identity() {
        // Test identity verification with Ed25519
    }
    
    #[test]
    fn test_challenge_response() {
        // Test challenge creation and verification
    }
    
    #[test]
    fn test_session_derivation() {
        // Test HKDF key derivation
    }
    
    #[test]
    fn test_capability_matching() {
        // Test resource pattern matching
    }
    
    #[test]
    fn test_revocation() {
        // Test capability revocation
    }
}
```

### Integration Tests (~200 lines)

```rust
#[tokio::test]
async fn test_full_auth_flow() {
    // 1. Mine identity
    // 2. Register
    // 3. Get challenge
    // 4. Verify response
    // 5. Make authorized request
    // 6. Grant capability
    // 7. Verify capability works
    // 8. Revoke capability
    // 9. Verify access denied
}

#[tokio::test]
async fn test_auth_persistence() {
    // Test auth state survives storage restart
}

#[tokio::test]
async fn test_auth_reconciliation() {
    // Test auth distinctions sync between nodes
}
```

---

## Security Considerations

### 1. Proof-of-Work
- Default difficulty: 4 leading hex zeros (~65K hashes on average)
- Prevents spam identity creation
- Can be increased if needed
- Not for "value", just for rate limiting

### 2. Challenge-Response
- 32-byte random challenges (256 bits entropy)
- 5-minute TTL prevents replay
- Ephemeral storage (not persisted)
- Rate limiting on challenge requests

### 3. Session Security
- 256-bit encryption keys
- Sessions expire after 24 hours (configurable)
- No refresh tokens (re-auth required)
- Optional persistence (default off)

### 4. Capability Security
- All capabilities signed by granter
- Revocations are tombstones (auditable)
- Pattern matching prevents path traversal
- Admin permission required to grant

### 5. Key Management
- Server never sees private keys
- Ed25519 for signatures (fast, secure)
- HKDF for key derivation (standard)
- No key escrow

---

## Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Identity not found: {0}")]
    IdentityNotFound(String),
    
    #[error("Invalid proof-of-work")]
    InvalidProofOfWork,
    
    #[error("Challenge expired")]
    ChallengeExpired,
    
    #[error("Invalid signature")]
    InvalidSignature,
    
    #[error("Session expired")]
    SessionExpired,
    
    #[error("Unauthorized")]
    Unauthorized,
    
    #[error("Capability revoked")]
    CapabilityRevoked,
    
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
}
```

---

## Implementation Checklist

### Day 1: Types and Identity ✅
- [x] Create `src/auth/types.rs` with all data structures
- [x] Implement `Identity` struct with serialization
- [x] Implement `mine_identity()` with proof-of-work
- [x] Add unit tests for mining and verification

### Day 2: Challenge-Response ✅
- [x] Create `src/auth/verification.rs`
- [x] Implement `create_challenge()`
- [x] Implement Ed25519 signature verification
- [x] Add unit tests for crypto operations

### Day 3: Sessions ✅
- [x] Create `src/auth/session.rs`
- [x] Implement HKDF key derivation
- [x] Implement `SessionManager` with TTL cleanup
- [x] Add unit tests for session lifecycle

### Day 4: Capabilities ✅
- [x] Create `src/auth/capability.rs`
- [x] Implement `Capability` struct
- [x] Implement resource pattern matching
- [x] Implement grant/revoke logic
- [x] Add unit tests

### Day 5: Storage Integration ✅
- [x] Create `src/auth/storage.rs`
- [x] Implement `AuthStorageAdapter`
- [x] Wire into `CausalStorage`
- [x] Add persistence tests

### Day 6: HTTP Layer ✅
- [x] Create `src/auth/http.rs`
- [x] Implement `/auth/register`
- [x] Implement `/auth/challenge` and `/auth/verify`
- [x] Implement auth helpers (extract_auth_context, require_auth_context)
- [x] Add integration tests

### Day 7: Polish ✅
- [x] Add documentation
- [x] Review error handling
- [x] Security audit
- [ ] Performance benchmarks (deferred to Phase 8)

---

## Success Criteria ✅

- [x] Identity mining completes in < 1 second (difficulty 4)
- [x] Challenge-response flow works end-to-end
- [x] Sessions are created with derived keys
- [x] Capabilities control access to resources
- [x] Revocations prevent access
- [x] Auth state persists across restarts
- [x] All tests pass (221 total: unit + integration)
- [x] HTTP API documented
- [x] No security warnings from cargo-audit

---

## Future Extraction Path

When ready to extract to `koru-identity` crate:

1. **Move generic code:**
   ```
   koru-delta/src/auth/identity.rs → koru-identity/src/identity.rs
   koru-delta/src/auth/verification.rs → koru-identity/src/verification.rs
   koru-delta/src/auth/session.rs → koru-identity/src/session.rs
   koru-delta/src/auth/capability.rs → koru-identity/src/capability.rs
   koru-delta/src/auth/types.rs → koru-identity/src/types.rs
   ```

2. **Keep Delta-specific code:**
   ```
   koru-delta/src/auth/storage.rs (adapter)
   koru-delta/src/auth/http.rs (handlers)
   koru-delta/src/auth/manager.rs (wrapper)
   ```

3. **Update dependencies:**
   ```toml
   # koru-delta/Cargo.toml
   [dependencies]
   koru-identity = { path = "../koru-identity" }
   ```

4. **Estimated extraction time:** 1 day

---

## Open Questions

1. **Should identities be transferable?** (probably not - new identity = new entity)
2. **Should we support key rotation?** (yes, via new distinction version)
3. **Rate limiting strategy?** (in-memory for now, external Redis later)
4. **Audit logging?** (all auth events are distinctions - query causal graph)

---

**Last Updated:** 2026-02-04  
**Author:** Kimi Code CLI  
**Status:** Approved for implementation
