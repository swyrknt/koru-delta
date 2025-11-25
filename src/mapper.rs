/// Document-to-Distinction mapping layer.
///
/// This module provides the critical bridge between user-facing JSON data
/// and the underlying distinction calculus engine. It enables:
///
/// - Conversion of arbitrary JSON → content-addressed distinctions
/// - Deterministic mapping (same data → same distinction)
/// - Efficient byte-level canonicalization via koru-lambda-core
///
/// The mapper is the foundation that allows KoruDelta to store any JSON
/// data while maintaining mathematical guarantees from the distinction engine.
use crate::error::{DeltaError, DeltaResult};
use koru_lambda_core::{Canonicalizable, Distinction, DistinctionEngine};
use serde_json::Value as JsonValue;

/// Maps JSON documents to and from distinction structures.
///
/// This is a stateless utility struct that performs deterministic
/// transformations between the user-facing JSON API and the internal
/// distinction-based representation.
pub struct DocumentMapper;

impl DocumentMapper {
    /// Convert a JSON value to a distinction structure.
    ///
    /// The mapping is deterministic and content-addressed:
    /// - Same JSON always produces the same distinction
    /// - The distinction ID serves as a cryptographic hash of the content
    ///
    /// # Algorithm
    ///
    /// 1. Serialize JSON to canonical byte representation
    /// 2. Map each byte to a distinction using koru-lambda-core's ByteMapping
    /// 3. Synthesize the byte distinctions into a single root distinction
    ///
    /// # Thread Safety
    ///
    /// This method is thread-safe and can be called concurrently.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let engine = DistinctionEngine::new();
    /// let json = serde_json::json!({"name": "Alice", "age": 30});
    /// let distinction = DocumentMapper::json_to_distinction(&json, &engine)?;
    /// ```
    pub fn json_to_distinction(
        value: &JsonValue,
        engine: &DistinctionEngine,
    ) -> DeltaResult<Distinction> {
        // Serialize to canonical JSON bytes
        let bytes = serde_json::to_vec(value).map_err(DeltaError::SerializationError)?;

        // Map bytes to distinction structure
        Ok(Self::bytes_to_distinction(&bytes, engine))
    }

    /// Convert raw bytes to a distinction structure.
    ///
    /// This is the core mapping function that converts any byte sequence
    /// into a content-addressed distinction. Each byte is mapped to a
    /// distinction, then all byte distinctions are synthesized together.
    ///
    /// # Determinism
    ///
    /// The synthesis order is deterministic (left-to-right fold), ensuring
    /// that the same byte sequence always produces the same distinction.
    ///
    /// # Performance
    ///
    /// Uses koru-lambda-core's cached byte mapping for O(1) per-byte lookups.
    pub fn bytes_to_distinction(bytes: &[u8], engine: &DistinctionEngine) -> Distinction {
        if bytes.is_empty() {
            // Empty data maps to d0 (the void)
            return engine.d0().clone();
        }

        // Convert each byte to a distinction and fold into a single root
        bytes
            .iter()
            .map(|&byte| byte.to_canonical_structure(engine))
            .fold(engine.d0().clone(), |acc, d| engine.synthesize(&acc, &d))
    }

    /// Store a distinction ID for later retrieval.
    ///
    /// Since distinctions are content-addressed, we only need to store
    /// the distinction ID (SHA256 hash) to later reconstruct the mapping.
    /// The actual data is stored separately as JSON.
    ///
    /// This method exists for API consistency and future extension.
    pub fn store_distinction_id(distinction: &Distinction) -> String {
        distinction.id().to_string()
    }

    /// Validate that a distinction ID matches the expected format.
    ///
    /// Distinction IDs are SHA256 hashes represented as hex strings.
    /// This validates basic structural integrity.
    ///
    /// This function is currently unused but kept for future validation needs.
    #[allow(dead_code)]
    pub fn validate_distinction_id(id: &str) -> DeltaResult<()> {
        // Basic validation: should be a hex string of appropriate length
        // SHA256 produces 64 hex characters
        if id.len() != 64 && id != "0" && id != "1" {
            return Err(DeltaError::InvalidData {
                reason: format!("Invalid distinction ID format: {}", id),
            });
        }

        // Validate hex characters
        if !id.chars().all(|c| c.is_ascii_hexdigit() || c == ':') {
            return Err(DeltaError::InvalidData {
                reason: format!("Distinction ID contains invalid characters: {}", id),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_json_to_distinction_determinism() {
        let engine = DistinctionEngine::new();
        let json = json!({"name": "Alice", "age": 30});

        let d1 = DocumentMapper::json_to_distinction(&json, &engine).unwrap();
        let d2 = DocumentMapper::json_to_distinction(&json, &engine).unwrap();

        // Same JSON should produce same distinction
        assert_eq!(d1.id(), d2.id());
    }

    #[test]
    fn test_different_json_different_distinction() {
        let engine = DistinctionEngine::new();
        let json1 = json!({"name": "Alice"});
        let json2 = json!({"name": "Bob"});

        let d1 = DocumentMapper::json_to_distinction(&json1, &engine).unwrap();
        let d2 = DocumentMapper::json_to_distinction(&json2, &engine).unwrap();

        // Different JSON should produce different distinctions
        assert_ne!(d1.id(), d2.id());
    }

    #[test]
    fn test_empty_bytes_maps_to_d0() {
        let engine = DistinctionEngine::new();
        let distinction = DocumentMapper::bytes_to_distinction(&[], &engine);

        // Empty bytes should map to d0 (the void)
        assert_eq!(distinction.id(), engine.d0().id());
    }

    #[test]
    fn test_single_byte_mapping() {
        let engine = DistinctionEngine::new();
        let byte = 42u8;

        let d1 = DocumentMapper::bytes_to_distinction(&[byte], &engine);
        let d2 = DocumentMapper::bytes_to_distinction(&[byte], &engine);

        // Same byte should produce same distinction
        assert_eq!(d1.id(), d2.id());
    }

    #[test]
    fn test_bytes_to_distinction_order_matters() {
        let engine = DistinctionEngine::new();
        let bytes1 = &[1, 2, 3];
        let bytes2 = &[3, 2, 1];

        let d1 = DocumentMapper::bytes_to_distinction(bytes1, &engine);
        let d2 = DocumentMapper::bytes_to_distinction(bytes2, &engine);

        // Different order should produce different distinctions
        assert_ne!(d1.id(), d2.id());
    }

    #[test]
    fn test_store_distinction_id() {
        let engine = DistinctionEngine::new();
        let json = json!({"test": true});
        let distinction = DocumentMapper::json_to_distinction(&json, &engine).unwrap();

        let id = DocumentMapper::store_distinction_id(&distinction);

        // Should return the distinction's ID
        assert_eq!(id, distinction.id());
    }

    #[test]
    fn test_validate_distinction_id_valid() {
        // Valid SHA256 hex string (64 characters)
        let valid_id = "a".repeat(64);
        assert!(DocumentMapper::validate_distinction_id(&valid_id).is_ok());

        // Primordial distinctions
        assert!(DocumentMapper::validate_distinction_id("0").is_ok());
        assert!(DocumentMapper::validate_distinction_id("1").is_ok());
    }

    #[test]
    fn test_validate_distinction_id_invalid() {
        // Too short
        let invalid_id = "abc123";
        assert!(DocumentMapper::validate_distinction_id(invalid_id).is_err());

        // Invalid characters
        let invalid_id = "g".repeat(64);
        assert!(DocumentMapper::validate_distinction_id(&invalid_id).is_err());
    }

    #[test]
    fn test_complex_json_structures() {
        let engine = DistinctionEngine::new();

        let complex_json = json!({
            "user": {
                "name": "Alice",
                "email": "alice@example.com",
                "age": 30,
                "active": true,
                "tags": ["admin", "developer"],
                "metadata": {
                    "created": "2025-01-01",
                    "updated": "2025-01-15"
                }
            }
        });

        let d1 = DocumentMapper::json_to_distinction(&complex_json, &engine).unwrap();
        let d2 = DocumentMapper::json_to_distinction(&complex_json, &engine).unwrap();

        // Complex structures should also be deterministic
        assert_eq!(d1.id(), d2.id());
    }
}
