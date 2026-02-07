"""Configuration for KoruDelta database."""

from __future__ import annotations
from dataclasses import dataclass
from pathlib import Path
from typing import Any


@dataclass
class Config:
    """
    Configuration for KoruDelta database.
    
    All parameters are optional. Sensible defaults are provided
    for zero-configuration usage.
    
    Example:
        >>> # In-memory (default)
        >>> config = Config()
        
        >>> # With persistence
        >>> config = Config(path="~/.myapp/db")
        
        >>> # With custom limits
        >>> config = Config(
        ...     path="~/.myapp/db",
        ...     max_memory_mb=1024,
        ...     enable_wal=True
        ... )
    """
    
    # Persistence
    path: str | Path | None = None
    """Path for persistent storage. If None, database is in-memory only."""
    
    # Memory limits
    max_memory_mb: int = 512
    """Maximum memory usage in MB (0 = unlimited)."""
    
    max_disk_mb: int = 10 * 1024
    """Maximum disk usage in MB (0 = unlimited)."""
    
    # Durability
    enable_wal: bool = True
    """Enable write-ahead logging for crash safety."""
    
    sync_on_write: bool = False
    """Sync to disk on every write (slower but safer)."""
    
    # Performance
    hot_cache_size: int = 1000
    """Number of items to keep in hot memory cache."""
    
    def __post_init__(self):
        """Validate and normalize configuration."""
        if self.path is not None:
            # Expand user home directory
            self.path = Path(self.path).expanduser()
    
    def _to_rust(self) -> dict[str, Any]:
        """Convert to dict for Rust FFI."""
        return {
            "path": str(self.path) if self.path else None,
            "max_memory_mb": self.max_memory_mb,
            "max_disk_mb": self.max_disk_mb,
            "enable_wal": self.enable_wal,
            "sync_on_write": self.sync_on_write,
            "hot_cache_size": self.hot_cache_size,
        }
