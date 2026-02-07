"""Agent memory management for AI agents."""

from __future__ import annotations
from dataclasses import dataclass
from enum import Enum, auto
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from koru_delta import Database

class MemoryType(Enum):
    """Types of agent memory."""
    EPISODIC = auto()
    SEMANTIC = auto()
    PROCEDURAL = auto()

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
    
    def __init__(self, agent: AgentMemory) -> None: ...
    
    async def remember(
        self,
        content: str,
        importance: float = 0.5,
        tags: list[str] | None = None,
    ) -> None:
        """Remember a specific event."""
        ...

class FactMemory:
    """Semantic memory (facts and knowledge)."""
    
    def __init__(self, agent: AgentMemory) -> None: ...
    
    async def learn(
        self,
        key: str,
        content: str,
        tags: list[str] | None = None,
    ) -> None:
        """Learn a fact."""
        ...

class ProcedureMemory:
    """Procedural memory (how-to knowledge)."""
    
    def __init__(self, agent: AgentMemory) -> None: ...
    
    async def learn(
        self,
        name: str,
        steps: list[str] | str,
        success_rate: float | None = None,
    ) -> None:
        """Learn a procedure."""
        ...

class AgentMemory:
    """
    Memory system for AI agents.
    
    Provides human-like memory with episodic, semantic, and procedural
    memory types. Supports natural forgetting and consolidation.
    """
    
    episodes: EpisodeMemory
    facts: FactMemory
    procedures: ProcedureMemory
    
    def __init__(self, db: Database, agent_id: str) -> None: ...
    
    async def recall(
        self,
        query: str,
        limit: int = 10,
        min_relevance: float = 0.0,
        memory_type: MemoryType | None = None,
    ) -> list[MemoryRecall]:
        """Recall memories relevant to a query."""
        ...
    
    async def consolidate(self) -> dict[str, Any]:
        """Consolidate old memories."""
        ...
    
    async def stats(self) -> dict[str, Any]:
        """Get memory statistics."""
        ...
