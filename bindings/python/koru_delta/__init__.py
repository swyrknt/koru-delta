"""
KoruDelta - The database for AI agents.

A zero-configuration, causal database with native support for:
- Vector embeddings and semantic search
- Agent memory (episodic, semantic, procedural)
- Time travel (query historical state)
- Edge deployment (8MB binary, runs anywhere)

Example:
    >>> import asyncio
    >>> from koru_delta import Database
    >>>
    >>> async def main():
    ...     async with Database() as db:
    ...         await db.put("users", "alice", {"name": "Alice"})
    ...         user = await db.get("users", "alice")
    ...         print(user["name"])
    ...
    >>> asyncio.run(main())
"""

from __future__ import annotations

__version__ = "2.0.0"

# Import from Rust extension
from koru_delta._internal import (
    Database as _RustDatabase,
    KoruDeltaError,
    KeyNotFoundError,
    SerializationError,
    ValidationError,
    DatabaseClosedError,
)

# Pure Python imports
from koru_delta.config import Config
from koru_delta.agent_memory import AgentMemory

__all__ = [
    "Database",
    "Config",
    "AgentMemory",
    "KoruDeltaError",
    "KeyNotFoundError",
    "SerializationError",
    "ValidationError",
    "DatabaseClosedError",
]


class Database:
    """
    KoruDelta database instance.
    
    This is the main entry point for interacting with the database.
    Use as an async context manager for automatic resource management.
    
    Args:
        config: Optional configuration. If not provided, uses defaults
               (in-memory, no persistence).
    
    Example:
        >>> # In-memory database (default)
        >>> async with Database() as db:
        ...     await db.put("users", "alice", {"name": "Alice"})
        ...     user = await db.get("users", "alice")
        ...     print(user["name"])  # "Alice"
        
        >>> # With persistence
        >>> from koru_delta import Config
        >>> config = Config(path="~/.myapp/db")
        >>> async with Database(config) as db:
        ...     # Data persists across restarts
        ...     pass
    """
    
    def __init__(self, config: Config | None = None):
        self._config = config or Config()
        self._db: _RustDatabase | None = None
    
    async def __aenter__(self) -> Database:
        """Async context manager entry."""
        self._db = await _RustDatabase.create()
        return self
    
    async def __aexit__(self, exc_type, exc_val, exc_tb) -> None:
        """Async context manager exit."""
        # Database cleanup happens automatically in Rust
        self._db = None
    
    def _check_initialized(self) -> _RustDatabase:
        """Check if database is initialized."""
        if self._db is None:
            raise DatabaseClosedError(
                "Database not initialized. Use 'async with Database() as db:'"
            )
        return self._db
    
    async def put(self, namespace: str, key: str, value: object) -> None:
        """
        Store a value.
        
        Args:
            namespace: Logical grouping (e.g., "users", "documents")
            key: Unique identifier within namespace
            value: Any JSON-serializable Python object
        
        Raises:
            ValidationError: If namespace or key is invalid
            SerializationError: If value cannot be serialized
        
        Example:
            >>> await db.put("users", "alice", {
            ...     "name": "Alice",
            ...     "email": "alice@example.com",
            ...     "tags": ["vip", "developer"]
            ... })
        """
        db = self._check_initialized()
        return await db.put(namespace, key, value)
    
    async def get(self, namespace: str, key: str) -> object:
        """
        Retrieve a value.
        
        Args:
            namespace: Logical grouping
            key: Unique identifier
        
        Returns:
            The stored value (deserialized from JSON)
        
        Raises:
            KeyNotFoundError: If key doesn't exist
        
        Example:
            >>> user = await db.get("users", "alice")
            >>> print(user["name"])  # "Alice"
        """
        db = self._check_initialized()
        return await db.get(namespace, key)
    
    async def get_at(self, namespace: str, key: str, timestamp: str) -> object:
        """
        Retrieve a value at a specific point in time.
        
        This enables time-travel queries - see what the value was
        at any historical timestamp.
        
        Args:
            namespace: Logical grouping
            key: Unique identifier
            timestamp: ISO 8601 timestamp (e.g., "2026-02-06T10:00:00Z")
        
        Returns:
            The value as it existed at that time
        
        Raises:
            KeyNotFoundError: If key didn't exist at that time
        
        Example:
            >>> # Get value from 1 hour ago
            >>> from datetime import datetime, timedelta
            >>> one_hour_ago = (datetime.utcnow() - timedelta(hours=1)).isoformat()
            >>> old_value = await db.get_at("config", "version", one_hour_ago)
        """
        db = self._check_initialized()
        return await db.get_at(namespace, key, timestamp)
    
    async def history(self, namespace: str, key: str) -> list[dict]:
        """
        Get complete history for a key.
        
        Returns all versions of the value, from oldest to newest.
        
        Args:
            namespace: Logical grouping
            key: Unique identifier
        
        Returns:
            List of history entries, each with:
                - value: The value at that point
                - timestamp: When it was stored
                - version_id: Unique version identifier
        
        Example:
            >>> history = await db.history("config", "model")
            >>> for entry in history:
            ...     print(f"{entry['timestamp']}: {entry['value']}")
        """
        db = self._check_initialized()
        return await db.history(namespace, key)
    
    async def delete(self, namespace: str, key: str) -> None:
        """
        Delete a key.
        
        Note: KoruDelta is append-only. This stores a null value
        as a tombstone. The history is preserved.
        
        Args:
            namespace: Logical grouping
            key: Unique identifier
        
        Example:
            >>> await db.delete("users", "alice")
            >>> # Key is now deleted, but history remains
        """
        db = self._check_initialized()
        return await db.delete(namespace, key)
    
    async def contains(self, namespace: str, key: str) -> bool:
        """
        Check if a key exists.
        
        Args:
            namespace: Logical grouping
            key: Unique identifier
        
        Returns:
            True if key exists, False otherwise
        """
        db = self._check_initialized()
        return await db.contains(namespace, key)
    
    async def list_keys(self, namespace: str) -> list[str]:
        """
        List all keys in a namespace.
        
        Args:
            namespace: Logical grouping
        
        Returns:
            List of key names
        """
        db = self._check_initialized()
        return await db.list_keys(namespace)
    
    async def embed(
        self,
        namespace: str,
        key: str,
        embedding: list[float],
        model: str,
        metadata: object | None = None,
    ) -> None:
        """
        Store a vector embedding.
        
        Args:
            namespace: Logical grouping
            key: Unique identifier
            embedding: Vector as list of floats (or numpy array)
            model: Name of embedding model (e.g., "text-embedding-3-small")
            metadata: Optional metadata dict
        
        Example:
            >>> await db.embed(
            ...     "documents", "doc1",
            ...     embedding=[0.1, 0.2, 0.3, ...],
            ...     model="text-embedding-3-small",
            ...     metadata={"title": "AI Paper"}
            ... )
        """
        db = self._check_initialized()
        return await db.embed(namespace, key, embedding, model, metadata)
    
    async def similar(
        self,
        namespace: str | None,
        query: list[float],
        top_k: int = 10,
        threshold: float = 0.0,
        model_filter: str | None = None,
    ) -> list[dict]:
        """
        Search for similar vectors.
        
        Args:
            namespace: Namespace to search (None = search all)
            query: Query vector
            top_k: Maximum results to return
            threshold: Minimum similarity score (0.0 to 1.0)
            model_filter: Only return vectors from this model
        
        Returns:
            List of results, each with:
                - namespace: Where the vector was found
                - key: Vector identifier
                - score: Similarity score (higher = more similar)
        
        Example:
            >>> results = await db.similar(
            ...     "documents",
            ...     query=[0.1, 0.2, 0.3, ...],
            ...     top_k=5,
            ...     threshold=0.8
            ... )
            >>> for r in results:
            ...     print(f"{r['key']}: {r['score']:.2f}")
        """
        db = self._check_initialized()
        return await db.similar(namespace, query, top_k, threshold, model_filter)
    
    async def stats(self) -> dict:
        """
        Get database statistics.
        
        Returns:
            Dict with:
                - key_count: Total number of keys
                - namespace_count: Number of namespaces
        """
        db = self._check_initialized()
        return await db.stats()
    
    def agent_memory(self, agent_id: str) -> AgentMemory:
        """
        Create an agent memory interface.
        
        This is a convenience method for creating an AgentMemory
        instance bound to this database.
        
        Args:
            agent_id: Unique identifier for the agent
        
        Returns:
            AgentMemory instance
        
        Example:
            >>> async with Database() as db:
            ...     memory = db.agent_memory("assistant-42")
            ...     await memory.episodes.remember("User asked about Python")
        """
        return AgentMemory(self, agent_id)
