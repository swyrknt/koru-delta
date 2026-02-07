"""Configuration for KoruDelta database."""

from __future__ import annotations
from dataclasses import dataclass
from pathlib import Path
from typing import Any

@dataclass
class Config:
    """Configuration for KoruDelta database."""
    
    # Persistence
    path: str | Path | None = None
    """Path for persistent storage. If None, database is in-memory only."""
    
    # Memory limits
    max_memory_mb: int = 512
    """Maximum memory usage in MB (0 = unlimited)."""
    
    max_disk_mb: int = 10240
    """Maximum disk usage in MB (0 = unlimited)."""
    
    # Durability
    enable_wal: bool = True
    """Enable write-ahead logging for crash safety."""
    
    sync_on_write: bool = False
    """Sync to disk on every write (slower but safer)."""
    
    # Performance
    hot_cache_size: int = 1000
    """Number of items to keep in hot memory cache."""
    
    def __post_init__(self) -> None: ...
    def _to_rust(self) -> dict[str, Any]: ...
