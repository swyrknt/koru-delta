/// Common types used throughout KoruDelta.
///
/// This module defines the core data structures that represent the database's
/// internal model. These types are designed to be simple, immutable, and
/// content-addressable where possible.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::sync::Arc;

/// A fully-qualified key combining namespace and key.
///
/// KoruDelta organizes data into namespaces (similar to tables or collections)
/// with keys within each namespace. This type represents the combination.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FullKey {
    /// The namespace (e.g., "users", "sessions", "config")
    pub namespace: String,
    /// The key within the namespace (e.g., "alice", "session:123")
    pub key: String,
}

impl FullKey {
    /// Create a new fully-qualified key.
    pub fn new(namespace: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            key: key.into(),
        }
    }

    /// Get a canonical string representation for hashing.
    ///
    /// Format: "namespace:key"
    pub fn to_canonical_string(&self) -> String {
        format!("{}:{}", self.namespace, self.key)
    }
}

/// A versioned value with metadata.
///
/// Every write in KoruDelta creates a new version. This structure captures
/// the value along with its temporal and causal metadata.
///
/// The value is stored in an `Arc` to enable memory-efficient deduplication:
/// identical values share the same underlying allocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedValue {
    /// The actual data stored (Arc-wrapped for deduplication)
    #[serde(
        serialize_with = "serialize_arc_json",
        deserialize_with = "deserialize_arc_json"
    )]
    pub value: Arc<JsonValue>,
    /// When this version was created
    pub timestamp: DateTime<Utc>,
    /// Content-addressed ID of this version (distinction ID)
    pub version_id: String,
    /// ID of the previous version (for causal chain)
    pub previous_version: Option<String>,
}

/// Serialize Arc<JsonValue> as plain JsonValue
fn serialize_arc_json<S>(value: &Arc<JsonValue>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    value.as_ref().serialize(serializer)
}

/// Deserialize JsonValue into Arc<JsonValue>
fn deserialize_arc_json<'de, D>(deserializer: D) -> Result<Arc<JsonValue>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = JsonValue::deserialize(deserializer)?;
    Ok(Arc::new(value))
}

impl VersionedValue {
    /// Create a new versioned value.
    pub fn new(
        value: Arc<JsonValue>,
        timestamp: DateTime<Utc>,
        version_id: String,
        previous_version: Option<String>,
    ) -> Self {
        Self {
            value,
            timestamp,
            version_id,
            previous_version,
        }
    }

    /// Create a new versioned value from a plain JsonValue.
    /// This wraps the value in an Arc.
    pub fn from_json(
        value: JsonValue,
        timestamp: DateTime<Utc>,
        version_id: String,
        previous_version: Option<String>,
    ) -> Self {
        Self {
            value: Arc::new(value),
            timestamp,
            version_id,
            previous_version,
        }
    }

    /// Get the value as a reference.
    pub fn value(&self) -> &JsonValue {
        &self.value
    }

    /// Get the timestamp when this version was created.
    pub fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    /// Get the content-addressed version ID.
    pub fn version_id(&self) -> &str {
        &self.version_id
    }

    /// Get the previous version ID if this is not the first version.
    pub fn previous_version(&self) -> Option<&str> {
        self.previous_version.as_deref()
    }
}

/// A history entry representing a single change to a key.
///
/// This is returned by the `history()` method and provides a chronological
/// view of all changes to a specific key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// The value at this point in history
    pub value: JsonValue,
    /// When this change occurred
    pub timestamp: DateTime<Utc>,
    /// The version ID for this change
    pub version_id: String,
}

impl HistoryEntry {
    /// Create a new history entry.
    pub fn new(value: JsonValue, timestamp: DateTime<Utc>, version_id: String) -> Self {
        Self {
            value,
            timestamp,
            version_id,
        }
    }
}

impl From<&VersionedValue> for HistoryEntry {
    fn from(versioned: &VersionedValue) -> Self {
        Self {
            value: (*versioned.value).clone(),
            timestamp: versioned.timestamp,
            version_id: versioned.version_id.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_key_canonical_string() {
        let key = FullKey::new("users", "alice");
        assert_eq!(key.to_canonical_string(), "users:alice");
    }

    #[test]
    fn test_full_key_equality() {
        let key1 = FullKey::new("users", "alice");
        let key2 = FullKey::new("users", "alice");
        let key3 = FullKey::new("users", "bob");

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_versioned_value_accessors() {
        let now = Utc::now();
        let value = serde_json::json!({"name": "Alice"});
        let versioned = VersionedValue::from_json(
            value.clone(),
            now,
            "version1".to_string(),
            Some("version0".to_string()),
        );

        assert_eq!(versioned.value(), &value);
        assert_eq!(versioned.timestamp(), now);
        assert_eq!(versioned.version_id(), "version1");
        assert_eq!(versioned.previous_version(), Some("version0"));
    }

    #[test]
    fn test_history_entry_from_versioned_value() {
        let now = Utc::now();
        let value = serde_json::json!({"count": 42});
        let versioned = VersionedValue::from_json(value.clone(), now, "v1".to_string(), None);

        let entry: HistoryEntry = (&versioned).into();

        assert_eq!(entry.value, value);
        assert_eq!(entry.timestamp, now);
        assert_eq!(entry.version_id, "v1");
    }
}
