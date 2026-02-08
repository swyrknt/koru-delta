/// Common types used throughout KoruDelta.
///
/// This module defines the core data structures that represent the database's
/// internal model. These types are designed to be simple, immutable, and
/// content-addressable where possible.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;

/// A version identifier for causal tracking.
pub type VersionId = u64;

/// Vector clock for causal ordering in distributed systems.
///
/// A vector clock tracks the happens-before relationship between events
/// across multiple nodes. Each node maintains a monotonic counter, and
/// the vector is updated on every write.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VectorClock {
    /// Node ID -> Logical timestamp mapping
    pub clocks: HashMap<String, u64>,
}

impl VectorClock {
    /// Create a new empty vector clock.
    pub fn new() -> Self {
        Self {
            clocks: HashMap::new(),
        }
    }

    /// Increment the clock for a specific node.
    pub fn increment(&mut self, node_id: &str) {
        let entry = self.clocks.entry(node_id.to_string()).or_insert(0);
        *entry += 1;
    }

    /// Merge another vector clock into this one (taking max of each clock).
    pub fn merge(&mut self, other: &VectorClock) {
        for (node_id, timestamp) in &other.clocks {
            let entry = self.clocks.entry(node_id.clone()).or_insert(0);
            *entry = (*entry).max(*timestamp);
        }
    }

    /// Compare two vector clocks.
    ///
    /// Returns:
    /// - `Some(Ordering::Less)` if self happened before other
    /// - `Some(Ordering::Greater)` if self happened after other
    /// - `Some(Ordering::Equal)` if they're the same
    /// - `None` if they're concurrent (conflict)
    pub fn compare(&self, other: &VectorClock) -> Option<std::cmp::Ordering> {
        let all_nodes: std::collections::HashSet<_> = self
            .clocks
            .keys()
            .chain(other.clocks.keys())
            .collect();

        let mut has_less = false;
        let mut has_greater = false;

        for node_id in all_nodes {
            let self_val = self.clocks.get(node_id).copied().unwrap_or(0);
            let other_val = other.clocks.get(node_id).copied().unwrap_or(0);

            match self_val.cmp(&other_val) {
                std::cmp::Ordering::Less => has_less = true,
                std::cmp::Ordering::Greater => has_greater = true,
                std::cmp::Ordering::Equal => {}
            }
        }

        match (has_less, has_greater) {
            (true, true) => None,     // Concurrent (conflict)
            (true, false) => Some(std::cmp::Ordering::Less),
            (false, true) => Some(std::cmp::Ordering::Greater),
            (false, false) => Some(std::cmp::Ordering::Equal),
        }
    }

    /// Check if this clock dominates (happened after) another.
    pub fn dominates(&self, other: &VectorClock) -> bool {
        matches!(self.compare(other), Some(std::cmp::Ordering::Greater))
    }

    /// Check if this clock is dominated by (happened before) another.
    pub fn is_dominated_by(&self, other: &VectorClock) -> bool {
        matches!(self.compare(other), Some(std::cmp::Ordering::Less))
    }

    /// Check if clocks are concurrent (conflict).
    pub fn is_concurrent_with(&self, other: &VectorClock) -> bool {
        self.compare(other).is_none()
    }
}

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
///
/// # ID Fields
///
/// - `write_id`: Unique identifier for this specific write event (includes timestamp)
/// - `distinction_id`: Content hash of the value (same content = same distinction_id)
/// - `previous_version`: The write_id of the previous version of this key
/// - `vector_clock`: Causal ordering for distributed conflict resolution
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
    /// Unique write ID (includes timestamp to distinguish identical values written at different times)
    pub write_id: String,
    /// Content-addressed distinction ID (content hash for deduplication)
    pub distinction_id: String,
    /// ID of the previous version (for causal chain)
    pub previous_version: Option<String>,
    /// Vector clock for causal ordering in distributed systems
    pub vector_clock: VectorClock,
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
        write_id: String,
        distinction_id: String,
        previous_version: Option<String>,
        vector_clock: VectorClock,
    ) -> Self {
        Self {
            value,
            timestamp,
            write_id,
            distinction_id,
            previous_version,
            vector_clock,
        }
    }

    /// Create a new versioned value from a plain JsonValue.
    /// This wraps the value in an Arc.
    pub fn from_json(
        value: JsonValue,
        timestamp: DateTime<Utc>,
        write_id: String,
        distinction_id: String,
        previous_version: Option<String>,
        vector_clock: VectorClock,
    ) -> Self {
        Self {
            value: Arc::new(value),
            timestamp,
            write_id,
            distinction_id,
            previous_version,
            vector_clock,
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

    /// Get the unique write ID (includes timestamp).
    /// This is unique per write, even for identical values.
    pub fn write_id(&self) -> &str {
        &self.write_id
    }

    /// Get the content-addressed distinction ID (content hash).
    /// Same content = same distinction_id (for deduplication).
    pub fn distinction_id(&self) -> &str {
        &self.distinction_id
    }

    /// Get the version_id (returns distinction_id for content addressing).
    /// Same value = same version_id (for content addressing/deduplication)
    pub fn version_id(&self) -> &str {
        &self.distinction_id
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
            version_id: versioned.distinction_id.clone(), // Use distinction_id (content hash)
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
            "write_1".to_string(),  // write_id (unique per write)
            "dist_abc".to_string(), // distinction_id (content hash)
            Some("write_0".to_string()),
            VectorClock::new(),
        );

        assert_eq!(versioned.value(), &value);
        assert_eq!(versioned.timestamp(), now);
        assert_eq!(versioned.write_id(), "write_1");
        assert_eq!(versioned.distinction_id(), "dist_abc");
        assert_eq!(versioned.version_id(), "dist_abc"); // version_id returns distinction_id
        assert_eq!(versioned.previous_version(), Some("write_0"));
    }

    #[test]
    fn test_history_entry_from_versioned_value() {
        let now = Utc::now();
        let value = serde_json::json!({"count": 42});
        let versioned = VersionedValue::from_json(
            value.clone(), 
            now, 
            "write_v1".to_string(), // write_id
            "dist_xyz".to_string(), // distinction_id
            None,
            VectorClock::new(),
        );

        let entry: HistoryEntry = (&versioned).into();

        assert_eq!(entry.value, value);
        assert_eq!(entry.timestamp, now);
        assert_eq!(entry.version_id, "dist_xyz"); // History uses distinction_id
    }
}
