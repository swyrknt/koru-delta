# KoruDelta Python API Design

**Goal:** Zero-friction Python API that feels native, not like a Rust library wrapped in Python

**Target Users:**
1. AI/ML engineers building agents
2. Data scientists doing RAG
3. Edge developers deploying on-device
4. Researchers experimenting with memory systems

---

## Core Principles

1. **Pythonic** - `async for`, context managers, duck typing
2. **Zero Boilerplate** - Sensible defaults, minimal setup
3. **Ecosystem Integration** - NumPy, Pandas, OpenAI SDK compatibility
4. **Type Safe** - Full type hints for IDE autocomplete
5. **Clear Errors** - Actionable error messages
6. **Batteries Included** - Common patterns built-in

---

## API Design: Iterations

### ❌ Bad: C-style API
```python
import koru_delta

db = koru_delta.Database()
result = db.put_bytes(b"namespace", b"key", json_data)
if result.status != 0:
    raise Exception("Error")
```

### ❌ Okay: Direct Rust Translation
```python
from koru_delta import KoruDelta
import asyncio

db = KoruDelta()
asyncio.run(db.start())
value = asyncio.run(db.put("ns", "key", {"data": "value"}))
```

### ✅ Good: Pythonic, Async-Native
```python
from koru_delta import Database

async with Database() as db:
    await db.put("users", "alice", {"name": "Alice"})
    user = await db.get("users", "alice")
```

### ✨ Excellent: Intent-Revealing
```python
from koru_delta import Database

# Agent memory - the "aha" use case
async with Database.for_agent("assistant-42") as memory:
    # Remember automatically embeds and stores
    await memory.remember("User prefers Python examples")
    
    # Recall finds semantically similar memories
    results = await memory.recall("What does user prefer?")
    # Returns: ["User prefers Python examples", ...]
```

---

## Final API Design

### 1. Core Database API

```python
from koru_delta import Database

# Zero-config, in-memory
async with Database() as db:
    # Store anything JSON-serializable
    await db["users/alice"] = {"name": "Alice", "tags": ["vip"]}
    
    # Retrieve
    user = await db["users/alice"]
    
    # Namespace-scoped access
    users = db.namespace("users")
    await users.put("bob", {"name": "Bob"})
    all_users = await users.all()
```

**Key design decisions:**
- `async with` for automatic lifecycle management
- Dict-like `db[key]` syntax for familiarity
- Namespaces as first-class objects
- No separate `start()`/`stop()` calls

### 2. Agent Memory API

```python
from koru_delta import AgentMemory

async with AgentMemory(agent_id="assistant-42") as memory:
    # Episodic memory (specific events)
    await memory.episodes.remember(
        "User asked about Python bindings",
        importance=0.8,
        tags=["python", "bindings"]
    )
    
    # Semantic memory (facts)
    await memory.facts.learn(
        "python_bindings",
        content="KoruDelta has Python bindings via PyO3",
        tags=["tech", "python"]
    )
    
    # Procedural memory (how-to)
    await memory.procedures.learn(
        "handle_error",
        steps=["1. Log error", "2. Notify user", "3. Retry"],
        success_rate=0.95
    )
    
    # Recall - automatically searches all memory types
    results = await memory.recall(
        "What did user ask about?",
        limit=5,
        min_relevance=0.7
    )
    
    for result in results:
        print(f"{result.relevance:.2f}: {result.content}")
        print(f"   Type: {result.memory_type}, Accessed: {result.access_count} times")
```

**Key design decisions:**
- Separate accessors for memory types (`memory.episodes`, `memory.facts`)
- Semantic method names (`remember` vs `learn` vs `store`)
- Automatic relevance scoring
- Rich result objects with metadata

### 3. Vector / Embedding API

```python
from koru_delta import Database
import numpy as np

async with Database() as db:
    # Store with embedding (automatic if you provide embedding)
    await db.embed(
        "documents", "doc1",
        content={"title": "AI Paper", "text": "..."},
        embedding=[0.1, 0.2, 0.3, ...],  # or numpy array
        model="text-embedding-3-small"
    )
    
    # Search by vector
    results = await db.similar(
        "documents",
        query=[0.1, 0.2, 0.3, ...],
        top_k=5,
        threshold=0.8
    )
    
    # Or search by text (if you provide embed_fn)
    from openai import AsyncOpenAI
    client = AsyncOpenAI()
    
    async def embed_text(text: str) -> list[float]:
        response = await client.embeddings.create(
            model="text-embedding-3-small",
            input=text
        )
        return response.data[0].embedding
    
    results = await db.similar_by_text(
        "documents",
        query="AI and machine learning",
        embed_fn=embed_text,
        top_k=5
    )
```

**Key design decisions:**
- Accept both lists and numpy arrays
- Explicit `embed()` for storing, `similar()` for searching
- Optional `embed_fn` for text-to-vector conversion
- Returns rich results with scores and metadata

### 4. RAG Pipeline API

```python
from koru_delta import RAG, Document
from openai import AsyncOpenAI

client = AsyncOpenAI()

async def embed_fn(text: str) -> list[float]:
    response = await client.embeddings.create(
        model="text-embedding-3-small",
        input=text
    )
    return response.data[0].embedding

async with RAG(
    embed_fn=embed_fn,
    chunk_size=512,
    overlap=128
) as rag:
    # Add documents
    await rag.add_documents([
        Document(id="doc1", text="Long document text here...", metadata={"source": "web"}),
        Document(id="doc2", text="Another document...", metadata={"source": "pdf"}),
    ])
    
    # Query
    context = await rag.query(
        "What are the key findings?",
        top_k=3,
        filters={"source": "web"}  # Optional metadata filter
    )
    
    # Use with LLM
    response = await client.chat.completions.create(
        model="gpt-4",
        messages=[
            {"role": "system", "content": "Answer based on context."},
            {"role": "user", "content": f"Context: {context}\n\nQuestion: What are the key findings?"}
        ]
    )
```

**Key design decisions:**
- High-level `RAG` class that orchestrates everything
- Automatic chunking with configurable size/overlap
- Pluggable `embed_fn` (use OpenAI, local models, etc.)
- Context returns formatted string ready for LLM prompt

### 5. Time Travel / Versioning API

```python
from koru_delta import Database
from datetime import datetime, timedelta

async with Database() as db:
    # Store multiple versions
    await db.put("config", "model", {"version": "gpt-3.5"})
    await asyncio.sleep(1)
    await db.put("config", "model", {"version": "gpt-4"})
    
    # Get current
    current = await db.get("config", "model")
    # {"version": "gpt-4"}
    
    # Get at specific time
    one_hour_ago = datetime.utcnow() - timedelta(hours=1)
    old = await db.get_at("config", "model", one_hour_ago)
    # {"version": "gpt-3.5"}
    
    # Get history
    history = await db.history("config", "model")
    for entry in history:
        print(f"{entry.timestamp}: {entry.value}")
    
    # Diff between versions
    diff = await db.diff("config", "model", v1="hash1", v2="hash2")
    # Shows what changed between versions
```

**Key design decisions:**
- `get_at()` for time-travel queries
- `history()` returns full audit trail
- `diff()` for comparing versions
- Familiar datetime objects

### 6. Persistence & Configuration

```python
from koru_delta import Database, Config

# With persistence
config = Config(
    path="~/.myapp/db",  # Auto-expands to absolute path
    max_memory_mb=512,
    enable_wal=True,  # Write-ahead logging for durability
)

async with Database(config) as db:
    # Data persists across restarts
    await db.put("state", "counter", 42)

# Later...
async with Database(config) as db:
    counter = await db.get("state", "counter")
    # 42 (restored from disk)
```

**Key design decisions:**
- `Config` object for all options
- Auto-creates directories
- WAL enabled by default for safety
- Path expansion (`~` → home directory)

### 7. Observability & Debugging

```python
from koru_delta import Database

async with Database() as db:
    # Stats
    stats = await db.stats()
    print(f"Keys: {stats.key_count}")
    print(f"Memory: {stats.memory_usage_mb:.1f} MB")
    print(f"Hot tier: {stats.hot_cache_size}")
    
    # Health check
    health = await db.health()
    if not health.healthy:
        print(f"Issues: {health.issues}")
    
    # Query memory tier breakdown
    tiers = await db.memory_tiers()
    print(f"Hot: {tiers.hot}, Warm: {tiers.warm}, Cold: {tiers.cold}")
```

---

## Error Handling

All errors are subclasses of `KoruDeltaError` with actionable messages:

```python
from koru_delta import Database, KoruDeltaError, KeyNotFoundError

async with Database() as db:
    try:
        value = await db.get("users", "nonexistent")
    except KeyNotFoundError as e:
        print(f"Key not found: {e.key}")
        # Suggests: "Did you mean 'alice'? Similar keys: ['alice', 'alex']"
    except KoruDeltaError as e:
        print(f"Database error: {e}")
        print(f"Suggestion: {e.suggestion}")  # Actionable fix
```

---

## Type Hints (Full IDE Support)

```python
from koru_delta import Database, MemoryRecall
from typing import Any

async def get_user(db: Database, user_id: str) -> dict[str, Any] | None:
    """Get user by ID."""
    return await db.get("users", user_id)

async def recall_facts(
    memory,
    query: str,
    limit: int = 10
) -> list[MemoryRecall]:
    """Recall facts from agent memory."""
    return await memory.recall(query, limit=limit)
```

---

## Installation & Quick Start

```bash
pip install koru-delta
```

```python
# 30-second "aha moment"
import asyncio
from koru_delta import Database

async def main():
    async with Database() as db:
        # Store
        await db["messages/1"] = {"role": "user", "content": "Hello!"}
        
        # Retrieve
        msg = await db["messages/1"]
        print(msg["content"])  # "Hello!"
        
        # History
        history = await db.history("messages", "1")
        print(f"Versions: {len(history)}")

asyncio.run(main())
```

---

## Implementation Notes

### Async Design
- All I/O operations are async (no blocking)
- Uses `pyo3-asyncio` for Tokio integration
- Supports both `asyncio` and `trio` (future)

### NumPy Integration
- Zero-copy where possible (Rust → Python memory sharing)
- Accepts `np.ndarray` for embeddings
- Returns NumPy arrays for vector data

### Serialization
- JSON by default (handles 99% of use cases)
- Custom serializers via `koru_delta.register_serializer()`
- Binary data via `bytes` (stored efficiently)

### Thread Safety
- Database instance is thread-safe
- Can be shared across threads
- Concurrent reads, serialized writes

---

## Success Criteria

User should be able to:
1. **Install and use in 5 minutes** - `pip install` → working code
2. **Understand the API from autocomplete** - No docs needed for basic usage
3. **Handle errors without docs** - Clear messages suggest fixes
4. **Integrate with existing stack** - Works with OpenAI, LangChain, etc.
5. **Deploy to production confidently** - Observability, persistence, stability

---

## Next Steps

1. Implement core `Database` class
2. Add `AgentMemory` wrapper
3. Create `RAG` high-level API
4. Write comprehensive examples
5. Benchmark vs. alternatives
6. Documentation site with interactive examples
