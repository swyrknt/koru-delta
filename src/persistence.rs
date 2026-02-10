/// Persistence layer for KoruDelta - Content-Addressed Write-Ahead Log.
///
/// This module provides durable storage using a write-ahead log (WAL) approach
/// combined with content-addressed value storage. This is efficient because:
///
/// 1. **Append-only writes**: Each write appends to the log (O(1), not O(n))
/// 2. **Content-addressed values**: Values stored by hash (koru-lambda-core's distinction IDs)
/// 3. **Structural sharing**: Identical values are stored once
/// 4. **Immutable history**: The log is the history - no duplication needed
///
/// # Storage Layout
///
/// ```text
/// ~/.korudelta/
/// ├── db/                    # Database directory
/// │   ├── wal/              # Write-ahead log files (append-only)
/// │   │   ├── 000001.wal    # Log segments
/// │   │   ├── 000002.wal
/// │   │   └── current       # Points to active segment
/// │   ├── values/           # Content-addressed value store
/// │   │   ├── ab/           # First 2 chars of hash
/// │   │   │   └── cd...     # Rest of hash
/// │   │   └── ef/
/// │   └── snapshots/        # Periodic full snapshots
/// │       └── 000001.snapshot
/// ```
///
/// # Log Entry Format
///
/// Each entry is a JSON line (newline-delimited JSON):
/// ```json
/// {"type":"put","ns":"users","key":"alice","value_hash":"abc123...","prev_hash":"def456...","timestamp":"2026-02-05T12:00:00Z","seq":42}
/// ```
///
/// On startup, we replay the log to rebuild the in-memory state.
///
/// # Usage
///
/// ```ignore
/// // Append a write to the log
/// persistence::append_write(&path, "users", "alice", &versioned_value).await?;
///
/// // Load database from log
/// let storage = persistence::load_from_wal(&path, engine).await?;
/// ```
use crate::error::{DeltaError, DeltaResult};
use crate::storage::CausalStorage;
use crate::types::{FullKey, VectorClock, VersionedValue};
use chrono::{DateTime, Utc};
use koru_lambda_core::DistinctionEngine;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use std::path::Path;
use std::sync::Arc;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// Current WAL format version.
const WAL_VERSION: u32 = 1;

/// Maximum WAL segment size before rotation (10MB).
const MAX_SEGMENT_SIZE: u64 = 10 * 1024 * 1024;

/// A single entry in the write-ahead log.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LogEntry {
    /// WAL format version.
    version: u32,
    /// Operation type: "put" or "delete".
    op: String,
    /// Namespace/collection.
    ns: String,
    /// Key within namespace.
    key: String,
    /// Content hash of the value (distinction ID).
    value_hash: String,
    /// Hash of previous version (for causal chain).
    prev_hash: Option<String>,
    /// Timestamp of the write.
    timestamp: DateTime<Utc>,
    /// Monotonic sequence number.
    seq: u64,
    /// The actual value (only in "inline" mode for small values).
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<JsonValue>,
    /// Checksum of the entry (for corruption detection).
    /// Format: "crc32:XXXXXXXX" where X is hex.
    checksum: String,
}

/// Calculate CRC32 checksum for data integrity.
fn calculate_checksum(data: &str) -> String {
    let crc = crc32fast::hash(data.as_bytes());
    format!("crc32:{:08x}", crc)
}

/// Verify entry checksum.
fn verify_checksum(entry: &LogEntry) -> bool {
    // Create a copy without the checksum for verification
    let json = serde_json::json!({
        "version": entry.version,
        "op": &entry.op,
        "ns": &entry.ns,
        "key": &entry.key,
        "value_hash": &entry.value_hash,
        "prev_hash": &entry.prev_hash,
        "timestamp": entry.timestamp,
        "seq": entry.seq,
        "value": &entry.value,
    });
    let data = json.to_string();
    let expected = calculate_checksum(&data);
    entry.checksum == expected
}

/// Metadata for the WAL.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WalMetadata {
    /// Last sequence number assigned.
    last_seq: u64,
    /// Current segment number.
    current_segment: u32,
}

impl Default for WalMetadata {
    fn default() -> Self {
        Self {
            last_seq: 0,
            current_segment: 1,
        }
    }
}

/// Append a write operation to the WAL.
///
/// This is the primary persistence method. It:
/// 1. Stores the value in the content-addressed store (if not already present)
/// 2. Appends a log entry referencing the value
///
/// # Arguments
///
/// * `db_path` - Base database directory
/// * `namespace` - The namespace/collection
/// * `key` - The key
/// * `versioned` - The versioned value to persist
///
/// # Example
///
/// ```ignore
/// persistence::append_write(Path::new("~/.korudelta/db"), "users", "alice", &versioned).await?;
/// ```
pub async fn append_write(
    db_path: &Path,
    namespace: &str,
    key: &str,
    versioned: &VersionedValue,
) -> DeltaResult<()> {
    // Ensure directories exist
    let wal_dir = db_path.join("wal");
    let values_dir = db_path.join("values");
    fs::create_dir_all(&wal_dir)
        .await
        .map_err(|e| DeltaError::StorageError(format!("Failed to create WAL dir: {}", e)))?;
    fs::create_dir_all(&values_dir)
        .await
        .map_err(|e| DeltaError::StorageError(format!("Failed to create values dir: {}", e)))?;

    // Get or load metadata
    let mut metadata = load_metadata(&wal_dir).await.unwrap_or_default();
    metadata.last_seq += 1;
    let seq = metadata.last_seq;

    // Store the value (content-addressed)
    let value_hash = versioned.version_id().to_string();
    store_value(&values_dir, &value_hash, versioned.value()).await?;

    // Create log entry (without checksum first)
    let entry_without_checksum = serde_json::json!({
        "version": WAL_VERSION,
        "op": "put",
        "ns": namespace,
        "key": key,
        "value_hash": &value_hash,
        "prev_hash": versioned.previous_version(),
        "timestamp": versioned.timestamp(),
        "seq": seq,
        "value": Option::<JsonValue>::None,
    });

    // Calculate checksum
    let checksum = calculate_checksum(&entry_without_checksum.to_string());

    // Create final entry with checksum
    let entry = LogEntry {
        version: WAL_VERSION,
        op: "put".to_string(),
        ns: namespace.to_string(),
        key: key.to_string(),
        value_hash,
        prev_hash: versioned.previous_version().map(|s| s.to_string()),
        timestamp: versioned.timestamp(),
        seq,
        value: None,
        checksum,
    };

    // Serialize to JSON line
    let line = serde_json::to_string(&entry)?;

    // Get current segment path
    let segment_path = wal_dir.join(format!("{:06}.wal", metadata.current_segment));

    // Check if we need to rotate
    let should_rotate = if segment_path.exists() {
        let metadata = fs::metadata(&segment_path).await.map_err(|e| {
            DeltaError::StorageError(format!("Failed to read segment metadata: {}", e))
        })?;
        metadata.len() > MAX_SEGMENT_SIZE
    } else {
        false
    };

    if should_rotate {
        metadata.current_segment += 1;
        save_metadata(&wal_dir, &metadata).await?;
    }

    // Append to current segment
    let segment_path = wal_dir.join(format!("{:06}.wal", metadata.current_segment));
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&segment_path)
        .await
        .map_err(|e| DeltaError::StorageError(format!("Failed to open WAL: {}", e)))?;

    file.write_all(line.as_bytes())
        .await
        .map_err(|e| DeltaError::StorageError(format!("Failed to write WAL: {}", e)))?;
    file.write_all(b"\n")
        .await
        .map_err(|e| DeltaError::StorageError(format!("Failed to write WAL: {}", e)))?;

    // Ensure data is flushed to disk
    file.sync_data()
        .await
        .map_err(|e| DeltaError::StorageError(format!("Failed to sync WAL: {}", e)))?;

    // Save metadata
    save_metadata(&wal_dir, &metadata).await?;

    Ok(())
}

/// Append multiple writes to the WAL in a single batch operation.
///
/// This is significantly more efficient than calling `append_write` multiple times
/// because it performs only one fsync for all entries.
///
/// # Arguments
///
/// * `db_path` - Path to the database directory
/// * `writes` - Vector of (namespace, key, versioned_value) tuples
///
/// # Returns
///
/// Returns `Ok(())` on success, or a `DeltaError` on failure.
///
/// # Performance
///
/// For N writes, this performs:
/// - 1 fsync (vs N fsyncs for individual writes)
/// - N value stores (deduplication still applies)
/// - 1 metadata save
///
/// Typical improvement: 10-50x faster for batches of 100+ writes.
pub async fn append_write_batch(
    db_path: &Path,
    writes: Vec<(&str, &str, &VersionedValue)>,
) -> DeltaResult<()> {
    if writes.is_empty() {
        return Ok(());
    }

    // Ensure directories exist
    let wal_dir = db_path.join("wal");
    let values_dir = db_path.join("values");
    fs::create_dir_all(&wal_dir)
        .await
        .map_err(|e| DeltaError::StorageError(format!("Failed to create WAL dir: {e}")))?;
    fs::create_dir_all(&values_dir)
        .await
        .map_err(|e| DeltaError::StorageError(format!("Failed to create values dir: {e}")))?;

    // Get or load metadata
    let mut metadata = load_metadata(&wal_dir).await.unwrap_or_default();

    // Collect all lines to write
    let mut lines = Vec::with_capacity(writes.len());

    for (namespace, key, versioned) in writes {
        metadata.last_seq += 1;
        let seq = metadata.last_seq;

        // Store the value (content-addressed)
        let value_hash = versioned.version_id().to_string();
        store_value(&values_dir, &value_hash, versioned.value()).await?;

        // Create log entry
        let entry_without_checksum = serde_json::json!({
            "version": WAL_VERSION,
            "op": "put",
            "ns": namespace,
            "key": key,
            "value_hash": &value_hash,
            "prev_hash": versioned.previous_version(),
            "timestamp": versioned.timestamp(),
            "seq": seq,
            "value": Option::<JsonValue>::None,
        });

        let checksum = calculate_checksum(&entry_without_checksum.to_string());

        let entry = LogEntry {
            version: WAL_VERSION,
            op: "put".to_string(),
            ns: namespace.to_string(),
            key: key.to_string(),
            value_hash,
            prev_hash: versioned.previous_version().map(|s| s.to_string()),
            timestamp: versioned.timestamp(),
            seq,
            value: None,
            checksum,
        };

        let line = serde_json::to_string(&entry)?;
        lines.push(line);
    }

    // Get current segment path
    let segment_path = wal_dir.join(format!("{:06}.wal", metadata.current_segment));

    // Check if we need to rotate (estimate size)
    let estimated_size = lines.iter().map(|l| l.len() + 1).sum::<usize>();
    let should_rotate = if segment_path.exists() {
        let meta = fs::metadata(&segment_path).await.map_err(|e| {
            DeltaError::StorageError(format!("Failed to read segment metadata: {e}"))
        })?;
        meta.len() + estimated_size as u64 > MAX_SEGMENT_SIZE
    } else {
        false
    };

    if should_rotate {
        metadata.current_segment += 1;
        save_metadata(&wal_dir, &metadata).await?;
    }

    // Append to current segment
    let segment_path = wal_dir.join(format!("{:06}.wal", metadata.current_segment));
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&segment_path)
        .await
        .map_err(|e| DeltaError::StorageError(format!("Failed to open WAL: {e}")))?;

    // Write all entries
    for line in lines {
        file.write_all(line.as_bytes())
            .await
            .map_err(|e| DeltaError::StorageError(format!("Failed to write WAL: {e}")))?;
        file.write_all(b"\n")
            .await
            .map_err(|e| DeltaError::StorageError(format!("Failed to write WAL: {e}")))?;
    }

    // Single fsync for entire batch
    file.sync_data()
        .await
        .map_err(|e| DeltaError::StorageError(format!("Failed to sync WAL: {e}")))?;

    // Save metadata
    save_metadata(&wal_dir, &metadata).await?;

    Ok(())
}

/// Store a value in the content-addressed store.
///
/// Values are stored in a directory structure based on their hash:
/// `values/AB/CD...` where AB are the first 2 chars and CD... is the rest.
async fn store_value(values_dir: &Path, value_hash: &str, value: &JsonValue) -> DeltaResult<()> {
    if value_hash.len() < 4 {
        return Err(DeltaError::StorageError("Value hash too short".to_string()));
    }

    // Create path: values/AB/CD...
    let prefix = &value_hash[0..2];
    let suffix = &value_hash[2..];
    let value_dir = values_dir.join(prefix);
    let value_path = value_dir.join(suffix);

    // If already exists, skip (content-addressed deduplication)
    if value_path.exists() {
        return Ok(());
    }

    fs::create_dir_all(&value_dir)
        .await
        .map_err(|e| DeltaError::StorageError(format!("Failed to create value dir: {}", e)))?;

    // Write value atomically
    let temp_path = value_path.with_extension("tmp");
    let json = serde_json::to_vec(value)?;
    fs::write(&temp_path, &json)
        .await
        .map_err(|e| DeltaError::StorageError(format!("Failed to write value: {}", e)))?;
    fs::rename(&temp_path, &value_path)
        .await
        .map_err(|e| DeltaError::StorageError(format!("Failed to rename value: {}", e)))?;

    Ok(())
}

/// Load a value from the content-addressed store.
async fn load_value(values_dir: &Path, value_hash: &str) -> DeltaResult<Option<JsonValue>> {
    if value_hash.len() < 4 {
        return Ok(None);
    }

    let prefix = &value_hash[0..2];
    let suffix = &value_hash[2..];
    let value_path = values_dir.join(prefix).join(suffix);

    if !value_path.exists() {
        return Ok(None);
    }

    let bytes = fs::read(&value_path)
        .await
        .map_err(|e| DeltaError::StorageError(format!("Failed to read value: {}", e)))?;
    let value: JsonValue = serde_json::from_slice(&bytes)?;
    Ok(Some(value))
}

/// Load WAL metadata.
async fn load_metadata(wal_dir: &Path) -> DeltaResult<WalMetadata> {
    let metadata_path = wal_dir.join("metadata.json");
    let bytes = fs::read(&metadata_path)
        .await
        .map_err(|e| DeltaError::StorageError(format!("Failed to read metadata: {}", e)))?;
    let metadata: WalMetadata = serde_json::from_slice(&bytes)?;
    Ok(metadata)
}

/// Save WAL metadata.
async fn save_metadata(wal_dir: &Path, metadata: &WalMetadata) -> DeltaResult<()> {
    let metadata_path = wal_dir.join("metadata.json");
    let temp_path = metadata_path.with_extension("tmp");
    let json = serde_json::to_vec(metadata)?;
    fs::write(&temp_path, &json)
        .await
        .map_err(|e| DeltaError::StorageError(format!("Failed to write metadata: {}", e)))?;
    fs::rename(&temp_path, &metadata_path)
        .await
        .map_err(|e| DeltaError::StorageError(format!("Failed to rename metadata: {}", e)))?;
    Ok(())
}

/// Load database state from WAL.
///
/// This replays all log entries to rebuild the in-memory state.
/// It's efficient because values are loaded on-demand from the content store.
pub async fn load_from_wal(
    db_path: &Path,
    engine: Arc<DistinctionEngine>,
) -> DeltaResult<CausalStorage> {
    let storage = CausalStorage::new(engine);
    let wal_dir = db_path.join("wal");
    let values_dir = db_path.join("values");

    if !wal_dir.exists() {
        // No WAL yet, return empty storage
        return Ok(storage);
    }

    // Get all WAL segments in order
    let mut read_dir = fs::read_dir(&wal_dir)
        .await
        .map_err(|e| DeltaError::StorageError(format!("Failed to read WAL dir: {}", e)))?;

    let mut segments = Vec::new();
    while let Some(entry) = read_dir
        .next_entry()
        .await
        .map_err(|e| DeltaError::StorageError(format!("Failed to read WAL entry: {}", e)))?
    {
        if let Some(name) = entry.file_name().to_str() {
            if name.ends_with(".wal") {
                segments.push(name.to_string());
            }
        }
    }

    segments.sort();

    // Replay each segment
    for segment in segments {
        let segment_path = wal_dir.join(&segment);
        replay_segment(&segment_path, &values_dir, &storage).await?;
    }

    Ok(storage)
}

/// Replay a single WAL segment.
async fn replay_segment(
    segment_path: &Path,
    values_dir: &Path,
    storage: &CausalStorage,
) -> DeltaResult<()> {
    let file = fs::File::open(segment_path)
        .await
        .map_err(|e| DeltaError::StorageError(format!("Failed to open segment: {}", e)))?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    while let Some(line) = lines
        .next_line()
        .await
        .map_err(|e| DeltaError::StorageError(format!("Failed to read line: {}", e)))?
    {
        if line.trim().is_empty() {
            continue;
        }

        let entry: LogEntry = match serde_json::from_str(&line) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Warning: Failed to parse WAL entry: {}", e);
                continue;
            }
        };

        // Verify checksum
        if !verify_checksum(&entry) {
            eprintln!(
                "Warning: Checksum mismatch for entry seq={}, possible corruption",
                entry.seq
            );
            continue;
        }

        if entry.op == "put" {
            // Load value from content store
            if let Some(value) = load_value(values_dir, &entry.value_hash).await? {
                // Reconstruct versioned value
                // For replay: write_id = value_hash + timestamp_nanos to match original
                let write_id = format!(
                    "{}_{}",
                    entry.value_hash,
                    entry.timestamp.timestamp_nanos_opt().unwrap_or(0)
                );
                let versioned = VersionedValue::new(
                    Arc::new(value),
                    entry.timestamp,
                    write_id,                 // unique write_id for replay
                    entry.value_hash.clone(), // distinction_id = content hash
                    entry.prev_hash.clone(),  // previous version
                    VectorClock::new(),       // Initialize empty vector clock
                );

                // Store in storage using direct insert to preserve original IDs
                let _ = storage.insert_direct(&entry.ns, &entry.key, versioned);
            } else {
                eprintln!("Warning: Value not found for hash {}", entry.value_hash);
            }
        }
    }

    Ok(())
}

/// Lock file for preventing concurrent database access and detecting unclean shutdown.
const LOCK_FILE: &str = ".lock";

/// Lock file contents indicating clean/unclean state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockState {
    /// Database is currently running.
    Running,
    /// Database was shut down cleanly.
    Clean,
    /// Unclean shutdown detected.
    Unclean,
}

/// Acquire a lock on the database.
///
/// Returns:
/// - `Ok(LockState::Clean)` if this is a fresh start
/// - `Ok(LockState::Unclean)` if previous shutdown was unclean (recovery needed)
/// - `Err(...)` if another process is currently holding the lock
pub async fn acquire_lock(db_path: &Path) -> DeltaResult<LockState> {
    let lock_path = db_path.join(LOCK_FILE);

    // Check if lock file exists
    if lock_path.exists() {
        let content = fs::read_to_string(&lock_path)
            .await
            .map_err(|e| DeltaError::StorageError(format!("Failed to read lock file: {}", e)))?;

        match content.trim() {
            "RUNNING" => {
                return Err(DeltaError::StorageError(
                    "Database is already running (lock file exists). \
                     If this is incorrect, remove the lock file manually."
                        .to_string(),
                ));
            }
            "CLEAN" => {
                // Clean shutdown, proceed
            }
            _ => {
                // Unclean shutdown detected (or unknown state)
                // Write RUNNING state
                fs::write(&lock_path, "RUNNING").await.map_err(|e| {
                    DeltaError::StorageError(format!("Failed to write lock file: {}", e))
                })?;
                return Ok(LockState::Unclean);
            }
        }
    }

    // Create lock file with RUNNING state
    fs::create_dir_all(db_path)
        .await
        .map_err(|e| DeltaError::StorageError(format!("Failed to create db directory: {}", e)))?;
    fs::write(&lock_path, "RUNNING")
        .await
        .map_err(|e| DeltaError::StorageError(format!("Failed to write lock file: {}", e)))?;

    Ok(LockState::Clean)
}

/// Mark the database as cleanly shut down.
pub async fn release_lock(db_path: &Path) -> DeltaResult<()> {
    let lock_path = db_path.join(LOCK_FILE);
    fs::write(&lock_path, "CLEAN")
        .await
        .map_err(|e| DeltaError::StorageError(format!("Failed to write lock file: {}", e)))?;
    Ok(())
}

/// Mark the database as having shut down uncleanly (for testing).
#[allow(dead_code)]
pub async fn mark_unclean_shutdown(db_path: &Path) -> DeltaResult<()> {
    let lock_path = db_path.join(LOCK_FILE);
    fs::write(&lock_path, "UNCLEAN")
        .await
        .map_err(|e| DeltaError::StorageError(format!("Failed to write lock file: {}", e)))?;
    Ok(())
}

/// Create a snapshot from current storage (for migration or compaction).
pub async fn create_snapshot(storage: &CausalStorage, snapshot_path: &Path) -> DeltaResult<()> {
    fs::create_dir_all(snapshot_path.parent().unwrap_or(Path::new(".")))
        .await
        .map_err(|e| DeltaError::StorageError(format!("Failed to create snapshot dir: {}", e)))?;

    let (current_state, history_log) = storage.create_snapshot();

    #[derive(Serialize)]
    struct Snapshot {
        version: u32,
        current_state: Vec<(FullKey, Vec<u8>)>, // Serialized values
        history_log: Vec<(FullKey, Vec<Vec<u8>>)>,
    }

    let current: Vec<_> = current_state
        .into_iter()
        .map(|(k, v)| {
            let bytes = serde_json::to_vec(&v)?;
            Ok((k, bytes))
        })
        .collect::<DeltaResult<Vec<_>>>()?;

    let history: Vec<_> = history_log
        .into_iter()
        .map(|(k, versions)| {
            let bytes: Vec<_> = versions
                .into_iter()
                .map(|v| serde_json::to_vec(&v))
                .collect::<Result<Vec<_>, _>>()?;
            Ok((k, bytes))
        })
        .collect::<DeltaResult<Vec<_>>>()?;

    let snapshot = Snapshot {
        version: 1,
        current_state: current,
        history_log: history,
    };

    let temp_path = snapshot_path.with_extension("tmp");
    let bytes = serde_json::to_vec(&snapshot)?;
    fs::write(&temp_path, &bytes)
        .await
        .map_err(|e| DeltaError::StorageError(format!("Failed to write snapshot: {}", e)))?;
    fs::rename(&temp_path, snapshot_path)
        .await
        .map_err(|e| DeltaError::StorageError(format!("Failed to rename snapshot: {}", e)))?;

    Ok(())
}

/// Check if a database exists at the given path.
///
/// This checks for either:
/// - New WAL format (db_path/wal/ directory exists)
/// - Legacy format (file exists)
pub async fn exists(path: &Path) -> bool {
    // Check for new WAL format
    if let Some(parent) = path.parent() {
        let wal_dir = parent.join("wal");
        if fs::try_exists(&wal_dir).await.unwrap_or(false) {
            return true;
        }
    }

    // Check for legacy format
    fs::try_exists(path).await.unwrap_or(false)
}

/// Save database to disk using WAL format.
///
/// The `path` should be a directory where the WAL and value store will be created.
/// For example: `~/.korudelta/db/`
pub async fn save(storage: &CausalStorage, path: &Path) -> DeltaResult<()> {
    // Ensure the directory exists
    fs::create_dir_all(path)
        .await
        .map_err(|e| DeltaError::StorageError(format!("Failed to create db dir: {}", e)))?;

    // Get all history and write to WAL (preserves full history)
    let (_current_state, history_log) = storage.create_snapshot();

    // Write all historical versions to WAL (in chronological order)
    for (full_key, versions) in history_log {
        for versioned in versions {
            append_write(path, &full_key.namespace, &full_key.key, &versioned).await?;
        }
    }

    Ok(())
}

/// Load database from disk using WAL format.
///
/// The `path` should be a directory containing the WAL and value store.
/// For example: `~/.korudelta/db/`
pub async fn load(path: &Path, engine: Arc<DistinctionEngine>) -> DeltaResult<CausalStorage> {
    // If path doesn't exist, return empty storage
    if !fs::try_exists(path).await.unwrap_or(false) {
        return Ok(CausalStorage::new(engine));
    }

    // Check if this looks like a WAL directory
    let wal_dir = path.join("wal");
    let is_wal = fs::try_exists(&wal_dir).await.unwrap_or(false);

    if is_wal {
        // Load from WAL format
        return load_from_wal(path, engine).await;
    }

    // Check if it's a legacy snapshot file
    if let Ok(metadata) = fs::metadata(path).await {
        if metadata.is_file() {
            // Fall back to legacy snapshot format
            let bytes = fs::read(path).await.map_err(|e| {
                DeltaError::StorageError(format!("Failed to read database file: {}", e))
            })?;

            #[derive(Deserialize)]
            struct LegacySnapshot {
                #[allow(dead_code)]
                version: u32,
                current_state: Vec<(FullKey, VersionedValue)>,
                #[allow(dead_code)]
                history_log: Vec<(FullKey, Vec<VersionedValue>)>,
            }

            let snapshot: LegacySnapshot = serde_json::from_slice(&bytes).map_err(|e| {
                DeltaError::StorageError(format!("Failed to deserialize database: {}", e))
            })?;

            let storage = CausalStorage::new(engine);

            // Restore current state
            for (full_key, versioned) in snapshot.current_state {
                let _ = storage.put(
                    &full_key.namespace,
                    &full_key.key,
                    versioned.value().clone(),
                );
            }

            return Ok(storage);
        }
    }

    // If directory exists but is empty (no WAL yet), return empty storage
    Ok(CausalStorage::new(engine))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_content_addressed_storage() {
        let temp_dir = TempDir::new().unwrap();
        let values_dir = temp_dir.path().join("values");
        fs::create_dir_all(&values_dir).await.unwrap();

        let value = json!({"name": "Alice", "age": 30});
        let hash = "abc123def456";

        // Store value
        store_value(&values_dir, hash, &value).await.unwrap();

        // Load value
        let loaded = load_value(&values_dir, hash).await.unwrap().unwrap();
        assert_eq!(loaded, value);

        // Verify file structure: values/ab/c123def456
        let expected_path = values_dir.join("ab").join("c123def456");
        assert!(expected_path.exists());
    }

    /// Calculate total disk usage of the database in bytes.
    #[allow(dead_code)]
    pub async fn get_disk_usage(db_path: &Path) -> DeltaResult<u64> {
        let mut total_size = 0u64;

        // Walk the directory tree
        if db_path.exists() {
            let mut entries = fs::read_dir(db_path)
                .await
                .map_err(|e| DeltaError::StorageError(format!("Failed to read db dir: {}", e)))?;

            while let Some(entry) = entries
                .next_entry()
                .await
                .map_err(|e| DeltaError::StorageError(format!("Failed to read entry: {}", e)))?
            {
                let path = entry.path();
                let metadata = entry.metadata().await.map_err(|e| {
                    DeltaError::StorageError(format!("Failed to read metadata: {}", e))
                })?;

                if metadata.is_file() {
                    total_size += metadata.len();
                } else if metadata.is_dir() {
                    // Recursively calculate subdirectory size
                    total_size += get_dir_size(&path).await?;
                }
            }
        }

        Ok(total_size)
    }

    /// Helper to recursively calculate directory size.
    #[allow(dead_code)]
    async fn get_dir_size(dir: &Path) -> DeltaResult<u64> {
        let mut total_size = 0u64;

        let mut entries = fs::read_dir(dir)
            .await
            .map_err(|e| DeltaError::StorageError(format!("Failed to read dir: {}", e)))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| DeltaError::StorageError(format!("Failed to read entry: {}", e)))?
        {
            let metadata = entry
                .metadata()
                .await
                .map_err(|e| DeltaError::StorageError(format!("Failed to read metadata: {}", e)))?;

            if metadata.is_file() {
                total_size += metadata.len();
            } else if metadata.is_dir() {
                total_size += Box::pin(get_dir_size(&entry.path())).await?;
            }
        }

        Ok(total_size)
    }

    #[tokio::test]
    async fn test_append_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("db");

        let engine = Arc::new(DistinctionEngine::new());
        let versioned = VersionedValue::new(
            Arc::new(json!({"test": "value"})),
            Utc::now(),
            "hash123".to_string(), // write_id
            "hash123".to_string(), // distinction_id (same for test)
            None,
            VectorClock::new(), // Initialize empty vector clock
        );

        // Append write
        append_write(&db_path, "test", "key", &versioned)
            .await
            .unwrap();

        // Load
        let storage = load_from_wal(&db_path, engine).await.unwrap();
        let keys = storage.list_keys("test");
        assert_eq!(keys.len(), 1);
    }
}
