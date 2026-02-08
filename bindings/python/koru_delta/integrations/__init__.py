"""
LLM Framework Integrations for KoruDelta.

This module provides integrations with popular LLM frameworks:
- LangChain: Vector store for RAG pipelines
- LlamaIndex: Native storage backend

Example:
    >>> # LangChain integration
    >>> from koru_delta import Database
    >>> from koru_delta.integrations.langchain import KoruDeltaVectorStore
    >>> 
    >>> db = Database()
    >>> vector_store = KoruDeltaVectorStore(db=db, namespace="docs")
    >>> vector_store.add_texts(["Hello world", "AI is amazing"])

    >>> # LlamaIndex integration  
    >>> from koru_delta.integrations.llamaindex import KoruDeltaVectorStore
    >>> vector_store = KoruDeltaVectorStore(db=db, namespace="docs")
"""

from __future__ import annotations

__all__ = [
    "chunk_document",
    "ChunkingConfig",
    "HybridSearcher",
    "HybridSearchResult",
    "CausalFilter",
]

# Import chunking utilities (always available)
from koru_delta.integrations.chunking import chunk_document, ChunkingConfig

# Import hybrid search (always available)
from koru_delta.integrations.hybrid import HybridSearcher, HybridSearchResult, CausalFilter

# LangChain integration (optional)
try:
    from koru_delta.integrations.langchain import KoruDeltaVectorStore
    __all__.append("KoruDeltaVectorStore")
except ImportError:
    pass

# LlamaIndex integration (optional)
try:
    from koru_delta.integrations.llamaindex import KoruDeltaVectorStore as LlamaIndexVectorStore
    __all__.append("LlamaIndexVectorStore")
except ImportError:
    pass
