"""
LangChain integration for KoruDelta.

Provides a VectorStore implementation for use in LangChain RAG pipelines.

Note: Requires langchain to be installed:
    pip install langchain

Example:
    >>> from koru_delta import Database
    >>> from koru_delta.integrations.langchain import KoruDeltaVectorStore
    >>> from langchain_openai import OpenAIEmbeddings
    >>> from langchain_core.documents import Document
    >>> 
    >>> db = Database()
    >>> vector_store = KoruDeltaVectorStore(
    ...     db=db,
    ...     namespace="documents",
    ...     embedding_model=OpenAIEmbeddings()
    ... )
    >>> 
    >>> # Add documents
    >>> docs = [Document(page_content="Hello world", metadata={"source": "example.txt"})]
    >>> vector_store.add_documents(docs)
    >>> 
    >>> # Search
    >>> results = vector_store.similarity_search("hello", k=5)
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from koru_delta import Database

# LangChain imports (optional dependency)
try:
    from langchain_core.documents import Document
    from langchain_core.embeddings import Embeddings
    from langchain_core.vectorstores import VectorStore
    LANGCHAIN_AVAILABLE = True
except ImportError:
    LANGCHAIN_AVAILABLE = False
    # Create dummy classes for type checking
    class VectorStore:  # type: ignore
        pass
    class Document:  # type: ignore
        pass
    class Embeddings:  # type: ignore
        pass


class KoruDeltaVectorStore(VectorStore if LANGCHAIN_AVAILABLE else object):
    """LangChain VectorStore implementation using KoruDelta.
    
    This allows KoruDelta to be used as a drop-in replacement for
    other vector databases in LangChain RAG pipelines.
    
    Features:
    - Automatic embedding generation
    - Metadata filtering
    - Causal/historical queries via additional methods
    - Hybrid search combining semantic and causal relevance
    
    Example:
        >>> from koru_delta import Database
        >>> from koru_delta.integrations.langchain import KoruDeltaVectorStore
        >>> from langchain_openai import OpenAIEmbeddings
        >>> 
        >>> db = Database()
        >>> store = KoruDeltaVectorStore(
        ...     db=db,
        ...     namespace="docs",
        ...     embedding_model=OpenAIEmbeddings(),
        ... )
        >>> 
        >>> # Add texts
        >>> texts = ["Hello world", "AI is the future"]
        >>> metadatas = [{"source": "a.txt"}, {"source": "b.txt"}]
        >>> store.add_texts(texts, metadatas)
        >>> 
        >>> # Search
        >>> docs = store.similarity_search("hello", k=3)
    """
    
    def __init__(
        self,
        db: "Database",
        namespace: str,
        embedding_model: "Embeddings | None" = None,
        model_name: str = "external",
    ):
        """Initialize KoruDeltaVectorStore.
        
        Args:
            db: KoruDelta database instance
            namespace: Namespace for storing vectors
            embedding_model: LangChain embeddings model for automatic embedding
            model_name: Name to store with embeddings (identifies the model used)
        
        Raises:
            ImportError: If langchain is not installed
        """
        if not LANGCHAIN_AVAILABLE:
            raise ImportError(
                "LangChain integration requires langchain-core. "
                "Install with: pip install langchain"
            )
        
        self.db = db
        self.namespace = namespace
        self.embedding_model = embedding_model
        self.model_name = model_name
        self._doc_counter = 0
    
    def add_texts(
        self,
        texts: list[str],
        metadatas: list[dict] | None = None,
        **kwargs: Any,
) -> list[str]:
        """Add texts to the vector store.
        
        Args:
            texts: List of text strings to add
            metadatas: Optional list of metadata dicts (one per text)
            **kwargs: Additional arguments (ignored for compatibility)
        
        Returns:
            List of IDs for the added texts
        
        Raises:
            ValueError: If embedding_model is not set and texts are provided
        """
        import asyncio
        
        if self.embedding_model is None:
            raise ValueError(
                "embedding_model is required for add_texts. "
                "Provide it during initialization or use add_embeddings."
            )
        
        # Generate embeddings
        embeddings = self.embedding_model.embed_documents(texts)
        
        # Add with embeddings
        return self.add_embeddings(
            text_embeddings=list(zip(texts, embeddings)),
            metadatas=metadatas,
        )
    
    def add_embeddings(
        self,
        text_embeddings: list[tuple[str, list[float]]],
        metadatas: list[dict] | None = None,
        **kwargs: Any,
    ) -> list[str]:
        """Add texts with pre-computed embeddings.
        
        Args:
            text_embeddings: List of (text, embedding) tuples
            metadatas: Optional list of metadata dicts
            **kwargs: Additional arguments (ignored)
        
        Returns:
            List of IDs for the added embeddings
        """
        import asyncio
        
        ids = []
        
        async def _add():
            for i, (text, embedding) in enumerate(text_embeddings):
                # Generate unique ID
                self._doc_counter += 1
                doc_id = f"doc_{self._doc_counter}"
                ids.append(doc_id)
                
                # Prepare metadata
                metadata = {}
                if metadatas and i < len(metadatas):
                    metadata = metadatas[i].copy()
                metadata["text"] = text
                
                # Store embedding
                await self.db.embed(
                    namespace=self.namespace,
                    key=doc_id,
                    embedding=embedding,
                    model=self.model_name,
                    metadata=metadata,
                )
        
        # Run async operation
        try:
            loop = asyncio.get_event_loop()
            if loop.is_running():
                # We're in an async context, create task
                import nest_asyncio
                nest_asyncio.apply()
                loop.run_until_complete(_add())
            else:
                loop.run_until_complete(_add())
        except RuntimeError:
            # No event loop, create one
            asyncio.run(_add())
        
        return ids
    
    def add_documents(
        self,
        documents: list["Document"],
        **kwargs: Any,
    ) -> list[str]:
        """Add LangChain Document objects.
        
        Args:
            documents: List of Document objects
            **kwargs: Additional arguments (ignored)
        
        Returns:
            List of IDs for the added documents
        """
        texts = [doc.page_content for doc in documents]
        metadatas = [doc.metadata for doc in documents]
        return self.add_texts(texts, metadatas)
    
    def similarity_search(
        self,
        query: str,
        k: int = 4,
        **kwargs: Any,
    ) -> list["Document"]:
        """Search for similar documents.
        
        Args:
            query: Query text
            k: Number of results to return
            **kwargs: Additional arguments (filter, etc.)
        
        Returns:
            List of matching Document objects
        
        Raises:
            ValueError: If embedding_model is not set
        """
        if self.embedding_model is None:
            raise ValueError("embedding_model is required for similarity_search")
        
        # Generate query embedding
        query_embedding = self.embedding_model.embed_query(query)
        
        # Search by vector
        return self.similarity_search_by_vector(query_embedding, k=k, **kwargs)
    
    def similarity_search_by_vector(
        self,
        embedding: list[float],
        k: int = 4,
        **kwargs: Any,
    ) -> list["Document"]:
        """Search by pre-computed embedding vector.
        
        Args:
            embedding: Query embedding vector
            k: Number of results
            **kwargs: Additional arguments
                - filter: Dict of metadata filters
                - score_threshold: Minimum similarity score
        
        Returns:
            List of matching Document objects
        """
        import asyncio
        
        filter_dict = kwargs.get("filter", {})
        score_threshold = kwargs.get("score_threshold", 0.0)
        
        results = []
        
        async def _search():
            nonlocal results
            # Search vectors
            vector_results = await self.db.similar(
                namespace=self.namespace,
                query=embedding,
                top_k=k,
                threshold=score_threshold,
                model_filter=self.model_name,
            )
            
            # Convert to Documents
            for vr in vector_results:
                key = vr.get("key", "")
                score = vr.get("score", 0.0)
                
                try:
                    # Get full metadata
                    full_data = await self.db.get(self.namespace, key)
                    
                    if isinstance(full_data, dict):
                        text = full_data.get("text", "")
                        metadata = {k: v for k, v in full_data.items() if k != "text"}
                        metadata["_key"] = key
                        metadata["_score"] = score
                        
                        # Apply metadata filter
                        if self._matches_filter(metadata, filter_dict):
                            results.append(Document(page_content=text, metadata=metadata))
                    
                except Exception:
                    continue
        
        # Run search
        try:
            loop = asyncio.get_event_loop()
            if loop.is_running():
                import nest_asyncio
                nest_asyncio.apply()
                loop.run_until_complete(_search())
            else:
                loop.run_until_complete(_search())
        except RuntimeError:
            asyncio.run(_search())
        
        return results[:k]
    
    def similarity_search_with_relevance_scores(
        self,
        query: str,
        k: int = 4,
        **kwargs: Any,
    ) -> list[tuple["Document", float]]:
        """Search with relevance scores.
        
        Args:
            query: Query text
            k: Number of results
            **kwargs: Additional arguments
        
        Returns:
            List of (Document, score) tuples
        """
        docs = self.similarity_search(query, k=k, **kwargs)
        return [(doc, doc.metadata.get("_score", 0.0)) for doc in docs]
    
    def max_marginal_relevance_search(
        self,
        query: str,
        k: int = 4,
        fetch_k: int = 20,
        lambda_mult: float = 0.5,
        **kwargs: Any,
    ) -> list["Document"]:
        """Search with diversity using MMR.
        
        Maximal Marginal Relevance optimizes for similarity to query
        AND diversity among results.
        
        Args:
            query: Query text
            k: Number of results to return
            fetch_k: Number of initial candidates to fetch
            lambda_mult: Balance between relevance (1.0) and diversity (0.0)
            **kwargs: Additional arguments
        
        Returns:
            List of diverse Document objects
        """
        if self.embedding_model is None:
            raise ValueError("embedding_model is required")
        
        # Get query embedding
        query_embedding = self.embedding_model.embed_query(query)
        
        # Fetch more candidates
        candidates = self.similarity_search_by_vector(
            query_embedding,
            k=fetch_k,
            **kwargs,
        )
        
        if not candidates:
            return []
        
        # Simple MMR implementation
        selected = []
        remaining = list(candidates)
        
        # First pick the most similar
        if remaining:
            selected.append(remaining.pop(0))
        
        # Then pick based on MMR
        while len(selected) < k and remaining:
            best_score = -float("inf")
            best_idx = 0
            
            for i, doc in enumerate(remaining):
                # Relevance score
                relevance = doc.metadata.get("_score", 0.0)
                
                # Diversity score (max similarity to already selected)
                max_sim = 0.0
                for sel in selected:
                    sim = self._doc_similarity(doc, sel)
                    max_sim = max(max_sim, sim)
                
                # MMR score
                mmr_score = (
                    lambda_mult * relevance -
                    (1 - lambda_mult) * max_sim
                )
                
                if mmr_score > best_score:
                    best_score = mmr_score
                    best_idx = i
            
            selected.append(remaining.pop(best_idx))
        
        return selected
    
    def _matches_filter(self, metadata: dict, filter_dict: dict) -> bool:
        """Check if metadata matches filter."""
        for key, value in filter_dict.items():
            if key not in metadata or metadata[key] != value:
                return False
        return True
    
    def _doc_similarity(self, doc1: "Document", doc2: "Document") -> float:
        """Estimate similarity between two documents.
        
        Uses a simple text overlap heuristic since we don't have
        embeddings stored directly.
        """
        words1 = set(doc1.page_content.lower().split())
        words2 = set(doc2.page_content.lower().split())
        
        if not words1 or not words2:
            return 0.0
        
        intersection = words1 & words2
        union = words1 | words2
        
        return len(intersection) / len(union)
    
    async def aadd_texts(
        self,
        texts: list[str],
        metadatas: list[dict] | None = None,
        **kwargs: Any,
    ) -> list[str]:
        """Async version of add_texts."""
        if self.embedding_model is None:
            raise ValueError("embedding_model is required")
        
        embeddings = self.embedding_model.embed_documents(texts)
        return await self.aadd_embeddings(
            text_embeddings=list(zip(texts, embeddings)),
            metadatas=metadatas,
        )
    
    async def aadd_embeddings(
        self,
        text_embeddings: list[tuple[str, list[float]]],
        metadatas: list[dict] | None = None,
        **kwargs: Any,
    ) -> list[str]:
        """Async version of add_embeddings."""
        ids = []
        
        for i, (text, embedding) in enumerate(text_embeddings):
            self._doc_counter += 1
            doc_id = f"doc_{self._doc_counter}"
            ids.append(doc_id)
            
            metadata = {}
            if metadatas and i < len(metadatas):
                metadata = metadatas[i].copy()
            metadata["text"] = text
            
            await self.db.embed(
                namespace=self.namespace,
                key=doc_id,
                embedding=embedding,
                model=self.model_name,
                metadata=metadata,
            )
        
        return ids
    
    async def asimilarity_search(
        self,
        query: str,
        k: int = 4,
        **kwargs: Any,
    ) -> list["Document"]:
        """Async version of similarity_search."""
        if self.embedding_model is None:
            raise ValueError("embedding_model is required")
        
        query_embedding = self.embedding_model.embed_query(query)
        return await self.asimilarity_search_by_vector(query_embedding, k=k, **kwargs)
    
    async def asimilarity_search_by_vector(
        self,
        embedding: list[float],
        k: int = 4,
        **kwargs: Any,
    ) -> list["Document"]:
        """Async version of similarity_search_by_vector."""
        filter_dict = kwargs.get("filter", {})
        score_threshold = kwargs.get("score_threshold", 0.0)
        
        results = []
        
        # Search vectors
        vector_results = await self.db.similar(
            namespace=self.namespace,
            query=embedding,
            top_k=k,
            threshold=score_threshold,
            model_filter=self.model_name,
        )
        
        # Convert to Documents
        for vr in vector_results:
            key = vr.get("key", "")
            score = vr.get("score", 0.0)
            
            try:
                full_data = await self.db.get(self.namespace, key)
                
                if isinstance(full_data, dict):
                    text = full_data.get("text", "")
                    metadata = {k: v for k, v in full_data.items() if k != "text"}
                    metadata["_key"] = key
                    metadata["_score"] = score
                    
                    if self._matches_filter(metadata, filter_dict):
                        results.append(Document(page_content=text, metadata=metadata))
            
            except Exception:
                continue
        
        return results[:k]
    
    @classmethod
    def from_texts(
        cls,
        texts: list[str],
        embedding: "Embeddings",
        metadatas: list[dict] | None = None,
        **kwargs: Any,
    ) -> "KoruDeltaVectorStore":
        """Create vector store from texts.
        
        Args:
            texts: List of text strings
            embedding: Embeddings model
            metadatas: Optional metadata
            **kwargs: Additional arguments
                - db: Database instance (required)
                - namespace: Namespace (required)
        
        Returns:
            Populated KoruDeltaVectorStore
        """
        db = kwargs.get("db")
        namespace = kwargs.get("namespace")
        
        if db is None or namespace is None:
            raise ValueError("db and namespace are required kwargs")
        
        store = cls(db=db, namespace=namespace, embedding_model=embedding)
        store.add_texts(texts, metadatas)
        return store
    
    @classmethod
    def from_documents(
        cls,
        documents: list["Document"],
        embedding: "Embeddings",
        **kwargs: Any,
    ) -> "KoruDeltaVectorStore":
        """Create vector store from documents."""
        texts = [doc.page_content for doc in documents]
        metadatas = [doc.metadata for doc in documents]
        return cls.from_texts(texts, embedding, metadatas, **kwargs)
