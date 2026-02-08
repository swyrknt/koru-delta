"""
LlamaIndex integration for KoruDelta.

Provides native storage backend for LlamaIndex vector store operations.

Note: Requires llama-index to be installed:
    pip install llama-index

Example:
    >>> from koru_delta import Database
    >>> from koru_delta.integrations.llamaindex import KoruDeltaVectorStore
    >>> from llama_index.core import VectorStoreIndex, SimpleDirectoryReader
    >>> 
    >>> db = Database()
    >>> vector_store = KoruDeltaVectorStore(
    ...     db=db,
    ...     namespace="llama_docs"
    ... )
    >>> 
    >>> # Create index with KoruDelta storage
    >>> documents = SimpleDirectoryReader("data").load_data()
    >>> index = VectorStoreIndex.from_documents(
    ...     documents,
    ...     vector_store=vector_store
    ... )
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from koru_delta import Database

# LlamaIndex imports (optional dependency)
try:
    from llama_index.core.vector_stores.types import (
        VectorStore,
        VectorStoreQuery,
        VectorStoreQueryResult,
        VectorStoreQueryMode,
        MetadataFilters,
    )
    from llama_index.core.schema import TextNode, BaseNode, NodeWithScore
    from llama_index.core.vector_stores.utils import (
        node_to_metadata_dict,
        metadata_dict_to_node,
    )
    LLAMAINDEX_AVAILABLE = True
except ImportError:
    LLAMAINDEX_AVAILABLE = False
    # Create dummy classes for type checking
    class VectorStore:  # type: ignore
        pass
    class VectorStoreQuery:  # type: ignore
        pass
    class VectorStoreQueryResult:  # type: ignore
        pass
    class VectorStoreQueryMode:  # type: ignore
        DEFAULT = "default"
        HYBRID = "hybrid"
    class MetadataFilters:  # type: ignore
        pass
    class TextNode:  # type: ignore
        pass
    class BaseNode:  # type: ignore
        pass
    class NodeWithScore:  # type: ignore
        pass


class KoruDeltaVectorStore(VectorStore if LLAMAINDEX_AVAILABLE else object):
    """LlamaIndex VectorStore implementation using KoruDelta.
    
    This allows KoruDelta to be used as a storage backend in
    LlamaIndex RAG pipelines.
    
    Features:
    - Native LlamaIndex integration
    - Metadata filtering
    - Causal queries (time-travel search)
    - Hybrid search modes
    
    Example:
        >>> from koru_delta import Database
        >>> from koru_delta.integrations.llamaindex import KoruDeltaVectorStore
        >>> from llama_index.core import VectorStoreIndex
        >>> 
        >>> db = Database()
        >>> vector_store = KoruDeltaVectorStore(
        ...     db=db,
        ...     namespace="documents"
        ... )
        >>> 
        >>> # Use with LlamaIndex
        >>> index = VectorStoreIndex.from_vector_store(vector_store)
    """
    
    stores_text: bool = True
    is_embedding_query: bool = True
    
    def __init__(
        self,
        db: "Database",
        namespace: str,
        model_name: str = "llamaindex",
    ):
        """Initialize KoruDeltaVectorStore for LlamaIndex.
        
        Args:
            db: KoruDelta database instance
            namespace: Namespace for storing vectors
            model_name: Name identifier for embeddings
        
        Raises:
            ImportError: If llama-index is not installed
        """
        if not LLAMAINDEX_AVAILABLE:
            raise ImportError(
                "LlamaIndex integration requires llama-index. "
                "Install with: pip install llama-index"
            )
        
        self.db = db
        self.namespace = namespace
        self.model_name = model_name
        self._node_counter = 0
    
    async def async_add(
        self,
        nodes: list[BaseNode],
    ) -> list[str]:
        """Add nodes to vector store (async).
        
        Args:
            nodes: List of LlamaIndex nodes to add
        
        Returns:
            List of node IDs
        """
        ids = []
        
        for node in nodes:
            node_id = node.node_id or f"node_{self._node_counter}"
            self._node_counter += 1
            ids.append(node_id)
            
            # Extract embedding
            embedding = node.embedding
            if embedding is None:
                raise ValueError(
                    f"Node {node_id} has no embedding. "
                    "Ensure nodes are embedded before adding."
                )
            
            # Convert node to metadata
            metadata = node_to_metadata_dict(
                node,
                remove_text=False,
                flat_metadata=False,
            )
            
            # Store in KoruDelta
            await self.db.embed(
                namespace=self.namespace,
                key=node_id,
                embedding=embedding,
                model=self.model_name,
                metadata=metadata,
            )
        
        return ids
    
    def add(
        self,
        nodes: list[BaseNode],
    ) -> list[str]:
        """Add nodes to vector store (sync).
        
        Args:
            nodes: List of LlamaIndex nodes to add
        
        Returns:
            List of node IDs
        """
        import asyncio
        
        try:
            loop = asyncio.get_event_loop()
            if loop.is_running():
                import nest_asyncio
                nest_asyncio.apply()
                return loop.run_until_complete(self.async_add(nodes))
            return loop.run_until_complete(self.async_add(nodes))
        except RuntimeError:
            return asyncio.run(self.async_add(nodes))
    
    async def adelete(
        self,
        ref_doc_id: str,
        **delete_kwargs: Any,
    ) -> None:
        """Delete nodes by reference document ID.
        
        Note: KoruDelta is append-only, so this stores a tombstone.
        The data remains in history.
        
        Args:
            ref_doc_id: Reference document ID to delete
        """
        # Mark as deleted by storing null
        await self.db.delete(self.namespace, ref_doc_id)
    
    def delete(
        self,
        ref_doc_id: str,
        **delete_kwargs: Any,
    ) -> None:
        """Delete nodes by reference document ID (sync)."""
        import asyncio
        
        try:
            loop = asyncio.get_event_loop()
            if loop.is_running():
                import nest_asyncio
                nest_asyncio.apply()
                loop.run_until_complete(self.adelete(ref_doc_id, **delete_kwargs))
            else:
                loop.run_until_complete(self.adelete(ref_doc_id, **delete_kwargs))
        except RuntimeError:
            asyncio.run(self.adelete(ref_doc_id, **delete_kwargs))
    
    async def aquery(
        self,
        query: VectorStoreQuery,
        **kwargs: Any,
    ) -> VectorStoreQueryResult:
        """Query vector store (async).
        
        Args:
            query: VectorStoreQuery with embedding and filters
            **kwargs: Additional query parameters
        
        Returns:
            VectorStoreQueryResult with matching nodes
        """
        # Get query embedding
        query_embedding = query.query_embedding
        if query_embedding is None:
            raise ValueError("Query must have an embedding")
        
        # Build filters
        filters = self._build_filters(query)
        
        # Determine top k
        top_k = query.similarity_top_k or 10
        
        # Search
        results = await self.db.similar(
            namespace=self.namespace,
            query=query_embedding,
            top_k=top_k,
            threshold=query.alpha or 0.0,
            model_filter=self.model_name,
        )
        
        # Convert to nodes with scores
        nodes_with_scores = []
        
        for result in results:
            key = result.get("key", "")
            score = result.get("score", 0.0)
            
            try:
                # Get full data
                data = await self.db.get(self.namespace, key)
                
                if isinstance(data, dict):
                    # Reconstruct node from metadata
                    node = metadata_dict_to_node(data)
                    
                    # Apply filters if present
                    if filters and not self._node_matches_filters(node, filters):
                        continue
                    
                    # Create NodeWithScore
                    node_with_score = NodeWithScore(node=node, score=score)
                    nodes_with_scores.append(node_with_score)
            
            except Exception:
                continue
        
        return VectorStoreQueryResult(
            nodes=[n.node for n in nodes_with_scores],
            similarities=[n.score for n in nodes_with_scores],
            ids=[n.node.node_id for n in nodes_with_scores],
        )
    
    def query(
        self,
        query: VectorStoreQuery,
        **kwargs: Any,
    ) -> VectorStoreQueryResult:
        """Query vector store (sync)."""
        import asyncio
        
        try:
            loop = asyncio.get_event_loop()
            if loop.is_running():
                import nest_asyncio
                nest_asyncio.apply()
                return loop.run_until_complete(self.aquery(query, **kwargs))
            return loop.run_until_complete(self.aquery(query, **kwargs))
        except RuntimeError:
            return asyncio.run(self.aquery(query, **kwargs))
    
    def _build_filters(
        self,
        query: VectorStoreQuery,
    ) -> dict[str, Any] | None:
        """Build metadata filters from query."""
        if query.filters is None:
            return None
        
        # Convert LlamaIndex filters to dict
        if isinstance(query.filters, MetadataFilters):
            return self._convert_metadata_filters(query.filters)
        
        return query.filters
    
    def _convert_metadata_filters(
        self,
        filters: MetadataFilters,
    ) -> dict[str, Any]:
        """Convert MetadataFilters to dict."""
        result = {}
        
        if hasattr(filters, "filters"):
            for f in filters.filters:
                if hasattr(f, "key") and hasattr(f, "value"):
                    result[f.key] = f.value
        
        return result
    
    def _node_matches_filters(
        self,
        node: BaseNode,
        filters: dict[str, Any],
    ) -> bool:
        """Check if node matches metadata filters."""
        metadata = node.metadata or {}
        
        for key, value in filters.items():
            if metadata.get(key) != value:
                return False
        
        return True
    
    async def get_nodes(
        self,
        node_ids: list[str],
    ) -> list[BaseNode]:
        """Get nodes by ID.
        
        Args:
            node_ids: List of node IDs to retrieve
        
        Returns:
            List of nodes (may be fewer than requested if some don't exist)
        """
        nodes = []
        
        for node_id in node_ids:
            try:
                data = await self.db.get(self.namespace, node_id)
                if isinstance(data, dict):
                    node = metadata_dict_to_node(data)
                    nodes.append(node)
            except Exception:
                continue
        
        return nodes
    
    async def time_travel_query(
        self,
        query: VectorStoreQuery,
        timestamp: str,
        **kwargs: Any,
    ) -> VectorStoreQueryResult:
        """Query as of a specific point in time.
        
        This is a KoruDelta-specific extension that allows querying
        the state of the index at any historical timestamp.
        
        Args:
            query: VectorStoreQuery with embedding
            timestamp: ISO 8601 timestamp to query at
            **kwargs: Additional parameters
        
        Returns:
            VectorStoreQueryResult with nodes as they existed at that time
        """
        query_embedding = query.query_embedding
        if query_embedding is None:
            raise ValueError("Query must have an embedding")
        
        top_k = query.similarity_top_k or 10
        
        # Get all current keys first (we need to know what to query)
        # This is a limitation - we search current keys but get historical values
        current_results = await self.db.similar(
            namespace=self.namespace,
            query=query_embedding,
            top_k=top_k * 3,  # Get extra to account for missing historical data
            model_filter=self.model_name,
        )
        
        nodes_with_scores = []
        
        for result in current_results:
            key = result.get("key", "")
            
            try:
                # Get historical value
                data = await self.db.get_at(self.namespace, key, timestamp)
                
                if isinstance(data, dict):
                    node = metadata_dict_to_node(data)
                    
                    # Recalculate similarity (embedding might have changed)
                    # For now, use stored score
                    score = result.get("score", 0.0)
                    
                    node_with_score = NodeWithScore(node=node, score=score)
                    nodes_with_scores.append(node_with_score)
            
            except Exception:
                # Node didn't exist at that time
                continue
        
        # Sort by score and take top_k
        nodes_with_scores.sort(key=lambda x: x.score or 0, reverse=True)
        nodes_with_scores = nodes_with_scores[:top_k]
        
        return VectorStoreQueryResult(
            nodes=[n.node for n in nodes_with_scores],
            similarities=[n.score for n in nodes_with_scores],
            ids=[n.node.node_id for n in nodes_with_scores],
        )
    
    async def get_node_history(
        self,
        node_id: str,
    ) -> list[dict]:
        """Get version history for a node.
        
        This is a KoruDelta-specific feature that shows all
        versions of a node over time.
        
        Args:
            node_id: Node ID to get history for
        
        Returns:
            List of version entries with timestamp and value
        """
        return await self.db.history(self.namespace, node_id)
    
    def persist(
        self,
        persist_path: str,
        **kwargs: Any,
    ) -> None:
        """Persist the vector store.
        
        Note: KoruDelta handles persistence automatically.
        This method is a no-op for compatibility.
        
        Args:
            persist_path: Path to persist to (ignored)
            **kwargs: Additional arguments (ignored)
        """
        # KoruDelta persists automatically, nothing to do
        pass
    
    @classmethod
    def from_params(
        cls,
        db: "Database",
        namespace: str,
        **kwargs: Any,
    ) -> "KoruDeltaVectorStore":
        """Create from parameters.
        
        Args:
            db: Database instance
            namespace: Namespace
            **kwargs: Additional parameters
        
        Returns:
            New KoruDeltaVectorStore instance
        """
        model_name = kwargs.get("model_name", "llamaindex")
        return cls(db=db, namespace=namespace, model_name=model_name)
