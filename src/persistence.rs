/// Persistence layer for KoruDelta.
///
/// This module provides serialization and deserialization of the database state
/// to/from disk, enabling durability across restarts. The persistence format
/// uses bincode for efficient binary serialization of the entire database state.
///
/// # File Format
///
/// The database is serialized to a single file containing:
/// - All versioned values for all keys
/// - Complete history logs
/// - Metadata (version, timestamps, etc.)
///
/// # Usage
///
/// ```ignore
/// // Save database to disk
/// persistence::save(&db, &path).await?;
///
/// // Load database from disk
/// let db = persistence::load(&path).await?;
/// ```
use crate::error::{DeltaError, DeltaResult};
use crate::storage::CausalStorage;
use crate::types::{FullKey, VersionedValue};
use koru_lambda_core::DistinctionEngine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::fs;

/// Serializable snapshot of the entire database state.
///
/// This structure captures all data needed to reconstruct the database
/// after a restart, including current state and complete history.
///
/// Uses Vec instead of HashMap for JSON serialization compatibility.
#[derive(Debug, Serialize, Deserialize)]
struct DatabaseSnapshot {
    /// Format version for future compatibility
    version: u32,
    /// Current state: latest value for each key
    current_state: Vec<(FullKey, VersionedValue)>,
    /// Complete history: all versions for each key
    history_log: Vec<(FullKey, Vec<VersionedValue>)>,
}

const SNAPSHOT_VERSION: u32 = 1;

/// Save the database state to disk.
///
/// This performs a consistent snapshot of the entire database and writes
/// it atomically to the specified path. The file is written to a temporary
/// location first, then moved to the final path to ensure atomicity.
///
/// # Arguments
///
/// * `storage` - The storage engine to save
/// * `path` - Path where the database file should be written
///
/// # Errors
///
/// Returns `DeltaError::StorageError` if:
/// - The file cannot be created
/// - Serialization fails
/// - The atomic rename fails
///
/// # Example
///
/// ```ignore
/// persistence::save(&storage, Path::new("/data/korudelta.db")).await?;
/// ```
pub async fn save(storage: &CausalStorage, path: &Path) -> DeltaResult<()> {
    // Create parent directory if it doesn't exist
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|e| DeltaError::StorageError(format!("Failed to create directory: {}", e)))?;
    }

    // Get consistent snapshot of the database state
    let (current_state, history_log) = storage.create_snapshot();

    // Convert HashMaps to Vecs for JSON serialization
    let current_state_vec: Vec<(FullKey, VersionedValue)> = current_state.into_iter().collect();
    let history_log_vec: Vec<(FullKey, Vec<VersionedValue>)> = history_log.into_iter().collect();

    let snapshot = DatabaseSnapshot {
        version: SNAPSHOT_VERSION,
        current_state: current_state_vec,
        history_log: history_log_vec,
    };

    // Serialize to JSON bytes (bincode doesn't work with serde_json::Value)
    let bytes = serde_json::to_vec(&snapshot)
        .map_err(|e| DeltaError::StorageError(format!("Failed to serialize database: {}", e)))?;

    // Write to temporary file first
    let temp_path = path.with_extension("tmp");
    fs::write(&temp_path, &bytes)
        .await
        .map_err(|e| DeltaError::StorageError(format!("Failed to write temporary file: {}", e)))?;

    // Atomic rename to final location
    fs::rename(&temp_path, path)
        .await
        .map_err(|e| DeltaError::StorageError(format!("Failed to rename file: {}", e)))?;

    Ok(())
}

/// Load the database state from disk.
///
/// Reads the serialized database from the specified path and reconstructs
/// the full database state, including all history.
///
/// # Arguments
///
/// * `path` - Path to the database file
/// * `engine` - The distinction engine to use (should be a fresh instance)
///
/// # Returns
///
/// Returns a new `CausalStorage` instance with all data restored.
///
/// # Errors
///
/// Returns `DeltaError::StorageError` if:
/// - The file doesn't exist or can't be read
/// - Deserialization fails
/// - The snapshot version is incompatible
///
/// # Example
///
/// ```ignore
/// let engine = Arc::new(DistinctionEngine::new());
/// let storage = persistence::load(Path::new("/data/korudelta.db"), engine).await?;
/// ```
pub async fn load(path: &Path, engine: Arc<DistinctionEngine>) -> DeltaResult<CausalStorage> {
    // Read file
    let bytes = fs::read(path)
        .await
        .map_err(|e| DeltaError::StorageError(format!("Failed to read database file: {}", e)))?;

    // Deserialize snapshot from JSON
    let snapshot: DatabaseSnapshot = serde_json::from_slice(&bytes)
        .map_err(|e| DeltaError::StorageError(format!("Failed to deserialize database: {}", e)))?;

    // Verify version compatibility
    if snapshot.version != SNAPSHOT_VERSION {
        return Err(DeltaError::StorageError(format!(
            "Incompatible database version: {} (expected {})",
            snapshot.version, SNAPSHOT_VERSION
        )));
    }

    // Convert Vecs back to HashMaps
    let current_state: HashMap<FullKey, VersionedValue> =
        snapshot.current_state.into_iter().collect();
    let history_log: HashMap<FullKey, Vec<VersionedValue>> =
        snapshot.history_log.into_iter().collect();

    // Restore storage from snapshot
    Ok(CausalStorage::from_snapshot(
        engine,
        current_state,
        history_log,
    ))
}

/// Check if a database file exists at the given path.
///
/// # Example
///
/// ```ignore
/// if persistence::exists(Path::new("/data/korudelta.db")).await {
///     // Load existing database
/// } else {
///     // Create new database
/// }
/// ```
pub async fn exists(path: &Path) -> bool {
    fs::metadata(path).await.is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::CausalStorage;
    use serde_json::json;
    use tempfile::NamedTempFile;

    fn create_test_storage() -> CausalStorage {
        let engine = Arc::new(DistinctionEngine::new());
        CausalStorage::new(engine)
    }

    #[tokio::test]
    async fn test_save_and_load() {
        let storage = create_test_storage();

        // Add some data
        storage
            .put("users", "alice", json!({"name": "Alice"}))
            .unwrap();
        storage.put("users", "bob", json!({"name": "Bob"})).unwrap();
        storage
            .put("users", "alice", json!({"name": "Alice", "age": 30}))
            .unwrap();

        // Save to file
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        save(&storage, path).await.unwrap();

        // Load from file
        let engine = Arc::new(DistinctionEngine::new());
        let loaded_storage = load(path, engine).await.unwrap();

        // Verify data was restored
        let alice = loaded_storage.get("users", "alice").unwrap();
        assert_eq!(alice.value(), &json!({"name": "Alice", "age": 30}));

        let bob = loaded_storage.get("users", "bob").unwrap();
        assert_eq!(bob.value(), &json!({"name": "Bob"}));

        // Verify history was restored
        let alice_history = loaded_storage.history("users", "alice").unwrap();
        assert_eq!(alice_history.len(), 2);
    }

    #[tokio::test]
    async fn test_save_empty_database() {
        let storage = create_test_storage();
        let temp_file = NamedTempFile::new().unwrap();

        // Should be able to save empty database
        save(&storage, temp_file.path()).await.unwrap();

        // Should be able to load empty database
        let engine = Arc::new(DistinctionEngine::new());
        let loaded = load(temp_file.path(), engine).await.unwrap();
        assert_eq!(loaded.key_count(), 0);
    }

    #[tokio::test]
    async fn test_exists() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // File exists (tempfile creates it)
        assert!(exists(path).await);

        // Non-existent file
        assert!(!exists(Path::new("/nonexistent/path/database.db")).await);
    }

    #[tokio::test]
    async fn test_load_nonexistent_file() {
        let engine = Arc::new(DistinctionEngine::new());
        let result = load(Path::new("/nonexistent/file.db"), engine).await;
        assert!(matches!(result, Err(DeltaError::StorageError(_))));
    }
}
