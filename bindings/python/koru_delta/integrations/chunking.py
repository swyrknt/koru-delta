"""
Document chunking utilities for embedding.

Provides intelligent text splitting for RAG pipelines.
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from typing import Iterator, Pattern


@dataclass
class ChunkingConfig:
    """Configuration for document chunking.
    
    Args:
        chunk_size: Target size of each chunk in characters
        chunk_overlap: Number of characters to overlap between chunks
        separators: List of separators to use for splitting, in order of preference
        keep_separator: Whether to keep the separator at the end of chunks
        strip_whitespace: Whether to strip whitespace from chunks
    
    Example:
        >>> # Default config (good for most docs)
        >>> config = ChunkingConfig()
        
        >>> # Large documents with paragraph boundaries
        >>> config = ChunkingConfig(
        ...     chunk_size=2000,
        ...     chunk_overlap=200,
        ...     separators=["\n\n", "\n", ". ", " ", ""]
        ... )
    """
    chunk_size: int = 1000
    chunk_overlap: int = 100
    separators: list[str] | None = None
    keep_separator: bool = True
    strip_whitespace: bool = True
    
    def __post_init__(self):
        if self.separators is None:
            # Default separators in order of semantic importance
            self.separators = ["\n\n", "\n", ". ", "! ", "? ", " ", ""]
        if self.chunk_size <= 0:
            raise ValueError("chunk_size must be positive")
        if self.chunk_overlap < 0:
            raise ValueError("chunk_overlap must be non-negative")
        if self.chunk_overlap >= self.chunk_size:
            raise ValueError("chunk_overlap must be less than chunk_size")


def chunk_document(
    text: str,
    config: ChunkingConfig | None = None,
) -> list[str]:
    """Split a document into chunks for embedding.
    
    Uses recursive character splitting with semantic awareness.
    Attempts to split at natural boundaries (paragraphs, sentences, words)
    before falling back to character-level splitting.
    
    Args:
        text: The document text to chunk
        config: Chunking configuration. Uses defaults if not provided.
    
    Returns:
        List of text chunks
    
    Example:
        >>> text = "First paragraph.\\n\\nSecond paragraph with more content."
        >>> chunks = chunk_document(text, ChunkingConfig(chunk_size=50))
        >>> len(chunks)
        2
    """
    config = config or ChunkingConfig()
    
    if not text:
        return []
    
    # Use recursive text splitter
    splitter = RecursiveCharacterTextSplitter(config)
    return splitter.split_text(text)


class RecursiveCharacterTextSplitter:
    """Recursively split text by different separators.
    
    This follows the LangChain pattern but is implemented independently
    to avoid a required dependency.
    """
    
    def __init__(self, config: ChunkingConfig):
        self.config = config
        self._separators = config.separators or ["\n\n", "\n", ". ", "! ", "? ", " ", ""]
    
    def split_text(self, text: str) -> list[str]:
        """Split text into chunks."""
        return self._split_text_recursive(text, self._separators)
    
    def _split_text_recursive(
        self, text: str, separators: list[str]
    ) -> list[str]:
        """Recursively split text trying each separator."""
        # Base case: if text fits in chunk_size, return it
        if len(text) <= self.config.chunk_size:
            cleaned = text.strip() if self.config.strip_whitespace else text
            return [cleaned] if cleaned else []
        
        # Try each separator
        for i, separator in enumerate(separators):
            if separator == "":
                # Last resort: character-level splitting
                return self._character_split(text)
            
            splits = self._split_with_separator(text, separator)
            
            # Check if any splits are too large
            good_splits = []
            for split in splits:
                if len(split) <= self.config.chunk_size:
                    good_splits.append(split)
                else:
                    # Recursively split large chunks with next separator
                    remaining_separators = separators[i + 1:]
                    if remaining_separators:
                        sub_splits = self._split_text_recursive(
                            split, remaining_separators
                        )
                        good_splits.extend(sub_splits)
                    else:
                        # No more separators, use character split
                        good_splits.extend(self._character_split(split))
            
            # Merge small chunks with overlap
            return self._merge_splits(good_splits, separator)
        
        return self._character_split(text)
    
    def _split_with_separator(self, text: str, separator: str) -> list[str]:
        """Split text using a separator."""
        if not separator:
            return list(text)
        
        # Use regex for more control
        if separator in (". ", "! ", "? "):
            # Keep the punctuation with the sentence
            pattern = f"(?<={re.escape(separator[:-1])}) "
            splits = re.split(pattern, text)
        else:
            splits = text.split(separator)
        
        if self.config.keep_separator and separator:
            # Add separator back to all but the last split
            result = []
            for i, split in enumerate(splits[:-1]):
                if separator in (". ", "! ", "? "):
                    result.append(split + " ")
                else:
                    result.append(split + separator)
            if splits:
                result.append(splits[-1])
            return result
        
        return [s for s in splits if s]
    
    def _character_split(self, text: str) -> list[str]:
        """Split text at character boundaries."""
        chunks = []
        start = 0
        
        while start < len(text):
            end = start + self.config.chunk_size
            chunk = text[start:end]
            
            if self.config.strip_whitespace:
                chunk = chunk.strip()
            
            if chunk:
                chunks.append(chunk)
            
            # Move start forward by chunk_size minus overlap
            start += self.config.chunk_size - self.config.chunk_overlap
        
        return chunks
    
    def _merge_splits(self, splits: list[str], separator: str) -> list[str]:
        """Merge small splits into chunks of appropriate size with overlap."""
        if not splits:
            return []
        
        chunks = []
        current_chunk = []
        current_length = 0
        
        for split in splits:
            split_len = len(split)
            
            # If adding this split would exceed chunk_size, finalize current chunk
            if current_length + split_len > self.config.chunk_size and current_chunk:
                chunk_text = "".join(current_chunk)
                if self.config.strip_whitespace:
                    chunk_text = chunk_text.strip()
                if chunk_text:
                    chunks.append(chunk_text)
                
                # Start new chunk with overlap
                current_chunk = self._get_overlap_chunk(current_chunk)
                current_length = sum(len(s) for s in current_chunk)
            
            current_chunk.append(split)
            current_length += split_len
        
        # Don't forget the last chunk
        if current_chunk:
            chunk_text = "".join(current_chunk)
            if self.config.strip_whitespace:
                chunk_text = chunk_text.strip()
            if chunk_text:
                chunks.append(chunk_text)
        
        return chunks
    
    def _get_overlap_chunk(self, chunks: list[str]) -> list[str]:
        """Get the last portion of chunks for overlap."""
        overlap_size = 0
        overlap_chunks = []
        
        # Work backwards to get up to chunk_overlap characters
        for chunk in reversed(chunks):
            if overlap_size + len(chunk) <= self.config.chunk_overlap:
                overlap_chunks.insert(0, chunk)
                overlap_size += len(chunk)
            else:
                # Partial chunk
                remaining = self.config.chunk_overlap - overlap_size
                if remaining > 0:
                    partial = chunk[-remaining:]
                    overlap_chunks.insert(0, partial)
                break
        
        return overlap_chunks


def count_tokens_approximate(text: str) -> int:
    """Approximate token count (roughly 4 characters per token).
    
    This is a fast approximation. For accurate counts, use tiktoken.
    
    Args:
        text: Text to count tokens for
    
    Returns:
        Approximate number of tokens
    
    Example:
        >>> count_tokens_approximate("Hello world")
        3
    """
    # Rough approximation: ~4 characters per token for English
    return max(1, len(text) // 4)


def chunk_by_tokens(
    text: str,
    target_tokens: int = 500,
    overlap_tokens: int = 50,
) -> list[str]:
    """Chunk text by approximate token count.
    
    Args:
        text: Text to chunk
        target_tokens: Target tokens per chunk
        overlap_tokens: Tokens to overlap between chunks
    
    Returns:
        List of chunks
    
    Example:
        >>> text = "This is a long document. " * 100
        >>> chunks = chunk_by_tokens(text, target_tokens=100, overlap_tokens=10)
        >>> len(chunks) > 1
        True
    """
    # Convert token counts to character counts (approximate)
    chars_per_token = 4
    chunk_size = target_tokens * chars_per_token
    overlap = overlap_tokens * chars_per_token
    
    config = ChunkingConfig(
        chunk_size=chunk_size,
        chunk_overlap=overlap,
    )
    return chunk_document(text, config)
