"""Agent memory management for AI agents."""

from __future__ import annotations
from dataclasses import dataclass
from enum import Enum, auto
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from koru_delta import Database


class MemoryType(Enum):
    """Types of agent memory."""
    EPISODIC = auto()    # Specific events
    SEMANTIC = auto()    # Facts and knowledge
    PROCEDURAL = auto()  # How-to knowledge


@dataclass
class MemoryRecall:
    """Result of a memory recall operation."""
    content: str
    memory_type: MemoryType
    relevance: float
    importance: float
    created_at: str
    access_count: int
    tags: list[str]


class EpisodeMemory:
    """Episodic memory (specific events)."""
    
    def __init__(self, agent: AgentMemory):
        self._agent = agent
    
    async def remember(
        self,
        content: str,
        importance: float = 0.5,
        tags: list[str] | None = None,
    ) -> None:
        """
        Remember a specific event.
        
        Args:
            content: Description of the event
            importance: Importance score (0.0 to 1.0)
            tags: Optional tags for categorization
        
        Example:
            >>> await memory.episodes.remember(
            ...     "User asked about Python bindings",
            ...     importance=0.8,
            ...     tags=["python", "bindings"]
            ... )
        """
        await self._agent._remember(MemoryType.EPISODIC, content, importance, tags or [])


class FactMemory:
    """Semantic memory (facts and knowledge)."""
    
    def __init__(self, agent: AgentMemory):
        self._agent = agent
    
    async def learn(
        self,
        key: str,
        content: str,
        tags: list[str] | None = None,
    ) -> None:
        """
        Learn a fact.
        
        Args:
            key: Unique identifier for this fact
            content: The fact content
            tags: Optional tags for categorization
        
        Example:
            >>> await memory.facts.learn(
            ...     "python_bindings",
            ...     "KoruDelta has Python bindings via PyO3",
            ...     tags=["tech", "python"]
            ... )
        """
        await self._agent._remember(MemoryType.SEMANTIC, content, 0.9, tags or [], key=key)


class ProcedureMemory:
    """Procedural memory (how-to knowledge)."""
    
    def __init__(self, agent: AgentMemory):
        self._agent = agent
    
    async def learn(
        self,
        name: str,
        steps: list[str] | str,
        success_rate: float | None = None,
    ) -> None:
        """
        Learn a procedure.
        
        Args:
            name: Name of the procedure
            steps: Steps to execute (list or newline-separated string)
            success_rate: Optional success rate (0.0 to 1.0)
        
        Example:
            >>> await memory.procedures.learn(
            ...     "handle_error",
            ...     ["1. Log error", "2. Notify user", "3. Retry"],
            ...     success_rate=0.95
            ... )
        """
        if isinstance(steps, list):
            steps = "\n".join(steps)
        
        importance = success_rate or 0.5
        await self._agent._remember(
            MemoryType.PROCEDURAL,
            steps,
            importance,
            ["procedure", name],
            key=name
        )


class AgentMemory:
    """
    Memory system for AI agents.
    
    Provides human-like memory with episodic, semantic, and procedural
    memory types. Supports natural forgetting and consolidation.
    
    Example:
        >>> async with Database() as db:
        ...     memory = db.agent_memory("assistant-42")
        ...
        ...     # Store memories
        ...     await memory.episodes.remember("User prefers dark mode")
        ...     await memory.facts.learn("user_name", "User's name is Alice")
        ...
        ...     # Recall
        ...     results = await memory.recall("What does user prefer?")
        ...     for r in results:
        ...         print(f"{r.relevance:.2f}: {r.content}")
    """
    
    def __init__(self, db: Database, agent_id: str):
        self._db = db
        self._agent_id = agent_id
        self.episodes = EpisodeMemory(self)
        self.facts = FactMemory(self)
        self.procedures = ProcedureMemory(self)
    
    def _namespace(self) -> str:
        """Get namespace for this agent's memories."""
        return f"agent_memory:{self._agent_id}"
    
    async def _remember(
        self,
        memory_type: MemoryType,
        content: str,
        importance: float,
        tags: list[str],
        key: str | None = None,
    ) -> None:
        """Internal method to store a memory."""
        import hashlib
        
        if key is None:
            # Generate key from content hash
            key = hashlib.sha256(f"{self._agent_id}/{content}".encode()).hexdigest()[:16]
        
        memory_data = {
            "type": memory_type.name.lower(),
            "content": content,
            "importance": importance,
            "tags": tags,
            "created_at": "now",  # Will be set by Rust
        }
        
        await self._db.put(self._namespace(), key, memory_data)
    
    async def recall(
        self,
        query: str,
        limit: int = 10,
        min_relevance: float = 0.0,
        memory_type: MemoryType | None = None,
    ) -> list[MemoryRecall]:
        """
        Recall memories relevant to a query.
        
        Searches through all memory types and returns the most
        relevant memories based on semantic similarity and importance.
        
        Args:
            query: Search query
            limit: Maximum number of results
            min_relevance: Minimum relevance score (0.0 to 1.0)
            memory_type: Filter by specific memory type
        
        Returns:
            List of MemoryRecall objects, sorted by relevance
        
        Example:
            >>> results = await memory.recall("Python bindings", limit=5)
            >>> for r in results:
            ...     print(f"{r.relevance:.2f}: {r.content}")
        """
        # For now, do simple keyword search
        # In the future, this will use vector search with embeddings
        keys = await self._db.list_keys(self._namespace())
        results = []
        
        query_lower = query.lower()
        
        for key in keys:
            try:
                data = await self._db.get(self._namespace(), key)
                content = data.get("content", "")
                
                # Simple relevance scoring
                relevance = 0.0
                if query_lower in content.lower():
                    relevance = 0.5
                
                # Boost by importance
                importance = data.get("importance", 0.5)
                relevance = relevance * 0.5 + importance * 0.5
                
                if relevance < min_relevance:
                    continue
                
                # Filter by type
                mem_type_str = data.get("type", "episodic")
                mem_type = MemoryType[mem_type_str.upper()]
                
                if memory_type is not None and mem_type != memory_type:
                    continue
                
                results.append(MemoryRecall(
                    content=content,
                    memory_type=mem_type,
                    relevance=relevance,
                    importance=importance,
                    created_at=data.get("created_at", ""),
                    access_count=data.get("access_count", 0),
                    tags=data.get("tags", []),
                ))
            except Exception:
                continue
        
        # Sort by relevance
        results.sort(key=lambda x: x.relevance, reverse=True)
        
        return results[:limit]
    
    async def consolidate(self) -> dict[str, Any]:
        """
        Consolidate old memories.
        
        This compresses old, low-importance memories to save space.
        Should be called periodically (e.g., daily).
        
        Returns:
            Dict with consolidation statistics
        
        Example:
            >>> summary = await memory.consolidate()
            >>> print(f"Consolidated {summary['consolidated']} memories")
        """
        # Placeholder - full implementation will be in Rust
        return {"consolidated": 0, "total": 0}
    
    async def stats(self) -> dict[str, Any]:
        """
        Get memory statistics.
        
        Returns:
            Dict with memory counts by type
        """
        keys = await self._db.list_keys(self._namespace())
        
        stats = {
            "total": len(keys),
            "episodic": 0,
            "semantic": 0,
            "procedural": 0,
        }
        
        for key in keys:
            try:
                data = await self._db.get(self._namespace(), key)
                mem_type = data.get("type", "episodic")
                stats[mem_type] = stats.get(mem_type, 0) + 1
            except Exception:
                continue
        
        return stats
