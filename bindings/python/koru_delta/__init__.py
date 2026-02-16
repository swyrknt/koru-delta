"""
KoruDelta Python Bindings - LCA Architecture v3.0.0

This package provides Python access to the Local Causal Agent (LCA) architecture,
a unified system where all state transformations follow the synthesis formula:
    ΔNew = ΔLocal_Root ⊕ ΔAction_Data

The LCA architecture unifies previously separate concepts:
- Storage (formerly HotMemory) → TemperatureAgent with distinction-based versioning
- Vectors (formerly ColdStorage) → DistinctionAgent with structure-based embeddings
- Queries & Views (formerly MediumTerm/Search) → ChronicleAgent
- Identity → SelfAgent with cryptography and autonomy

Example usage:
    >>> import asyncio
    >>> from koru_delta import Database
    >>> 
    >>> async def main():
    ...     db = await Database.create()
    ...     
    ...     # Store with automatic semantic embedding
    ...     await db.put_similar("docs", "doc1", "Hello world", {"type": "greeting"})
    ...     
    ...     # Semantic search
    ...     results = await db.find_similar("docs", "search query", top_k=5)
    ...     
    ...     # Batch operations
    ...     await db.put_batch_in_ns("data", [
    ...         ("key1", {"value": 1}),
    ...         ("key2", {"value": 2}),
    ...     ])
    >>> 
    >>> asyncio.run(main())
"""

from koru_delta._internal import (
    # Core classes
    Database,
    IdentityManager,
    Workspace,
    
    # Exceptions
    KoruDeltaError,
    KeyNotFoundError,
    InvalidDataError,
    StorageError,
    SerializationError,
    EngineError,
    TimeError,
    
    # Version
    __version__,
)

__all__ = [
    # Core classes
    "Database",
    "IdentityManager", 
    "Workspace",
    
    # Exceptions
    "KoruDeltaError",
    "KeyNotFoundError",
    "InvalidDataError",
    "StorageError",
    "SerializationError",
    "EngineError",
    "TimeError",
]

__version__ = "3.0.0"


def version():
    """Return the version of the KoruDelta bindings."""
    return __version__


async def create_database():
    """
    Convenience function to create a new Database instance.
    
    This is equivalent to calling `Database.create()`.
    
    Example:
        >>> import koru_delta as kd
        >>> db = await kd.create_database()
    """
    return await Database.create()
