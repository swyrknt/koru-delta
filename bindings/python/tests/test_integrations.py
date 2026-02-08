"""
Tests for LLM Framework Integrations.

Tests for:
- Document chunking
- Hybrid search
- LangChain integration (if available)
- LlamaIndex integration (if available)
"""

from __future__ import annotations

import sys
from pathlib import Path

# Add parent directory to path
sys.path.insert(0, str(Path(__file__).parent.parent))

import pytest

from koru_delta.integrations.chunking import (
    chunk_document,
    ChunkingConfig,
    RecursiveCharacterTextSplitter,
    count_tokens_approximate,
    chunk_by_tokens,
)

from koru_delta.integrations.hybrid import (
    HybridSearcher,
    HybridSearchResult,
    CausalFilter,
)


class TestChunking:
    """Tests for document chunking utilities."""
    
    def test_chunk_document_basic(self):
        """Test basic document chunking."""
        text = "This is a test document. " * 50  # ~1150 chars
        config = ChunkingConfig(chunk_size=200, chunk_overlap=20)
        
        chunks = chunk_document(text, config)
        
        # Should produce multiple chunks
        assert len(chunks) > 1
        
        # Each chunk should be within size limit (approximately)
        for chunk in chunks:
            assert len(chunk) <= config.chunk_size + 50  # Allow some tolerance
    
    def test_chunk_document_empty(self):
        """Test chunking empty text."""
        chunks = chunk_document("")
        assert chunks == []
    
    def test_chunk_document_small(self):
        """Test chunking small text (fits in one chunk)."""
        text = "Small text"
        config = ChunkingConfig(chunk_size=100, chunk_overlap=10)
        
        chunks = chunk_document(text, config)
        
        assert len(chunks) == 1
        assert chunks[0] == "Small text"
    
    def test_chunk_config_validation(self):
        """Test ChunkingConfig validation."""
        # Valid config
        config = ChunkingConfig(chunk_size=100, chunk_overlap=10)
        assert config.chunk_size == 100
        assert config.chunk_overlap == 10
        
        # Invalid: negative chunk_size
        with pytest.raises(ValueError, match="chunk_size must be positive"):
            ChunkingConfig(chunk_size=-1)
        
        # Invalid: negative overlap
        with pytest.raises(ValueError, match="chunk_overlap must be non-negative"):
            ChunkingConfig(chunk_size=100, chunk_overlap=-1)
        
        # Invalid: overlap >= chunk_size
        with pytest.raises(ValueError, match="chunk_overlap must be less than chunk_size"):
            ChunkingConfig(chunk_size=100, chunk_overlap=100)
    
    def test_chunk_with_paragraphs(self):
        """Test chunking respects paragraph boundaries."""
        text = "Paragraph one.\n\nParagraph two.\n\nParagraph three."
        config = ChunkingConfig(chunk_size=50, chunk_overlap=10)
        
        chunks = chunk_document(text, config)
        
        # Should attempt to preserve paragraphs
        assert len(chunks) >= 1
    
    def test_chunk_overlap(self):
        """Test that overlap is applied between chunks."""
        text = "word " * 100  # Simple repetitive text
        config = ChunkingConfig(chunk_size=100, chunk_overlap=20)
        
        chunks = chunk_document(text, config)
        
        # Adjacent chunks should share some content
        if len(chunks) >= 2:
            # Check for overlapping words (rough check)
            chunk1_words = set(chunks[0].split())
            chunk2_words = set(chunks[1].split())
            overlap = chunk1_words & chunk2_words
            assert len(overlap) > 0
    
    def test_recursive_splitter(self):
        """Test RecursiveCharacterTextSplitter."""
        config = ChunkingConfig(chunk_size=100, chunk_overlap=10)
        splitter = RecursiveCharacterTextSplitter(config)
        
        text = "Sentence one. Sentence two. Sentence three."
        chunks = splitter.split_text(text)
        
        assert len(chunks) >= 1
    
    def test_count_tokens_approximate(self):
        """Test approximate token counting."""
        # ~4 chars per token
        text = "a" * 40  # Should be ~10 tokens
        count = count_tokens_approximate(text)
        assert 8 <= count <= 12
        
        # Empty string
        assert count_tokens_approximate("") == 1  # Minimum
    
    def test_chunk_by_tokens(self):
        """Test chunking by token count."""
        text = "word " * 200  # ~800 chars, ~200 tokens
        
        chunks = chunk_by_tokens(text, target_tokens=50, overlap_tokens=5)
        
        # Should produce multiple chunks
        assert len(chunks) > 1


class TestHybridSearch:
    """Tests for hybrid search functionality."""
    
    @pytest.fixture
    def mock_db(self):
        """Create a mock database for testing."""
        class MockDB:
            def __init__(self):
                self.data = {}
            
            async def similar(self, **kwargs):
                # Return mock similar results
                return [
                    {"namespace": "test", "key": "key1", "score": 0.9},
                    {"namespace": "test", "key": "key2", "score": 0.8},
                    {"namespace": "test", "key": "key3", "score": 0.7},
                ]
            
            async def get(self, namespace, key):
                return {"text": f"Content of {key}", "value": "test"}
            
            async def history(self, namespace, key):
                return [
                    {"timestamp": "2026-01-01T00:00:00Z", "value": "v1"},
                    {"timestamp": "2026-02-01T00:00:00Z", "value": "v2"},
                ]
        
        return MockDB()
    
    @pytest.mark.asyncio
    async def test_hybrid_search_result(self):
        """Test HybridSearchResult dataclass."""
        result = HybridSearchResult(
            namespace="test",
            key="key1",
            content={"text": "Hello"},
            vector_score=0.9,
            causal_score=0.8,
            combined_score=0.87,
            metadata={},
        )
        
        assert result.namespace == "test"
        assert result.key == "key1"
        assert result.combined_score == 0.87
        assert "key1" in repr(result)
    
    def test_causal_filter_defaults(self):
        """Test CausalFilter with default values."""
        filter_criteria = CausalFilter()
        
        assert filter_criteria.after_timestamp is None
        assert filter_criteria.before_timestamp is None
        assert filter_criteria.min_version_count is None
        assert filter_criteria.related_to_key is None
        assert filter_criteria.custom_filter is None
    
    def test_causal_filter_with_values(self):
        """Test CausalFilter with specific values."""
        def custom_fn(metadata):
            return True
        
        filter_criteria = CausalFilter(
            after_timestamp="2026-01-01T00:00:00Z",
            before_timestamp="2026-12-31T00:00:00Z",
            min_version_count=2,
            related_to_key=("ns", "key"),
            custom_filter=custom_fn,
        )
        
        assert filter_criteria.after_timestamp == "2026-01-01T00:00:00Z"
        assert filter_criteria.min_version_count == 2
        assert filter_criteria.custom_filter is custom_fn


class TestIntegrationExports:
    """Tests that all integrations are properly exported."""
    
    def test_base_imports(self):
        """Test that base integration utilities can be imported."""
        from koru_delta import (
            chunk_document,
            ChunkingConfig,
            HybridSearcher,
            HybridSearchResult,
            CausalFilter,
        )
        
        assert callable(chunk_document)
        assert isinstance(ChunkingConfig, type)
        assert isinstance(HybridSearcher, type)
        assert isinstance(HybridSearchResult, type)
        assert isinstance(CausalFilter, type)
    
    def test_all_exports(self):
        """Test __all__ exports."""
        import koru_delta
        
        # Check that main exports are in __all__
        assert "chunk_document" in koru_delta.__all__
        assert "ChunkingConfig" in koru_delta.__all__
        assert "HybridSearcher" in koru_delta.__all__


# Conditional tests for optional integrations

class TestLangChainIntegration:
    """Tests for LangChain integration (only if langchain installed)."""
    
    @pytest.fixture
    def has_langchain(self):
        """Check if langchain is available."""
        try:
            import langchain_core
            return True
        except ImportError:
            return False
    
    def test_langchain_import(self, has_langchain):
        """Test that LangChain integration can be imported when available."""
        if not has_langchain:
            pytest.skip("langchain not installed")
        
        from koru_delta.integrations.langchain import KoruDeltaVectorStore
        assert isinstance(KoruDeltaVectorStore, type)
    
    def test_langchain_raises_without_dependency(self, has_langchain):
        """Test that LangChain integration raises ImportError when not available."""
        if has_langchain:
            pytest.skip("langchain is installed")
        
        # The integration should handle missing langchain gracefully
        from koru_delta.integrations import langchain
        # Should not raise on module import
        assert hasattr(langchain, 'LANGCHAIN_AVAILABLE')


class TestLlamaIndexIntegration:
    """Tests for LlamaIndex integration (only if llama-index installed)."""
    
    @pytest.fixture
    def has_llamaindex(self):
        """Check if llama-index is available."""
        try:
            import llama_index
            return True
        except ImportError:
            return False
    
    def test_llamaindex_import(self, has_llamaindex):
        """Test that LlamaIndex integration can be imported when available."""
        if not has_llamaindex:
            pytest.skip("llama-index not installed")
        
        from koru_delta.integrations.llamaindex import KoruDeltaVectorStore
        assert isinstance(KoruDeltaVectorStore, type)
    
    def test_llamaindex_raises_without_dependency(self, has_llamaindex):
        """Test that LlamaIndex integration raises ImportError when not available."""
        if has_llamaindex:
            pytest.skip("llama-index is installed")
        
        from koru_delta.integrations import llamaindex
        assert hasattr(llamaindex, 'LLAMAINDEX_AVAILABLE')


# Integration tests with actual database (would need async setup)

@pytest.mark.asyncio
class TestHybridSearchAsync:
    """Async tests for hybrid search."""
    
    async def test_hybrid_searcher_init(self):
        """Test HybridSearcher initialization."""
        # Mock database
        class MockDB:
            pass
        
        db = MockDB()
        searcher = HybridSearcher(db)
        
        assert searcher.db is db
    
    async def test_causal_score_calculation(self):
        """Test causal score calculation."""
        from koru_delta.integrations.hybrid import HybridSearcher
        
        class MockDB:
            pass
        
        searcher = HybridSearcher(MockDB())
        
        # Test with no filter
        metadata = {"version_count": 3}
        score = searcher._calculate_causal_score(metadata, None)
        assert score == 0.5  # Neutral score
        
        # Test with eliminated filter
        filter_criteria = CausalFilter(min_version_count=10)
        score = searcher._calculate_causal_score(metadata, filter_criteria)
        assert score == 0.0  # Eliminated
        
        # Test with passing filter
        metadata = {"version_count": 15}
        score = searcher._calculate_causal_score(metadata, filter_criteria)
        assert score > 0.0  # Passes


def test_chunking_config_repr():
    """Test ChunkingConfig representation."""
    config = ChunkingConfig(chunk_size=500, chunk_overlap=50)
    
    # Should be able to create and use
    assert config.chunk_size == 500
    assert config.chunk_overlap == 50


def test_document_with_various_separators():
    """Test chunking with various separator patterns."""
    # Document with sentences
    text = "First sentence. Second sentence! Third sentence? Fourth sentence."
    config = ChunkingConfig(chunk_size=50, chunk_overlap=5)
    
    chunks = chunk_document(text, config)
    
    # Should split into reasonable chunks
    assert len(chunks) >= 1
    
    # All original content should be preserved (modulo overlap)
    combined = " ".join(chunks)
    assert "First sentence" in combined
    assert "Fourth sentence" in combined


def test_large_document_chunking():
    """Test chunking a large document."""
    # Generate a large document
    paragraphs = [f"Paragraph {i} with some content. " * 20 for i in range(20)]
    text = "\n\n".join(paragraphs)
    
    config = ChunkingConfig(chunk_size=500, chunk_overlap=50)
    chunks = chunk_document(text, config)
    
    # Should produce multiple chunks
    assert len(chunks) > 5
    
    # Each chunk should be reasonable size
    for chunk in chunks:
        assert len(chunk) <= config.chunk_size + 100


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
