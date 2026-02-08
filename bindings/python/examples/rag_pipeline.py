"""
Full RAG Pipeline Example with KoruDelta.

This example demonstrates a complete Retrieval-Augmented Generation pipeline
using KoruDelta for document storage, hybrid search, and time-travel queries.

Features:
- Document chunking and embedding
- Hybrid search (semantic + causal)
- Time-travel queries (search as of past date)
- Integration with OpenAI for generation

Requirements:
    pip install koru-delta[rag] openai langchain langchain-openai

Example:
    python rag_pipeline.py --docs_dir ./documents --query "What is causal consistency?"
"""

from __future__ import annotations

import argparse
import asyncio
import os
import sys
from datetime import datetime, timedelta
from pathlib import Path
from typing import Any

# Add parent directory to path for local development
sys.path.insert(0, str(Path(__file__).parent.parent))

import koru_delta
from koru_delta import Database
from koru_delta.integrations import (
    chunk_document,
    ChunkingConfig,
    HybridSearcher,
    CausalFilter,
)


# Try to import optional dependencies
try:
    import openai
    OPENAI_AVAILABLE = True
except ImportError:
    OPENAI_AVAILABLE = False
    print("Warning: openai not installed. Generation will be unavailable.")

try:
    from langchain_openai import OpenAIEmbeddings
    LANGCHAIN_AVAILABLE = True
except ImportError:
    LANGCHAIN_AVAILABLE = False
    print("Warning: langchain-openai not installed. Using simple embeddings.")


class SimpleEmbedding:
    """Simple embedding fallback when langchain is not available."""
    
    def __init__(self):
        self.dimension = 128
    
    def embed_documents(self, texts: list[str]) -> list[list[float]]:
        """Create simple hash-based embeddings."""
        import hashlib
        
        embeddings = []
        for text in texts:
            # Use hash to create deterministic pseudo-embeddings
            hash_bytes = hashlib.sha256(text.encode()).digest()
            # Convert to float vector
            vector = [
                ((hash_bytes[i] + hash_bytes[i+1]*256) / 65536.0) * 2 - 1
                for i in range(0, min(len(hash_bytes)-1, self.dimension*2), 2)
            ]
            # Pad or truncate to dimension
            while len(vector) < self.dimension:
                vector.extend(vector[:self.dimension - len(vector)])
            embeddings.append(vector[:self.dimension])
        
        return embeddings
    
    def embed_query(self, text: str) -> list[float]:
        """Embed a query."""
        return self.embed_documents([text])[0]


class RAGPipeline:
    """Complete RAG pipeline using KoruDelta.
    
    This pipeline demonstrates:
    - Document ingestion with chunking
    - Vector embedding and storage
    - Hybrid search (semantic + temporal)
    - Context assembly for LLM generation
    - Time-travel queries
    
    Example:
        >>> pipeline = RAGPipeline(Database())
        >>> 
        >>> # Ingest documents
        >>> await pipeline.ingest_document("path/to/doc.txt")
        >>> 
        >>> # Query
        >>> answer = await pipeline.query("What is the main topic?")
        >>> print(answer)
        
        >>> # Time-travel query
        >>> answer = await pipeline.query_at(
        ...     "What was the status?",
        ...     "2026-01-01T00:00:00Z"
        ... )
    """
    
    def __init__(
        self,
        db: Database,
        embedding_model: Any = None,
        namespace: str = "rag_documents",
        chunk_size: int = 1000,
        chunk_overlap: int = 100,
    ):
        """Initialize RAG pipeline.
        
        Args:
            db: KoruDelta database instance
            embedding_model: Embedding model (OpenAIEmbeddings or similar)
            namespace: Namespace for document storage
            chunk_size: Size of document chunks
            chunk_overlap: Overlap between chunks
        """
        self.db = db
        self.namespace = namespace
        
        # Initialize embedding model
        if embedding_model:
            self.embeddings = embedding_model
        elif LANGCHAIN_AVAILABLE:
            self.embeddings = OpenAIEmbeddings()
        else:
            self.embeddings = SimpleEmbedding()
        
        # Initialize chunking config
        self.chunk_config = ChunkingConfig(
            chunk_size=chunk_size,
            chunk_overlap=chunk_overlap,
        )
        
        # Initialize hybrid searcher
        self.searcher = HybridSearcher(db)
        
        # Track ingested documents
        self._doc_counter = 0
        self._chunk_counter = 0
    
    async def ingest_document(
        self,
        file_path: str | Path,
        metadata: dict | None = None,
    ) -> list[str]:
        """Ingest a document into the RAG pipeline.
        
        Steps:
        1. Read document
        2. Chunk into smaller pieces
        3. Generate embeddings
        4. Store in KoruDelta
        
        Args:
            file_path: Path to document file
            metadata: Optional metadata dict
        
        Returns:
            List of chunk IDs
        """
        file_path = Path(file_path)
        
        if not file_path.exists():
            raise FileNotFoundError(f"Document not found: {file_path}")
        
        # Read document
        text = file_path.read_text(encoding="utf-8")
        
        # Chunk document
        chunks = chunk_document(text, self.chunk_config)
        print(f"  Chunked into {len(chunks)} pieces")
        
        # Generate embeddings
        embeddings = self.embeddings.embed_documents(chunks)
        
        # Store each chunk
        chunk_ids = []
        for i, (chunk_text, embedding) in enumerate(zip(chunks, embeddings)):
            self._chunk_counter += 1
            chunk_id = f"chunk_{self._chunk_counter}"
            chunk_ids.append(chunk_id)
            
            # Build metadata
            chunk_metadata = {
                "source": str(file_path),
                "chunk_index": i,
                "total_chunks": len(chunks),
                "text": chunk_text,
                "ingested_at": datetime.utcnow().isoformat(),
            }
            if metadata:
                chunk_metadata.update(metadata)
            
            # Store embedding
            await self.db.embed(
                namespace=self.namespace,
                key=chunk_id,
                embedding=embedding,
                model="embedding-3-small",
                metadata=chunk_metadata,
            )
        
        self._doc_counter += 1
        print(f"  Ingested {len(chunks)} chunks from {file_path.name}")
        
        return chunk_ids
    
    async def ingest_text(
        self,
        text: str,
        source: str = "unknown",
        metadata: dict | None = None,
    ) -> list[str]:
        """Ingest raw text into the RAG pipeline.
        
        Args:
            text: Text content to ingest
            source: Identifier for the source
            metadata: Optional metadata
        
        Returns:
            List of chunk IDs
        """
        # Chunk text
        chunks = chunk_document(text, self.chunk_config)
        
        # Generate embeddings
        embeddings = self.embeddings.embed_documents(chunks)
        
        # Store each chunk
        chunk_ids = []
        for i, (chunk_text, embedding) in enumerate(zip(chunks, embeddings)):
            self._chunk_counter += 1
            chunk_id = f"chunk_{self._chunk_counter}"
            chunk_ids.append(chunk_id)
            
            chunk_metadata = {
                "source": source,
                "chunk_index": i,
                "total_chunks": len(chunks),
                "text": chunk_text,
                "ingested_at": datetime.utcnow().isoformat(),
            }
            if metadata:
                chunk_metadata.update(metadata)
            
            await self.db.embed(
                namespace=self.namespace,
                key=chunk_id,
                embedding=embedding,
                model="embedding-3-small",
                metadata=chunk_metadata,
            )
        
        return chunk_ids
    
    async def query(
        self,
        question: str,
        top_k: int = 5,
        vector_weight: float = 0.7,
        causal_weight: float = 0.3,
        generate_answer: bool = True,
    ) -> dict[str, Any]:
        """Query the RAG pipeline.
        
        Steps:
        1. Generate query embedding
        2. Hybrid search (semantic + causal relevance)
        3. Assemble context from top results
        4. Generate answer with LLM (optional)
        
        Args:
            question: User question
            top_k: Number of chunks to retrieve
            vector_weight: Weight for semantic similarity
            causal_weight: Weight for causal/temporal relevance
            generate_answer: Whether to generate an answer with LLM
        
        Returns:
            Dict with keys:
                - answer: Generated answer (if generate_answer=True)
                - context: Retrieved context chunks
                - sources: List of source documents
                - hybrid_results: Raw search results
        """
        # Generate query embedding
        query_embedding = self.embeddings.embed_query(question)
        
        # Perform hybrid search
        results = await self.searcher.search(
            query_vector=query_embedding,
            namespace=self.namespace,
            top_k=top_k,
            vector_weight=vector_weight,
            causal_weight=causal_weight,
        )
        
        # Assemble context
        context_chunks = []
        sources = set()
        
        for result in results:
            metadata = result.metadata
            text = ""
            
            if isinstance(result.content, dict):
                text = result.content.get("text", "")
                source = result.content.get("source", result.key)
                sources.add(source)
            
            context_chunks.append({
                "text": text,
                "score": result.combined_score,
                "source": result.key,
            })
        
        # Build context string
        context_str = "\n\n".join([
            f"[Source {i+1}]: {chunk['text']}"
            for i, chunk in enumerate(context_chunks)
        ])
        
        response = {
            "question": question,
            "context": context_chunks,
            "sources": list(sources),
            "hybrid_results": results,
        }
        
        # Generate answer if requested and OpenAI is available
        if generate_answer and OPENAI_AVAILABLE:
            answer = await self._generate_answer(question, context_str)
            response["answer"] = answer
        
        return response
    
    async def query_at(
        self,
        question: str,
        timestamp: str,
        top_k: int = 5,
    ) -> dict[str, Any]:
        """Query as of a specific point in time.
        
        This is a time-travel query - it retrieves documents
        as they existed at a specific historical timestamp.
        
        Args:
            question: User question
            timestamp: ISO 8601 timestamp (e.g., "2026-01-01T00:00:00Z")
            top_k: Number of chunks to retrieve
        
        Returns:
            Dict with answer, context, and historical sources
        """
        # Generate query embedding
        query_embedding = self.embeddings.embed_query(question)
        
        # Time-travel search
        results = await self.searcher.time_travel_search(
            query_vector=query_embedding,
            timestamp=timestamp,
            namespace=self.namespace,
            top_k=top_k,
        )
        
        # Assemble context
        context_chunks = []
        sources = set()
        
        for result in results:
            text = ""
            if isinstance(result.content, dict):
                text = result.content.get("text", "")
                source = result.content.get("source", result.key)
                sources.add(source)
            
            context_chunks.append({
                "text": text,
                "score": result.combined_score,
                "source": result.key,
                "as_of": timestamp,
            })
        
        context_str = "\n\n".join([
            f"[Source {i+1}]: {chunk['text']}"
            for i, chunk in enumerate(context_chunks)
        ])
        
        response = {
            "question": question,
            "timestamp": timestamp,
            "context": context_chunks,
            "sources": list(sources),
        }
        
        # Generate answer
        if OPENAI_AVAILABLE and context_chunks:
            prompt = f"""Based on the following historical information (as of {timestamp}):

{context_str}

Question: {question}

Please provide an answer based only on the information available at that time."""
            
            answer = await self._generate_raw(prompt)
            response["answer"] = answer
        
        return response
    
    async def query_recent(
        self,
        question: str,
        days: int = 7,
        top_k: int = 5,
    ) -> dict[str, Any]:
        """Query only recent documents.
        
        Args:
            question: User question
            days: Only consider documents from last N days
            top_k: Number of chunks to retrieve
        
        Returns:
            Query results from recent documents only
        """
        # Calculate cutoff timestamp
        cutoff = (datetime.utcnow() - timedelta(days=days)).isoformat()
        
        # Generate query embedding
        query_embedding = self.embeddings.embed_query(question)
        
        # Search with temporal filter
        causal_filter = CausalFilter(after_timestamp=cutoff)
        
        results = await self.searcher.search(
            query_vector=query_embedding,
            namespace=self.namespace,
            top_k=top_k,
            causal_filter=causal_filter,
            vector_weight=0.6,
            causal_weight=0.4,  # Higher weight for recency
        )
        
        # Assemble response
        context_chunks = []
        for result in results:
            text = ""
            if isinstance(result.content, dict):
                text = result.content.get("text", "")
            
            context_chunks.append({
                "text": text,
                "score": result.combined_score,
                "source": result.key,
            })
        
        response = {
            "question": question,
            "days": days,
            "context": context_chunks,
        }
        
        if OPENAI_AVAILABLE and context_chunks:
            context_str = "\n\n".join([
                f"[Source {i+1}]: {chunk['text']}"
                for i, chunk in enumerate(context_chunks)
            ])
            answer = await self._generate_answer(question, context_str)
            response["answer"] = answer
        
        return response
    
    async def _generate_answer(
        self,
        question: str,
        context: str,
    ) -> str:
        """Generate answer using OpenAI."""
        if not OPENAI_AVAILABLE:
            return "[OpenAI not available]"
        
        prompt = f"""Use the following context to answer the question.
If the answer cannot be found in the context, say "I don't have enough information to answer that."

Context:
{context}

Question: {question}

Answer:"""
        
        return await self._generate_raw(prompt)
    
    async def _generate_raw(self, prompt: str) -> str:
        """Generate text using OpenAI API."""
        if not OPENAI_AVAILABLE:
            return "[OpenAI not available]"
        
        try:
            client = openai.AsyncOpenAI()
            response = await client.chat.completions.create(
                model="gpt-4o-mini",
                messages=[
                    {
                        "role": "system",
                        "content": "You are a helpful assistant that answers questions based on provided context."
                    },
                    {"role": "user", "content": prompt}
                ],
                temperature=0.3,
                max_tokens=500,
            )
            return response.choices[0].message.content or ""
        except Exception as e:
            return f"[Error generating answer: {e}]"
    
    async def get_stats(self) -> dict[str, Any]:
        """Get pipeline statistics."""
        db_stats = await self.db.stats()
        
        return {
            "documents_ingested": self._doc_counter,
            "chunks_stored": self._chunk_counter,
            "namespace": self.namespace,
            "database": db_stats,
        }


async def demo():
    """Demonstrate the full RAG pipeline."""
    print("=" * 60)
    print("KoruDelta RAG Pipeline Demo")
    print("=" * 60)
    
    # Initialize database
    print("\n1. Initializing database...")
    async with Database() as db:
        # Initialize pipeline
        pipeline = RAGPipeline(db, namespace="demo_docs")
        
        # Ingest sample documents
        print("\n2. Ingesting documents...")
        
        # Sample document 1
        doc1 = """
        Causal consistency is a consistency model used in distributed systems.
        It ensures that processes agree on the order of causally related events.
        If event A causes event B, all processes must observe A before B.
        
        This is weaker than sequential consistency but stronger than eventual consistency.
        It captures the intuition that if one event influences another, the order matters.
        Concurrent events (neither causing the other) can be seen in any order.
        
        Causal consistency was first formalized by Leslie Lamport in 1978 through
        the concept of "happens-before" relationships.
        """
        
        await pipeline.ingest_text(doc1, source="causal_consistency.txt", metadata={"topic": "consistency"})
        
        # Sample document 2
        doc2 = """
        Vector databases store high-dimensional vectors for similarity search.
        They use approximate nearest neighbor (ANN) algorithms like HNSW or IVF
        to efficiently find similar vectors without scanning the entire database.
        
        Common use cases include:
        - Semantic search
        - Recommendation systems
        - Image retrieval
        - Anomaly detection
        
        Vector embeddings are typically generated using neural networks that
        map content (text, images, etc.) into a dense vector space where
        semantic similarity corresponds to vector proximity.
        """
        
        await pipeline.ingest_text(doc2, source="vector_databases.txt", metadata={"topic": "databases"})
        
        # Sample document 3
        doc3 = """
        KoruDelta is a zero-configuration causal database for AI agents.
        It combines Git-like versioning with Redis-like simplicity.
        
        Key features:
        - Automatic causal tracking: Every change is linked to its cause
        - Time travel: Query any historical state
        - Edge deployment: Runs anywhere including browsers
        - Vector search: Native semantic search capabilities
        
        The name comes from "Koru" (Māori for spiral/loop) representing
        continuous growth and "Delta" representing change.
        """
        
        await pipeline.ingest_text(doc3, source="korudelta_intro.txt", metadata={"topic": "product"})
        
        print(f"\n3. Pipeline stats:")
        stats = await pipeline.get_stats()
        print(f"   - Documents: {stats['documents_ingested']}")
        print(f"   - Chunks: {stats['chunks_stored']}")
        
        # Query 1: Basic semantic search
        print("\n4. Query 1: What is causal consistency?")
        print("-" * 40)
        result = await pipeline.query(
            "What is causal consistency?",
            top_k=3,
            generate_answer=False,  # Skip LLM generation for demo
        )
        
        print(f"   Retrieved {len(result['context'])} chunks:")
        for i, chunk in enumerate(result['context']):
            text_preview = chunk['text'][:100].replace('\n', ' ')
            print(f"   [{i+1}] (score: {chunk['score']:.3f}) {text_preview}...")
        
        # Query 2: Vector databases
        print("\n5. Query 2: How do vector databases work?")
        print("-" * 40)
        result = await pipeline.query(
            "How do vector databases work?",
            top_k=3,
            generate_answer=False,
        )
        
        print(f"   Retrieved {len(result['context'])} chunks:")
        for i, chunk in enumerate(result['context']):
            text_preview = chunk['text'][:100].replace('\n', ' ')
            print(f"   [{i+1}] (score: {chunk['score']:.3f}) {text_preview}...")
        
        # Query 3: KoruDelta specific
        print("\n6. Query 3: What is KoruDelta?")
        print("-" * 40)
        result = await pipeline.query(
            "What is KoruDelta?",
            top_k=3,
            generate_answer=False,
        )
        
        print(f"   Retrieved {len(result['context'])} chunks:")
        for i, chunk in enumerate(result['context']):
            text_preview = chunk['text'][:100].replace('\n', ' ')
            print(f"   [{i+1}] (score: {chunk['score']:.3f}) {text_preview}...")
        
        print("\n" + "=" * 60)
        print("Demo complete!")
        print("=" * 60)


def main():
    """Main entry point."""
    parser = argparse.ArgumentParser(
        description="KoruDelta RAG Pipeline Example"
    )
    parser.add_argument(
        "--docs_dir",
        type=str,
        help="Directory containing documents to ingest",
    )
    parser.add_argument(
        "--query",
        type=str,
        help="Query to run",
    )
    parser.add_argument(
        "--demo",
        action="store_true",
        help="Run demo with sample documents",
    )
    
    args = parser.parse_args()
    
    if args.demo or (not args.docs_dir and not args.query):
        # Run demo
        asyncio.run(demo())
    elif args.docs_dir:
        # Ingest documents
        print(f"Ingesting documents from {args.docs_dir}")
        # Implementation would scan directory and ingest files
    elif args.query:
        # Run query
        print(f"Query: {args.query}")
        # Implementation would initialize and run query


if __name__ == "__main__":
    main()
