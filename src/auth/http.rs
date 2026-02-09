//! HTTP API for authentication.
//!
//! This module provides HTTP endpoints for the auth system:
//!
//! ## Endpoints
//!
//! ### Identity Management
//! - `POST /api/v1/auth/register` - Register a new identity
//!
//! ### Authentication
//! - `POST /api/v1/auth/challenge` - Request a challenge
//! - `POST /api/v1/auth/verify` - Verify challenge response and create session
//!
//! ### Session Management
//! - `POST /api/v1/auth/session/validate` - Validate a session
//! - `POST /api/v1/auth/session/revoke` - Revoke a session
//!
//! ### Capabilities
//! - `POST /api/v1/auth/capability/grant` - Grant a capability
//! - `POST /api/v1/auth/capability/revoke` - Revoke a capability
//! - `GET /api/v1/auth/capabilities` - List capabilities for current identity
//!
//! ### Protected Data Operations
//! All existing endpoints can be protected by adding the auth middleware.
//!
//! # Example
//!
//! ```rust,ignore
//! use koru_delta::auth::http::{auth_routes, auth_middleware};
//! use koru_delta::auth::AuthManager;
//!
//! let auth_manager = AuthManager::new(storage);
//! let app = Router::new()
//!     .merge(auth_routes(auth_manager.clone()))
//!     .route_layer(auth_middleware(auth_manager));
//! ```

use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::auth::manager::AuthManager;
use crate::auth::types::{
    AuthError, Capability, Identity, IdentityUserData, Permission, ResourcePattern, Session,
};

/// Extension trait for extracting identity from request extensions.
#[derive(Clone)]
pub struct AuthContext {
    /// The authenticated identity (if any)
    pub identity: Option<Identity>,
    /// The session (if authenticated)
    pub session: Option<Session>,
}

impl AuthContext {
    /// Create an empty auth context (unauthenticated).
    pub fn unauthenticated() -> Self {
        Self {
            identity: None,
            session: None,
        }
    }

    /// Create an authenticated context.
    pub fn authenticated(identity: Identity, session: Session) -> Self {
        Self {
            identity: Some(identity),
            session: Some(session),
        }
    }

    /// Check if the request is authenticated.
    pub fn is_authenticated(&self) -> bool {
        self.identity.is_some() && self.session.is_some()
    }

    /// Get the identity public key.
    pub fn identity_key(&self) -> Option<&str> {
        self.identity.as_ref().map(|i| i.public_key.as_str())
    }

    /// Require authentication, returning 401 if not authenticated.
    pub fn require_auth(&self) -> Result<(&Identity, &Session), StatusCode> {
        match (&self.identity, &self.session) {
            (Some(identity), Some(session)) => Ok((identity, session)),
            _ => Err(StatusCode::UNAUTHORIZED),
        }
    }
}

// ============================================================================
// Request/Response Types
// ============================================================================

/// Request to register a new identity.
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    /// User data for the identity
    #[serde(flatten)]
    pub user_data: IdentityUserData,
    /// Pre-mined identity (optional - if not provided, server will mine)
    pub identity: Option<Identity>,
}

/// Response for successful registration.
#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    /// The registered identity
    pub identity: Identity,
    /// Message for the user
    pub message: String,
}

/// Request to get a challenge.
#[derive(Debug, Deserialize)]
pub struct ChallengeRequest {
    /// Public key of the identity
    pub public_key: String,
}

/// Response with challenge.
#[derive(Debug, Serialize)]
pub struct ChallengeResponse {
    /// The challenge string (base58 encoded)
    pub challenge: String,
    /// Expiry time
    pub expires_at: String,
}

/// Request to verify challenge and create session.
#[derive(Debug, Deserialize)]
pub struct VerifyRequest {
    /// Public key
    pub public_key: String,
    /// Challenge string
    pub challenge: String,
    /// Signed response (base58 encoded signature)
    pub response: String,
}

/// Response with session.
#[derive(Debug, Serialize)]
pub struct SessionResponse {
    /// Session ID
    pub session_id: String,
    /// Identity public key
    pub identity_key: String,
    /// Expiry time
    pub expires_at: String,
}

/// Request to validate a session.
#[derive(Debug, Deserialize)]
pub struct ValidateSessionRequest {
    /// Session ID
    pub session_id: String,
}

/// Response for session validation.
#[derive(Debug, Serialize)]
pub struct ValidateSessionResponse {
    /// Whether the session is valid
    pub valid: bool,
    /// Session info (if valid)
    pub session: Option<SessionInfo>,
}

/// Session info for responses.
#[derive(Debug, Serialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub identity_key: String,
    pub expires_at: String,
}

/// Request to grant a capability.
#[derive(Debug, Deserialize)]
pub struct GrantCapabilityRequest {
    /// Grantee public key
    pub grantee: String,
    /// Resource pattern (exact, wildcard, or namespace)
    pub resource: String,
    /// Permission level
    pub permission: Permission,
    /// Optional expiry (seconds from now)
    pub expires_in_seconds: Option<i64>,
}

/// Response for capability grant.
#[derive(Debug, Serialize)]
pub struct CapabilityResponse {
    /// The granted capability
    pub capability: Capability,
    /// Storage key
    pub key: String,
}

/// Request to revoke a capability.
#[derive(Debug, Deserialize)]
pub struct RevokeCapabilityRequest {
    /// Capability ID to revoke
    pub capability_id: String,
    /// Optional reason
    pub reason: Option<String>,
}

/// Request to check authorization.
#[derive(Debug, Deserialize)]
pub struct AuthorizeRequest {
    /// Namespace to check
    pub namespace: String,
    /// Key to check
    pub key: String,
    /// Required permission
    pub permission: Permission,
}

/// Response for authorization check.
#[derive(Debug, Serialize)]
pub struct AuthorizeResponse {
    /// Whether authorized
    pub authorized: bool,
    /// Capability that grants access (if authorized)
    pub capability: Option<CapabilityInfo>,
}

/// Capability info for responses.
#[derive(Debug, Serialize)]
pub struct CapabilityInfo {
    pub id: String,
    pub resource: String,
    pub permission: String,
}

/// Error response.
#[derive(Debug, Serialize)]
pub struct AuthErrorResponse {
    pub error: String,
    pub code: String,
}

// ============================================================================
// Routes
// ============================================================================

/// Create auth routes.
pub fn auth_routes(auth: Arc<AuthManager>) -> Router {
    Router::new()
        // Identity management
        .route("/api/v1/auth/register", post(handle_register))
        // Authentication
        .route("/api/v1/auth/challenge", post(handle_challenge))
        .route("/api/v1/auth/verify", post(handle_verify))
        // Session management
        .route(
            "/api/v1/auth/session/validate",
            post(handle_validate_session),
        )
        // Capabilities
        .route("/api/v1/auth/capabilities", get(handle_list_capabilities))
        .with_state(auth)
}

/// Create protected routes that require authentication.
pub fn protected_routes(auth: Arc<AuthManager>) -> Router {
    Router::new()
        // Session management
        .route("/api/v1/auth/session/revoke", post(handle_revoke_session))
        // Capabilities
        .route(
            "/api/v1/auth/capability/grant",
            post(handle_grant_capability),
        )
        .route(
            "/api/v1/auth/capability/revoke",
            post(handle_revoke_capability),
        )
        .route("/api/v1/auth/authorize", post(handle_authorize))
        .with_state(auth)
}

// ============================================================================
// Middleware (axum 0.7 compatible)
// ============================================================================

/// Layer function that creates an auth middleware layer.
///
/// Use with `.layer()` on routes that need authentication.
///
/// # Example
/// ```rust,ignore
/// let app = Router::new()
///     .route("/protected", get(protected_handler))
///     .layer(auth_layer(auth_manager));
/// ```
pub fn auth_layer(auth: Arc<AuthManager>) -> axum::Extension<Arc<AuthManager>> {
    axum::Extension(auth)
}

/// Extract auth context from headers in a handler.
///
/// # Example
/// ```rust,ignore
/// async fn handler(
///     headers: axum::http::HeaderMap,
///     State(auth): State<Arc<AuthManager>>,
/// ) -> Result<Response, StatusCode> {
///     let ctx = extract_auth_context(&headers, &auth).await?;
///     // Use ctx...
/// }
/// ```
pub async fn extract_auth_context(
    headers: &axum::http::HeaderMap,
    auth: &AuthManager,
) -> Result<AuthContext, StatusCode> {
    let session_id = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "));

    match session_id {
        Some(session_id) => {
            let session = auth
                .validate_session(session_id)
                .map_err(|_| StatusCode::UNAUTHORIZED)?;
            let identity = auth
                .get_identity(&session.identity_key)
                .map_err(|_| StatusCode::UNAUTHORIZED)?
                .ok_or(StatusCode::UNAUTHORIZED)?;
            Ok(AuthContext::authenticated(identity, session))
        }
        None => Ok(AuthContext::unauthenticated()),
    }
}

/// Require authentication from headers.
pub async fn require_auth_context(
    headers: &axum::http::HeaderMap,
    auth: &AuthManager,
) -> Result<(Identity, Session), StatusCode> {
    let session_id = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let session = auth
        .validate_session(session_id)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    let identity = auth
        .get_identity(&session.identity_key)
        .map_err(|_| StatusCode::UNAUTHORIZED)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    Ok((identity, session))
}

// ============================================================================
// Handlers
// ============================================================================

/// Handle identity registration.
async fn handle_register(
    State(auth): State<Arc<AuthManager>>,
    Json(request): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    // For now, we mine the identity server-side
    // In production, client should mine and just submit the proof
    let (identity, _secret_key) = auth
        .create_identity(request.user_data)
        .map_err(auth_error)?;

    Ok(Json(RegisterResponse {
        identity,
        message: "Identity registered successfully. Store your secret key securely.".to_string(),
    }))
}

/// Handle challenge request.
async fn handle_challenge(
    State(auth): State<Arc<AuthManager>>,
    Json(request): Json<ChallengeRequest>,
) -> Result<Json<ChallengeResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    let challenge = auth
        .create_challenge(&request.public_key)
        .map_err(auth_error)?;

    Ok(Json(ChallengeResponse {
        challenge: challenge.clone(),
        expires_at: (chrono::Utc::now() + chrono::Duration::seconds(300)).to_rfc3339(),
    }))
}

/// Handle challenge verification and session creation.
async fn handle_verify(
    State(auth): State<Arc<AuthManager>>,
    Json(request): Json<VerifyRequest>,
) -> Result<Json<SessionResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    let session = auth
        .verify_and_create_session(&request.public_key, &request.challenge, &request.response)
        .map_err(auth_error)?;

    Ok(Json(SessionResponse {
        session_id: session.session_id,
        identity_key: session.identity_key,
        expires_at: session.expires_at.to_rfc3339(),
    }))
}

/// Handle session validation.
async fn handle_validate_session(
    State(auth): State<Arc<AuthManager>>,
    Json(request): Json<ValidateSessionRequest>,
) -> Json<ValidateSessionResponse> {
    match auth.validate_session(&request.session_id) {
        Ok(session) => Json(ValidateSessionResponse {
            valid: true,
            session: Some(SessionInfo {
                session_id: session.session_id,
                identity_key: session.identity_key,
                expires_at: session.expires_at.to_rfc3339(),
            }),
        }),
        Err(_) => Json(ValidateSessionResponse {
            valid: false,
            session: None,
        }),
    }
}

/// Handle session revocation.
async fn handle_revoke_session(
    State(auth): State<Arc<AuthManager>>,
    headers: axum::http::HeaderMap,
    Json(request): Json<ValidateSessionRequest>,
) -> Result<StatusCode, (StatusCode, Json<AuthErrorResponse>)> {
    // Require authentication
    let _ = require_auth_context(&headers, &auth).await.map_err(|e| {
        (
            e,
            Json(AuthErrorResponse {
                error: "Unauthorized".to_string(),
                code: "UNAUTHORIZED".to_string(),
            }),
        )
    });

    // Revoke the session
    match auth.revoke_session(&request.session_id) {
        Ok(()) => Ok(StatusCode::OK),
        Err(e) => Err(auth_error(e)),
    }
}

/// Handle capability grant.
async fn handle_grant_capability(
    State(auth): State<Arc<AuthManager>>,
    headers: axum::http::HeaderMap,
    Json(request): Json<GrantCapabilityRequest>,
) -> Result<Json<CapabilityResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    // Require authentication
    let (_identity, _) = require_auth_context(&headers, &auth).await.map_err(|e| {
        (
            e,
            Json(AuthErrorResponse {
                error: "Unauthorized".to_string(),
                code: "UNAUTHORIZED".to_string(),
            }),
        )
    })?;

    // Get identity secret key (in production, this would come from secure storage)
    // For now, we return error as we can't sign without the secret key
    // In real implementation, you'd have a way to retrieve the granter's secret key
    // or the client would sign the capability

    // Parse resource pattern
    let _pattern = parse_resource_pattern(&request.resource).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(AuthErrorResponse {
                error: e,
                code: "INVALID_RESOURCE".to_string(),
            }),
        )
    })?;

    // Calculate expiry
    let _expires_at = request
        .expires_in_seconds
        .map(|secs| chrono::Utc::now() + chrono::Duration::seconds(secs));

    // NOTE: This is a placeholder - in production, capability signing
    // should happen client-side or with proper key management
    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(AuthErrorResponse {
            error: "Capability granting requires client-side signing or secure key storage"
                .to_string(),
            code: "NOT_IMPLEMENTED".to_string(),
        }),
    ))
}

/// Handle capability revocation.
async fn handle_revoke_capability(
    State(auth): State<Arc<AuthManager>>,
    headers: axum::http::HeaderMap,
    Json(_request): Json<RevokeCapabilityRequest>,
) -> Result<StatusCode, (StatusCode, Json<AuthErrorResponse>)> {
    // Require authentication
    let _ = require_auth_context(&headers, &auth).await.map_err(|e| {
        (
            e,
            Json(AuthErrorResponse {
                error: "Unauthorized".to_string(),
                code: "UNAUTHORIZED".to_string(),
            }),
        )
    })?;

    // Get the capability
    // NOTE: This is a placeholder - full implementation needs capability lookup
    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(AuthErrorResponse {
            error: "Capability revocation requires full implementation".to_string(),
            code: "NOT_IMPLEMENTED".to_string(),
        }),
    ))
}

/// Handle authorization check.
async fn handle_authorize(
    State(auth): State<Arc<AuthManager>>,
    headers: axum::http::HeaderMap,
    Json(request): Json<AuthorizeRequest>,
) -> Result<Json<AuthorizeResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    // Require authentication
    let (identity, _) = require_auth_context(&headers, &auth).await.map_err(|e| {
        (
            e,
            Json(AuthErrorResponse {
                error: "Unauthorized".to_string(),
                code: "UNAUTHORIZED".to_string(),
            }),
        )
    })?;

    // Check authorization
    let authorized = auth.check_permission(
        &identity.public_key,
        &request.namespace,
        &request.key,
        request.permission,
    );

    Ok(Json(AuthorizeResponse {
        authorized,
        capability: None, // Could look up and return the specific capability
    }))
}

/// Handle listing capabilities.
async fn handle_list_capabilities(
    State(auth): State<Arc<AuthManager>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<Vec<Capability>>, (StatusCode, Json<AuthErrorResponse>)> {
    // Get identity key from headers (may be unauthenticated)
    let ctx = extract_auth_context(&headers, &auth).await.map_err(|e| {
        (
            e,
            Json(AuthErrorResponse {
                error: "Unauthorized".to_string(),
                code: "UNAUTHORIZED".to_string(),
            }),
        )
    })?;
    let identity_key = match ctx.identity_key() {
        Some(key) => key,
        None => return Ok(Json(vec![])),
    };

    // Get capabilities
    let capabilities = auth.get_capabilities(identity_key).map_err(auth_error)?;

    Ok(Json(capabilities))
}

// ============================================================================
// Helpers
// ============================================================================

/// Convert AuthError to HTTP error response.
fn auth_error(err: AuthError) -> (StatusCode, Json<AuthErrorResponse>) {
    let (status, code) = match err {
        AuthError::IdentityNotFound(_) => (StatusCode::NOT_FOUND, "IDENTITY_NOT_FOUND"),
        AuthError::IdentityExists(_) => (StatusCode::CONFLICT, "IDENTITY_EXISTS"),
        AuthError::InvalidProofOfWork => (StatusCode::BAD_REQUEST, "INVALID_PROOF_OF_WORK"),
        AuthError::InvalidKeyFormat => (StatusCode::BAD_REQUEST, "INVALID_KEY_FORMAT"),
        AuthError::ChallengeExpired => (StatusCode::GONE, "CHALLENGE_EXPIRED"),
        AuthError::InvalidSignature => (StatusCode::UNAUTHORIZED, "INVALID_SIGNATURE"),
        AuthError::SessionExpired => (StatusCode::UNAUTHORIZED, "SESSION_EXPIRED"),
        AuthError::Unauthorized => (StatusCode::FORBIDDEN, "UNAUTHORIZED"),
        AuthError::CapabilityNotFound(_) => (StatusCode::NOT_FOUND, "CAPABILITY_NOT_FOUND"),
        AuthError::CapabilityRevoked => (StatusCode::FORBIDDEN, "CAPABILITY_REVOKED"),
        AuthError::InsufficientPermissions => (StatusCode::FORBIDDEN, "INSUFFICIENT_PERMISSIONS"),
        AuthError::RateLimitExceeded => (StatusCode::TOO_MANY_REQUESTS, "RATE_LIMIT_EXCEEDED"),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
    };

    (
        status,
        Json(AuthErrorResponse {
            error: err.to_string(),
            code: code.to_string(),
        }),
    )
}

/// Parse a resource pattern string.
fn parse_resource_pattern(pattern: &str) -> Result<ResourcePattern, String> {
    if pattern.ends_with(":**") {
        // Namespace pattern: "users:**"
        let ns = pattern.trim_end_matches(":**");
        Ok(ResourcePattern::Namespace(ns.to_string()))
    } else if pattern.ends_with('*') {
        // Wildcard pattern: "users:alice:*"
        let prefix = pattern.trim_end_matches('*');
        Ok(ResourcePattern::Wildcard {
            prefix: prefix.to_string(),
        })
    } else if pattern.contains(':') {
        // Exact pattern: "users:alice:profile"
        Ok(ResourcePattern::Exact(pattern.to_string()))
    } else {
        Err(format!("Invalid resource pattern: {}", pattern))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_resource_pattern() {
        // Exact
        let exact = parse_resource_pattern("users:alice:profile").unwrap();
        assert!(matches!(exact, ResourcePattern::Exact(_)));

        // Wildcard
        let wildcard = parse_resource_pattern("users:alice:*").unwrap();
        assert!(matches!(wildcard, ResourcePattern::Wildcard { .. }));

        // Namespace
        let ns = parse_resource_pattern("users:**").unwrap();
        assert!(matches!(ns, ResourcePattern::Namespace(_)));

        // Invalid
        assert!(parse_resource_pattern("invalid").is_err());
    }

    #[test]
    fn test_auth_context() {
        let ctx = AuthContext::unauthenticated();
        assert!(!ctx.is_authenticated());
        assert!(ctx.require_auth().is_err());
    }
}
