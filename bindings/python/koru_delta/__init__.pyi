"""
KoruDelta - The Causal Database for AI agents.

A zero-configuration, causal database with native support for:
- Vector embeddings and semantic search
- Agent memory (episodic, semantic, procedural)
- Time travel (query historical state)
- Edge deployment (8MB binary, runs anywhere)
"""

from __future__ import annotations
from typing import Any

from koru_delta.config import Config
from koru_delta.agent_memory import AgentMemory

__version__: str = "2.0.0"

class KoruDeltaError(Exception):
    """Base exception for all KoruDelta errors."""
    ...

class KeyNotFoundError(KoruDeltaError):
    """Raised when a key is not found."""
    ...

class SerializationError(KoruDeltaError):
    """Raised when serialization fails."""
    ...

class ValidationError(KoruDeltaError):
    """Raised when validation fails."""
    ...

class DatabaseClosedError(KoruDeltaError):
    """Raised when operating on a closed database."""
    ...

class Database:
    """
    KoruDelta database instance.
    
    This is the main entry point for interacting with the database.
    Use as an async context manager for automatic resource management.
    """
    
    def __init__(self, config: Config | None = None) -> None: ...
    
    async def __aenter__(self) -> Database: ...
    async def __aexit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None: ...
    
    async def put(self, namespace: str, key: str, value: object) -> None:
        """Store a value."""
        ...
    
    async def get(self, namespace: str, key: str) -> object:
        """Retrieve a value."""
        ...
    
    async def get_at(self, namespace: str, key: str, timestamp: str) -> object:
        """Retrieve a value at a specific point in time."""
        ...
    
    async def history(self, namespace: str, key: str) -> list[dict[str, Any]]:
        """Get complete history for a key."""
        ...
    
    async def delete(self, namespace: str, key: str) -> None:
        """Delete a key."""
        ...
    
    async def contains(self, namespace: str, key: str) -> bool:
        """Check if a key exists."""
        ...
    
    async def list_keys(self, namespace: str) -> list[str]:
        """List all keys in a namespace."""
        ...
    
    async def embed(
        self,
        namespace: str,
        key: str,
        embedding: list[float],
        model: str,
        metadata: object | None = None,
    ) -> None:
        """Store a vector embedding."""
        ...
    
    async def similar(
        self,
        namespace: str | None,
        query: list[float],
        top_k: int = 10,
        threshold: float = 0.0,
        model_filter: str | None = None,
    ) -> list[dict[str, Any]]:
        """Search for similar vectors."""
        ...
    
    async def stats(self) -> dict[str, Any]:
        """Get database statistics."""
        ...
    
    def agent_memory(self, agent_id: str) -> AgentMemory:
        """Create an agent memory interface."""
        ...

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
