"""
Hybrid search combining vector similarity with causal filters.

Provides the ability to search by semantic similarity while also
filtering based on causal properties like time ranges, version history,
and content relationships.
"""

from __future__ import annotations

import math
from dataclasses import dataclass
from typing import TYPE_CHECKING, Callable

if TYPE_CHECKING:
    from koru_delta import Database


@dataclass
class HybridSearchResult:
    """Result from a hybrid search query.
    
    Combines vector similarity score with causal relevance information.
    
    Attributes:
        namespace: The namespace where the result was found
        key: The key identifier
        content: The actual content/value
        vector_score: Similarity score from vector search (0.0-1.0)
        causal_score: Relevance score from causal filters (0.0-1.0)
        combined_score: Weighted combination of vector and causal scores
        metadata: Additional metadata including timestamps, versions
    """
    namespace: str
    key: str
    content: object
    vector_score: float
    causal_score: float
    combined_score: float
    metadata: dict
    
    def __repr__(self) -> str:
        return (
            f"HybridSearchResult({self.namespace}:{self.key}, "
            f"combined={self.combined_score:.3f})"
        )


@dataclass
class CausalFilter:
    """Filter based on causal/temporal properties.
    
    These filters allow querying based on when data was created,
    how it's related to other data, or its position in the causal graph.
    
    Attributes:
        after_timestamp: Only include entries created after this ISO timestamp
        before_timestamp: Only include entries created before this ISO timestamp
        min_version_count: Minimum number of versions in history
        related_to_key: Only include entries related to this key
        custom_filter: Custom predicate function (receives metadata dict)
    """
    after_timestamp: str | None = None
    before_timestamp: str | None = None
    min_version_count: int | None = None
    related_to_key: tuple[str, str] | None = None  # (namespace, key)
    custom_filter: Callable[[dict], bool] | None = None


class HybridSearcher:
    """Hybrid search combining vector similarity with causal filtering.
    
    This searcher first performs vector similarity search, then
    applies causal filters to refine results based on temporal
    and relational properties.
    
    Example:
        >>> from koru_delta import Database
        >>> from koru_delta.integrations import HybridSearcher, CausalFilter
        >>> 
        >>> db = Database()
        >>> searcher = HybridSearcher(db)
        >>> 
        >>> # Search with time filter
        >>> results = await searcher.search(
        ...     query_vector=[0.1, 0.2, ...],
        ...     namespace="documents",
        ...     causal_filter=CausalFilter(after_timestamp="2026-01-01T00:00:00Z"),
        ...     vector_weight=0.7,
        ...     causal_weight=0.3
        ... )
    """
    
    def __init__(self, db: "Database"):
        """Initialize hybrid searcher.
        
        Args:
            db: KoruDelta database instance
        """
        self.db = db
    
    async def search(
        self,
        query_vector: list[float],
        namespace: str | None = None,
        top_k: int = 10,
        vector_threshold: float = 0.0,
        causal_filter: CausalFilter | None = None,
        vector_weight: float = 0.7,
        causal_weight: float = 0.3,
        model_filter: str | None = None,
    ) -> list[HybridSearchResult]:
        """Perform hybrid search.
        
        Args:
            query_vector: The query embedding vector
            namespace: Namespace to search (None = all namespaces)
            top_k: Maximum number of results
            vector_threshold: Minimum vector similarity score
            causal_filter: Optional causal filter criteria
            vector_weight: Weight for vector score (0.0-1.0)
            causal_weight: Weight for causal score (0.0-1.0)
            model_filter: Only search vectors from this embedding model
        
        Returns:
            List of hybrid search results, sorted by combined score
        
        Raises:
            ValueError: If weights don't sum to ~1.0 or are invalid
        """
        # Validate weights
        if not (0.0 <= vector_weight <= 1.0):
            raise ValueError("vector_weight must be between 0.0 and 1.0")
        if not (0.0 <= causal_weight <= 1.0):
            raise ValueError("causal_weight must be between 0.0 and 1.0")
        if abs(vector_weight + causal_weight - 1.0) > 0.01:
            raise ValueError("vector_weight + causal_weight must equal 1.0")
        
        # Step 1: Vector similarity search
        vector_results = await self.db.similar(
            namespace=namespace,
            query=query_vector,
            top_k=top_k * 3,  # Fetch more to allow for causal filtering
            threshold=vector_threshold,
            model_filter=model_filter,
        )
        
        if not vector_results:
            return []
        
        # Step 2: Fetch full content and history for each result
        results_with_metadata = []
        for vr in vector_results:
            ns = vr.get("namespace", namespace or "")
            key = vr.get("key", "")
            vector_score = vr.get("score", 0.0)
            
            try:
                # Get current value
                content = await self.db.get(ns, key)
                
                # Get history for causal analysis
                history = await self.db.history(ns, key)
                
                metadata = {
                    "history": history,
                    "version_count": len(history),
                    "latest_timestamp": history[-1].get("timestamp") if history else None,
                    "earliest_timestamp": history[0].get("timestamp") if history else None,
                }
                
                results_with_metadata.append({
                    "namespace": ns,
                    "key": key,
                    "content": content,
                    "vector_score": vector_score,
                    "metadata": metadata,
                })
            except Exception:
                # Skip entries that can't be retrieved
                continue
        
        # Step 3: Apply causal filtering and scoring
        hybrid_results = []
        for item in results_with_metadata:
            causal_score = self._calculate_causal_score(
                item["metadata"],
                causal_filter,
            )
            
            # Skip if causal filter eliminates this result
            if causal_filter is not None and causal_score <= 0:
                continue
            
            # Calculate combined score
            combined = (
                vector_weight * item["vector_score"] +
                causal_weight * causal_score
            )
            
            hybrid_results.append(HybridSearchResult(
                namespace=item["namespace"],
                key=item["key"],
                content=item["content"],
                vector_score=item["vector_score"],
                causal_score=causal_score,
                combined_score=combined,
                metadata=item["metadata"],
            ))
        
        # Step 4: Sort by combined score and return top_k
        hybrid_results.sort(key=lambda x: x.combined_score, reverse=True)
        return hybrid_results[:top_k]
    
    def _calculate_causal_score(
        self,
        metadata: dict,
        filter_criteria: CausalFilter | None,
    ) -> float:
        """Calculate causal relevance score.
        
        Returns a score between 0.0 and 1.0 based on how well
        the item matches the causal filter criteria.
        """
        if filter_criteria is None:
            # No filter = neutral score
            return 0.5
        
        scores = []
        
        # Check timestamp filters
        latest = metadata.get("latest_timestamp")
        earliest = metadata.get("earliest_timestamp")
        
        if filter_criteria.after_timestamp and latest:
            if latest < filter_criteria.after_timestamp:
                return 0.0  # Eliminated
            scores.append(1.0)
        
        if filter_criteria.before_timestamp and earliest:
            if earliest > filter_criteria.before_timestamp:
                return 0.0  # Eliminated
            scores.append(1.0)
        
        # Check version count
        version_count = metadata.get("version_count", 0)
        if filter_criteria.min_version_count is not None:
            if version_count < filter_criteria.min_version_count:
                return 0.0  # Eliminated
            # Higher version count = more evolved = slightly higher score
            scores.append(min(1.0, version_count / 10.0))
        
        # Apply custom filter
        if filter_criteria.custom_filter is not None:
            if not filter_criteria.custom_filter(metadata):
                return 0.0  # Eliminated
            scores.append(1.0)
        
        # Return average of applicable scores
        if scores:
            return sum(scores) / len(scores)
        return 0.5  # No applicable filters = neutral
    
    async def time_travel_search(
        self,
        query_vector: list[float],
        timestamp: str,
        namespace: str | None = None,
        top_k: int = 10,
    ) -> list[HybridSearchResult]:
        """Search as of a specific point in time.
        
        This returns results based on the state of the database
        at a specific historical timestamp.
        
        Args:
            query_vector: The query embedding vector
            timestamp: ISO 8601 timestamp to search at
            namespace: Namespace to search
            top_k: Maximum results
        
        Returns:
            Results as they would have appeared at that time
        """
        # Use causal filter with before_timestamp
        causal_filter = CausalFilter(before_timestamp=timestamp)
        
        # Get results
        results = await self.search(
            query_vector=query_vector,
            namespace=namespace,
            top_k=top_k * 2,  # Get extra to account for eliminated results
            causal_filter=causal_filter,
            vector_weight=1.0,  # Pure vector search for time travel
            causal_weight=0.0,
        )
        
        # Fetch historical values
        time_travel_results = []
        for result in results:
            try:
                historical_value = await self.db.get_at(
                    result.namespace,
                    result.key,
                    timestamp,
                )
                # Create new result with historical value
                time_travel_results.append(HybridSearchResult(
                    namespace=result.namespace,
                    key=result.key,
                    content=historical_value,
                    vector_score=result.vector_score,
                    causal_score=result.causal_score,
                    combined_score=result.combined_score,
                    metadata={**result.metadata, "as_of_timestamp": timestamp},
                ))
            except Exception:
                # Key didn't exist at that time
                continue
        
        return time_travel_results[:top_k]
    
    async def search_with_temporal_decay(
        self,
        query_vector: list[float],
        namespace: str | None = None,
        top_k: int = 10,
        half_life_days: float = 30.0,
    ) -> list[HybridSearchResult]:
        """Search with exponential temporal decay.
        
        More recent entries get higher scores, with decay based on
        the half-life parameter.
        
        Args:
            query_vector: The query embedding vector
            namespace: Namespace to search
            top_k: Maximum results
            half_life_days: Number of days for score to halve
        
        Returns:
            Results with temporal decay applied to scores
        """
        from datetime import datetime, timezone
        
        # Get base results
        results = await self.search(
            query_vector=query_vector,
            namespace=namespace,
            top_k=top_k * 2,
            vector_weight=1.0,
            causal_weight=0.0,
        )
        
        now = datetime.now(timezone.utc)
        
        # Apply temporal decay
        decayed_results = []
        for result in results:
            timestamp_str = result.metadata.get("latest_timestamp")
            if timestamp_str:
                try:
                    # Parse timestamp
                    timestamp = datetime.fromisoformat(
                        timestamp_str.replace("Z", "+00:00")
                    )
                    age_days = (now - timestamp).total_seconds() / 86400
                    
                    # Apply exponential decay
                    decay_factor = math.exp(-0.693 * age_days / half_life_days)
                    
                    # Adjust combined score
                    adjusted_score = result.vector_score * decay_factor
                    
                    decayed_results.append(HybridSearchResult(
                        namespace=result.namespace,
                        key=result.key,
                        content=result.content,
                        vector_score=result.vector_score,
                        causal_score=decay_factor,  # Use decay as causal score
                        combined_score=adjusted_score,
                        metadata={
                            **result.metadata,
                            "age_days": age_days,
                            "decay_factor": decay_factor,
                        },
                    ))
                except (ValueError, TypeError):
                    # Keep original if timestamp parsing fails
                    decayed_results.append(result)
            else:
                decayed_results.append(result)
        
        # Re-sort by adjusted scores
        decayed_results.sort(key=lambda x: x.combined_score, reverse=True)
        return decayed_results[:top_k]
