# KoruDelta API Reference

Complete API documentation for KoruDelta v2.0.0.

---

## Table of Contents

- [Rust API](#rust-api)
  - [KoruDelta](#korudelta)
  - [Database Operations](#database-operations)
  - [Vector Operations](#vector-operations)
  - [Query Operations](#query-operations)
  - [Lifecycle Management](#lifecycle-management)
- [Python API](#python-api)
  - [Database](#database)
  - [Vector Search](#vector-search)
  - [Integrations](#integrations)
- [CLI Reference](#cli-reference)
- [Error Handling](#error-handling)
- [Type Definitions](#type-definitions)

---

## Rust API

### KoruDelta

Main database struct. Entry point for all operations.

```rust
use koru_delta::KoruDelta;

let db = KoruDelta::start().await?;
```

#### Methods

##### `start()` → `Result<Self, DeltaError>`

Start an in-memory database with default configuration.

```rust
let db = KoruDelta::start().await?;
```

##### `start_with_config(config: Config)` → `Result<Self, DeltaError>`

Start database with custom configuration.

```rust
let config = Config::new()
    .with_persistence("./data")
    .with_memory_limit(1024 * 1024 * 1024);  // 1GB

let db = KoruDelta::start_with_config(config).await?;
```

##### `shutdown(self)` → `Result<(), DeltaError>`

Gracefully shutdown the database.

```rust
db.shutdown().await?;
```

---

### Database Operations

#### `put(namespace: &str, key: &str, value: Value)` → `Result<VersionedValue, DeltaError>`

Store a value. Creates a new version if key exists.

**Parameters:**
- `namespace`: Logical grouping (e.g., "users", "config")
- `key`: Unique identifier within namespace
- `value`: Any JSON-serializable value

**Returns:** The versioned value that was stored.

**Example:**
```rust
let result = db.put("users", "alice", json!({
    "name": "Alice",
    "email": "alice@example.com"
})).await?;

println!("Version: {}", result.version_id);
```

---

#### `get(namespace: &str, key: &str)` → `Result<Value, DeltaError>`

Retrieve the current value for a key.

**Parameters:**
- `namespace`: Logical grouping
- `key`: Unique identifier

**Returns:** Current value.

**Errors:**
- `KeyNotFoundError`: If key doesn't exist

**Example:**
```rust
let user = db.get("users", "alice").await?;
println!("Name: {}", user["name"]);
```

---

#### `get_at(namespace: &str, key: &str, timestamp: &str)` → `Result<Value, DeltaError>`

Retrieve value at a specific point in time (time-travel query).

**Parameters:**
- `namespace`: Logical grouping
- `key`: Unique identifier
- `timestamp`: ISO 8601 timestamp (e.g., "2026-02-07T12:00:00Z")

**Returns:** Value as it existed at that timestamp.

**Errors:**
- `KeyNotFoundError`: If key didn't exist at that time

**Example:**
```rust
let user = db.get_at("users", "alice", "2026-01-01T00:00:00Z").await?;
println!("User on Jan 1: {:?}", user);
```

---

#### `history(namespace: &str, key: &str)` → `Result<Vec<HistoryEntry>, DeltaError>`

Get complete version history for a key.

**Parameters:**
- `namespace`: Logical grouping
- `key`: Unique identifier

**Returns:** Vector of history entries, oldest first.

**Example:**
```rust
let history = db.history("users", "alice").await?;
for entry in history {
    println!("{}: {:?}", entry.timestamp, entry.value);
}
```

---

#### `delete(namespace: &str, key: &str)` → `Result<VersionedValue, DeltaError>`

Delete a key (stores tombstone, history preserved).

**Parameters:**
- `namespace`: Logical grouping
- `key`: Unique identifier

**Returns:** Tombstone version.

**Example:**
```rust
db.delete("users", "alice").await?;
// Key is now deleted, but history remains
```

---

#### `contains(namespace: &str, key: &str)` → `Result<bool, DeltaError>`

Check if key exists (and is not deleted).

**Parameters:**
- `namespace`: Logical grouping
- `key`: Unique identifier

**Returns:** True if key exists.

**Example:**
```rust
if db.contains("users", "alice").await? {
    println!("User exists");
}
```

---

#### `list_keys(namespace: &str)` → `Result<Vec<String>, DeltaError>`

List all keys in a namespace.

**Parameters:**
- `namespace`: Logical grouping

**Returns:** Vector of key strings.

**Example:**
```rust
let keys = db.list_keys("users").await?;
println!("Users: {:?}", keys);
```

---

#### `stats()` → `Result<DatabaseStats, DeltaError>`

Get database statistics.

**Returns:** Statistics about database state.

**Example:**
```rust
let stats = db.stats().await?;
println!("Keys: {}", stats.key_count);
println!("Namespaces: {}", stats.namespace_count);
```

---

### Vector Operations

#### `embed(namespace: &str, key: &str, embedding: Vec<f32>, model: &str, metadata: Option<Value>)` → `Result<VectorEmbedding, DeltaError>`

Store a vector embedding.

**Parameters:**
- `namespace`: Logical grouping
- `key`: Unique identifier
- `embedding`: Vector of f32 values
- `model`: Embedding model identifier (e.g., "text-embedding-3-small")
- `metadata`: Optional metadata dict

**Returns:** The stored embedding.

**Example:**
```rust
let embedding = db.embed(
    "documents",
    "doc1",
    vec![0.1, 0.2, 0.3, /* ... 1536 dims */],
    "text-embedding-3-small",
    Some(json!({
        "title": "Introduction",
        "author": "Alice"
    }))
).await?;
```

---

#### `similar(namespace: Option<&str>, query: &[f32], top_k: usize, threshold: f32, model_filter: Option<&str>)` → `Result<Vec<SimilarityResult>, DeltaError>`

Search for similar vectors.

**Parameters:**
- `namespace`: Namespace to search (None = all namespaces)
- `query`: Query vector
- `top_k`: Maximum results to return (default: 10)
- `threshold`: Minimum similarity score 0.0-1.0 (default: 0.0)
- `model_filter`: Only return vectors from this model (optional)

**Returns:** Vector of similarity results.

**Example:**
```rust
let results = db.similar(
    Some("documents"),
    &query_embedding,
    5,           // top_k
    0.8,         // threshold
    Some("text-embedding-3-small")
).await?;

for result in results {
    println!("{}: {:.3}", result.key, result.score);
}
```

---

#### `similar_at(namespace: &str, query: &[f32], timestamp: &str, top_k: usize)` → `Result<Vec<SimilarityResult>, DeltaError>`

Search for similar vectors at a specific point in time.

**Parameters:**
- `namespace`: Namespace to search
- `query`: Query vector
- `timestamp`: ISO 8601 timestamp
- `top_k`: Maximum results

**Returns:** Similarity results as of that timestamp.

**Example:**
```rust
let results = db.similar_at(
    "documents",
    &query,
    "2026-01-01T00:00:00Z",
    5
).await?;
```

---

### Query Operations

#### `query(namespace: &str)` → `QueryBuilder`

Build a filtered query.

**Parameters:**
- `namespace`: Namespace to query

**Returns:** QueryBuilder for constructing queries.

**Example:**
```rust
let results = db.query("users")
    .filter(Filter::new()
        .field("status").eq("active")
        .and_field("age").gte(18))
    .sort("created_at", SortOrder::Desc)
    .limit(100)
    .execute().await?;
```

---

#### QueryBuilder Methods

##### `filter(filter: Filter)` → `Self`

Add a filter condition.

##### `sort(field: &str, order: SortOrder)` → `Self`

Add sort criteria.

##### `limit(n: usize)` → `Self`

Limit number of results.

##### `offset(n: usize)` → `Self`

Skip first n results.

##### `execute()` → `Result<Vec<QueryResult>, DeltaError>`

Execute the query.

---

### Filter Operations

#### `Filter::new()` → `Filter`

Create a new filter.

**Example:**
```rust
let filter = Filter::new()
    .field("status").eq("active")
    .and_field("age").gte(18)
    .and_field("name").like("Alice%");
```

#### Filter Conditions

| Method | Description | Example |
|--------|-------------|---------|
| `eq(value)` | Equals | `.eq("active")` |
| `ne(value)` | Not equals | `.ne("deleted")` |
| `gt(value)` | Greater than | `.gt(100)` |
| `gte(value)` | Greater than or equal | `.gte(18)` |
| `lt(value)` | Less than | `.lt(1000)` |
| `lte(value)` | Less than or equal | `.lte(100)` |
| `in(values)` | In list | `.in(["a", "b", "c"])` |
| `like(pattern)` | Pattern match | `.like("Alice%")` |
| `exists()` | Field exists | `.exists()` |
| `is_null()` | Field is null | `.is_null()` |

#### Logical Operators

```rust
// AND
let filter = Filter::new()
    .field("status").eq("active")
    .and_field("age").gte(18);

// OR
let filter = Filter::new()
    .field("role").eq("admin")
    .or_field("role").eq("moderator");

// NOT
let filter = Filter::new()
    .not(Filter::new().field("status").eq("deleted"));
```

---

### Workspace Operations

#### `workspace(name: &str)` → `Workspace`

Get or create a workspace.

**Parameters:**
- `name`: Workspace name (isolation boundary)

**Returns:** Workspace instance.

**Example:**
```rust
let workspace = db.workspace("audit-2026");

// Store in workspace
workspace.store("tx-123", data, MemoryPattern::Event).await?;

// Query workspace
let history = workspace.history("tx-123").await?;
```

---

### Agent Memory Operations

#### `agent_memory(agent_id: &str)` → `AgentMemory`

Create agent memory interface (Python only in v2.0, Rust in v2.1).

**Parameters:**
- `agent_id`: Unique agent identifier

**Returns:** AgentMemory instance.

---

## Python API

### Database

```python
from koru_delta import Database

async with Database() as db:
    # Operations here
    pass
```

#### `__init__(config: Config | None = None)`

Initialize database connection.

**Parameters:**
- `config`: Optional configuration. Defaults to in-memory.

**Example:**
```python
from koru_delta import Database, Config

# In-memory (default)
async with Database() as db:
    pass

# With persistence
config = Config(path="~/.myapp/db")
async with Database(config) as db:
    pass
```

---

#### `put(namespace: str, key: str, value: Any) -> None`

Store a value.

**Parameters:**
- `namespace`: Logical grouping
- `key`: Unique identifier
- `value`: Any JSON-serializable Python object

**Example:**
```python
await db.put("users", "alice", {
    "name": "Alice",
    "email": "alice@example.com"
})
```

---

#### `get(namespace: str, key: str) -> Any`

Retrieve current value.

**Parameters:**
- `namespace`: Logical grouping
- `key`: Unique identifier

**Returns:** The stored value.

**Raises:**
- `KeyNotFoundError`: If key doesn't exist

**Example:**
```python
user = await db.get("users", "alice")
print(user["name"])  # "Alice"
```

---

#### `get_at(namespace: str, key: str, timestamp: str) -> Any`

Retrieve value at specific point in time.

**Parameters:**
- `namespace`: Logical grouping
- `key`: Unique identifier
- `timestamp`: ISO 8601 timestamp

**Example:**
```python
from datetime import datetime, timedelta

one_hour_ago = (datetime.utcnow() - timedelta(hours=1)).isoformat()
old_value = await db.get_at("config", "version", one_hour_ago)
```

---

#### `history(namespace: str, key: str) -> List[Dict]`

Get complete history for a key.

**Returns:** List of history entries, each with:
- `value`: The value at that point
- `timestamp`: When it was stored
- `version_id`: Unique version identifier

**Example:**
```python
history = await db.history("config", "model")
for entry in history:
    print(f"{entry['timestamp']}: {entry['value']}")
```

---

#### `delete(namespace: str, key: str) -> None`

Delete a key (stores tombstone).

**Example:**
```python
await db.delete("users", "alice")
# History is preserved
```

---

#### `contains(namespace: str, key: str) -> bool`

Check if key exists.

---

#### `list_keys(namespace: str) -> List[str]`

List all keys in namespace.

---

#### `embed(namespace: str, key: str, embedding: List[float], model: str, metadata: Any = None) -> None`

Store vector embedding.

**Parameters:**
- `namespace`: Logical grouping
- `key`: Unique identifier
- `embedding`: List of floats (or numpy array)
- `model`: Embedding model name
- `metadata`: Optional metadata dict

**Example:**
```python
import numpy as np

await db.embed(
    "documents", "doc1",
    embedding=[0.1, 0.2, 0.3, ...],  # or np.array([...])
    model="text-embedding-3-small",
    metadata={"title": "AI Paper"}
)
```

---

#### `similar(namespace: str | None, query: List[float], top_k: int = 10, threshold: float = 0.0, model_filter: str | None = None) -> List[Dict]`

Search for similar vectors.

**Parameters:**
- `namespace`: Namespace to search (None = search all)
- `query`: Query vector
- `top_k`: Maximum results (default: 10)
- `threshold`: Minimum similarity 0.0-1.0 (default: 0.0)
- `model_filter`: Only return vectors from this model

**Returns:** List of results with:
- `namespace`: Where found
- `key`: Vector identifier
- `score`: Similarity score

**Example:**
```python
results = await db.similar(
    "documents",
    query=[0.1, 0.2, 0.3, ...],
    top_k=5,
    threshold=0.8
)

for r in results:
    print(f"{r['key']}: {r['score']:.2f}")
```

---

#### `stats() -> Dict`

Get database statistics.

**Returns:** Dict with:
- `key_count`: Total keys
- `namespace_count`: Number of namespaces

---

#### `agent_memory(agent_id: str) -> AgentMemory`

Create agent memory interface.

**Example:**
```python
memory = db.agent_memory("assistant-42")
await memory.episodes.remember("User asked about Python")
```

---

## Python Integrations

### Document Chunking

```python
from koru_delta import chunk_document, ChunkingConfig
```

#### `chunk_document(text: str, config: ChunkingConfig | None = None) -> List[str]`

Split document into chunks.

**Parameters:**
- `text`: Document text to chunk
- `config`: Chunking configuration

**Returns:** List of text chunks.

**Example:**
```python
chunks = chunk_document(long_text, ChunkingConfig(
    chunk_size=1000,
    chunk_overlap=100
))
```

---

#### `ChunkingConfig`

Configuration for document chunking.

**Parameters:**
- `chunk_size`: Target chunk size in characters (default: 1000)
- `chunk_overlap`: Characters to overlap (default: 100)
- `separators`: List of separators in order of preference
- `keep_separator`: Whether to keep separator (default: True)
- `strip_whitespace`: Whether to strip whitespace (default: True)

---

### Hybrid Search

```python
from koru_delta import HybridSearcher, CausalFilter
```

#### `HybridSearcher`

Combine vector similarity with causal filters.

**Example:**
```python
searcher = HybridSearcher(db)

results = await searcher.search(
    query_vector=embedding,
    namespace="documents",
    causal_filter=CausalFilter(
        after_timestamp="2026-01-01T00:00:00Z"
    ),
    vector_weight=0.7,
    causal_weight=0.3
)
```

---

#### `CausalFilter`

Filter based on causal/temporal properties.

**Parameters:**
- `after_timestamp`: Only include entries after this time
- `before_timestamp`: Only include entries before this time
- `min_version_count`: Minimum number of versions
- `related_to_key`: Related to this (namespace, key) tuple
- `custom_filter`: Custom predicate function

---

### LangChain Integration

```python
from koru_delta.integrations.langchain import KoruDeltaVectorStore
```

#### `KoruDeltaVectorStore`

LangChain VectorStore implementation.

**Parameters:**
- `db`: Database instance
- `namespace`: Namespace for storage
- `embedding_model`: LangChain embeddings model
- `model_name`: Model identifier

**Methods:**
- `add_texts(texts, metadatas=None)` → List of IDs
- `add_documents(documents)` → List of IDs
- `similarity_search(query, k=4)` → List of Documents
- `similarity_search_by_vector(embedding, k=4)` → List of Documents
- `max_marginal_relevance_search(query, k=4, fetch_k=20, lambda_mult=0.5)` → List of Documents

**Example:**
```python
from langchain_openai import OpenAIEmbeddings

store = KoruDeltaVectorStore(
    db=db,
    namespace="docs",
    embedding_model=OpenAIEmbeddings()
)

# Add documents
store.add_texts(["Hello world", "AI is amazing"])

# Search
docs = store.similarity_search("hello", k=3)
```

---

### LlamaIndex Integration

```python
from koru_delta.integrations.llamaindex import KoruDeltaVectorStore
```

#### `KoruDeltaVectorStore`

LlamaIndex storage backend.

**Parameters:**
- `db`: Database instance
- `namespace`: Namespace for storage
- `model_name`: Model identifier

**Methods:**
- `add(nodes)` → List of IDs
- `query(query)` → VectorStoreQueryResult
- `delete(ref_doc_id)` → None
- `time_travel_query(query, timestamp)` → VectorStoreQueryResult
- `get_node_history(node_id)` → List of versions

**Example:**
```python
from llama_index.core import VectorStoreIndex

vector_store = KoruDeltaVectorStore(db=db, namespace="llama_docs")
index = VectorStoreIndex.from_vector_store(vector_store)
```

---

## CLI Reference

### Global Options

```bash
kdelta [options] <command>
```

**Options:**
- `--url <url>`: Connect to remote server (default: local)
- `--db <path>`: Database path (default: ~/.korudelta/db)
- `--format <format>`: Output format: json, yaml, table (default: table)
- `--help`: Show help
- `--version`: Show version

---

### Commands

#### `kdelta put <namespace/key> <value>`

Store a value.

```bash
kdelta put users/alice '{"name": "Alice", "age": 30}'
kdelta put config/version '"2.0.0"'  # Note: JSON strings need quotes
```

---

#### `kdelta get <namespace/key>`

Retrieve a value.

```bash
kdelta get users/alice
kdelta get --at "2026-01-01T00:00:00Z" users/alice
```

---

#### `kdelta history <namespace/key>`

Show version history.

```bash
kdelta history users/alice
kdelta history --limit 10 users/alice
```

---

#### `kdelta delete <namespace/key>`

Delete a key.

```bash
kdelta delete users/alice
```

---

#### `kdelta list <namespace>`

List keys in namespace.

```bash
kdelta list users
kdelta list --pattern "alice*" users
```

---

#### `kdelta query <namespace>`

Query with filters.

```bash
kdelta query users --filter 'status=active' --sort 'created_at:desc'
```

---

#### `kdelta embed <namespace/key> <model> <vector...>`

Store embedding.

```bash
kdelta embed docs/doc1 text-embedding-3-small 0.1 0.2 0.3 ...
```

---

#### `kdelta similar <namespace> <query...>`

Search similar vectors.

```bash
kdelta similar docs 0.1 0.2 0.3 ... --top-k 5
```

---

#### `kdelta start`

Start server mode.

```bash
kdelta start --port 7878 --host 0.0.0.0
```

---

#### `kdelta stats`

Show database statistics.

```bash
kdelta stats
```

---

## Error Handling

### Rust Error Types

```rust
pub enum DeltaError {
    // Key errors
    KeyNotFound { namespace: String, key: String },
    KeyAlreadyExists { namespace: String, key: String },
    
    // Serialization errors
    SerializationError(String),
    DeserializationError(String),
    
    // Validation errors
    InvalidData { field: String, reason: String },
    InvalidNamespace(String),
    InvalidKey(String),
    
    // Storage errors
    StorageError(String),
    PersistenceError(String),
    
    // Network errors (cluster mode)
    NetworkError(String),
    
    // Other
    InternalError(String),
    NotImplemented(String),
}
```

### Python Exceptions

```python
from koru_delta import (
    KoruDeltaError,      # Base exception
    KeyNotFoundError,    # Key doesn't exist
    SerializationError,  # JSON serialization failed
    ValidationError,     # Invalid input
    DatabaseClosedError, # Database not initialized
)

async with Database() as db:
    try:
        value = await db.get("users", "unknown")
    except KeyNotFoundError as e:
        print(f"Key not found: {e}")
    except KoruDeltaError as e:
        print(f"Database error: {e}")
```

---

## Type Definitions

### Rust Types

```rust
// Core value type
pub type Value = serde_json::Value;

// Versioned value
pub struct VersionedValue {
    pub namespace: String,
    pub key: String,
    pub value: Value,
    pub timestamp: DateTime<Utc>,
    pub version_id: String,
    pub previous_version: Option<String>,
}

// History entry
pub struct HistoryEntry {
    pub value: Value,
    pub timestamp: String,
    pub version_id: String,
    pub previous_version: Option<String>,
}

// Vector embedding
pub struct VectorEmbedding {
    pub namespace: String,
    pub key: String,
    pub embedding: Arc<[f32]>,
    pub model: String,
    pub metadata: Option<Value>,
    pub timestamp: DateTime<Utc>,
}

// Similarity result
pub struct SimilarityResult {
    pub namespace: String,
    pub key: String,
    pub score: f32,
    pub embedding: Arc<[f32]>,
}

// Query result
pub struct QueryResult {
    pub namespace: String,
    pub key: String,
    pub value: Value,
    pub version_info: VersionInfo,
}

// Database stats
pub struct DatabaseStats {
    pub key_count: usize,
    pub namespace_count: usize,
    pub vector_count: usize,
    pub memory_usage_bytes: usize,
    pub disk_usage_bytes: usize,
}
```

### Python Types

```python
from typing import Any, Dict, List, Optional, Union
import numpy as np

# Config
class Config:
    path: Optional[str]
    memory_limit: Optional[int]
    persistence: bool

# History entry
HistoryEntry = Dict[str, Any]  # {
    # "value": Any,
    # "timestamp": str,
    # "version_id": str,
    # "previous_version": Optional[str]
# }

# Similarity result
SimilarityResult = Dict[str, Any]  # {
    # "namespace": str,
    # "key": str,
    # "score": float
# }

# Database stats
DatabaseStats = Dict[str, int]  # {
    # "key_count": int,
    # "namespace_count": int
# }

# Embedding input
EmbeddingInput = Union[List[float], np.ndarray]
```

---

## Performance Characteristics

| Operation | Latency | Throughput | Notes |
|-----------|---------|------------|-------|
| `get()` | ~400ns | ~2.5M ops/sec | Hot tier |
| `put()` | ~50µs | ~20K ops/sec | With versioning |
| `get_at()` | ~1ms | ~1K ops/sec | Historical query |
| `similar()` | ~2.5µs | ~400K queries/sec | SNSW index, 5K vectors |
| `history()` | ~1ms | N/A | Depends on history size |
| Query (filter) | ~1-10ms | N/A | Depends on filter complexity |

---

## Version Compatibility

| API Version | Status | Rust | Python |
|-------------|--------|------|--------|
| v2.0.0 | Stable | ✅ | ✅ |
| v1.x | Deprecated | ⚠️ | ❌ |

---

For more information, see:
- [The Causal Database Guide](./THE_CAUSAL_DATABASE.md)
- [Architecture Documentation](../ARCHITECTURE.md)
- [Python Examples](../bindings/python/examples/)
