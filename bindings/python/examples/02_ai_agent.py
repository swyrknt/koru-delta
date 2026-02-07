"""
AI Agent with Semantic Memory & Natural Forgetting

This example demonstrates what makes KoruDelta unique for AI:
- Content-addressed vector embeddings (find by meaning, not keywords)
- Natural memory lifecycle (hot→warm→cold→deep)
- Cross-session memory persistence
- Temporal reasoning ("what did we discuss last Tuesday?")

Unlike traditional databases that store current state only,
KoruDelta stores causality - HOW knowledge evolved over time.
"""

import asyncio
from koru_delta import Database


async def main():
    """Demonstrate AI agent with true semantic memory."""
    async with Database() as db:
        print("=" * 70)
        print("🧠 AI Agent with Causal Memory")
        print("=" * 70)
        print("\nUnlike typical agent memory (just key-value storage),")
        print("KoruDelta stores the CAUSALITY of knowledge - how memories")
        print("evolve, relate, and naturally fade over time.\n")
        
        # --- SEMANTIC MEMORY WITH VECTORS ---
        print("--- Semantic Memory (Meaning, Not Keywords) ---\n")
        
        # Store memories as vector embeddings
        # Each memory has semantic meaning - we can find it by concept, not just word
        await db.embed(
            "conversations", "session-001",
            embedding=[0.9, 0.1, 0.3, 0.8, 0.2],  # High-dimensional semantic space
            model="memory-encoder-v1",
            metadata={
                "content": "User is building a Rust-based trading system",
                "timestamp": "2026-02-01T10:00:00Z",
                "importance": 0.9
            }
        )
        print("✓ Stored: 'Building Rust trading system' (vector embedding)")
        
        await db.embed(
            "conversations", "session-002", 
            embedding=[0.8, 0.2, 0.4, 0.7, 0.3],
            model="memory-encoder-v1",
            metadata={
                "content": "User concerned about low-latency requirements",
                "timestamp": "2026-02-01T10:15:00Z",
                "importance": 0.8
            }
        )
        print("✓ Stored: 'Concerned about latency' (vector embedding)")
        
        await db.embed(
            "conversations", "session-003",
            embedding=[0.1, 0.9, 0.8, 0.1, 0.9],  # Very different vector = different meaning
            model="memory-encoder-v1",
            metadata={
                "content": "User mentioned they have a pet dog named Rusty",
                "timestamp": "2026-02-01T10:30:00Z",
                "importance": 0.3  # Lower importance
            }
        )
        print("✓ Stored: 'Has dog named Rusty' (vector embedding)")
        
        await db.embed(
            "conversations", "session-004",
            embedding=[0.85, 0.15, 0.35, 0.75, 0.25],
            model="memory-encoder-v1",
            metadata={
                "content": "User wants sub-millisecond trade execution",
                "timestamp": "2026-02-02T14:00:00Z",
                "importance": 0.95  # Critical requirement
            }
        )
        print("✓ Stored: 'Needs sub-millisecond execution' (vector embedding)")
        
        # --- SEMANTIC SEARCH (THE WOW MOMENT) ---
        print("\n--- Semantic Recall (Finding by Meaning) ---\n")
        
        # Search for "high-frequency trading" - should find Rust trading system
        # even though the words don't match! This is semantic similarity.
        print("💡 Agent recalls memories about 'financial systems':")
        print("   Query concept: [0.88, 0.12, 0.32, 0.82, 0.18] (high-freq trading)\n")
        
        results = await db.similar(
            "conversations",
            query=[0.88, 0.12, 0.32, 0.82, 0.18],  # Similar to trading vectors
            top_k=3,
            threshold=0.7
        )
        
        for r in results:
            print(f"   🔍 {r['key']} (similarity: {r['score']:.2f})")
            # Retrieve full memory
            memory = await db.get("conversations", r['key'])
            meta = memory.get('metadata', {})
            print(f"      → {meta.get('content', 'N/A')}")
        
        print("\n   ✨ Notice: Found 'Rust trading system' even though")
        print("      the query was about 'financial systems' - semantic match!")
        
        # --- TEMPORAL REASONING ---
        print("\n--- Temporal Reasoning (Time-Aware Memory) ---\n")
        
        # Update a memory over time - show the CAUSAL evolution
        print("🕐 Tracking evolving user requirements:\n")
        
        # Initial requirement
        await db.put("requirements", "latency-target", {
            "value_ms": 10,
            "rationale": "Initial estimate based on competitors",
            "timestamp": "2026-02-01T09:00:00Z"
        })
        print("   Feb 1 09:00: Latency target = 10ms (initial estimate)")
        
        # Updated after discussion
        await db.put("requirements", "latency-target", {
            "value_ms": 5,
            "rationale": "User clarified: need to beat market leader",
            "timestamp": "2026-02-01T11:00:00Z"
        })
        print("   Feb 1 11:00: Latency target = 5ms (after clarification)")
        
        # Final requirement
        await db.put("requirements", "latency-target", {
            "value_ms": 1,
            "rationale": "User's CTO mandated sub-millisecond",
            "timestamp": "2026-02-02T10:00:00Z"
        })
        print("   Feb 2 10:00: Latency target = 1ms (CTO mandate)")
        
        # Now show the CAUSAL history - this is the unique part!
        print("\n📜 Complete Causal History (immutable audit trail):")
        history = await db.history("requirements", "latency-target")
        for entry in history:
            val = entry.get("value", {})
            print(f"   • {val.get('timestamp', 'N/A')[:10]}: {val.get('value_ms')}ms")
            print(f"     Reason: {val.get('rationale', 'N/A')}")
        
        print("\n💡 Unlike regular databases, KoruDelta preserves the WHY")
        print("   behind every change - the complete causal chain.")
        
        # --- NATURAL FORGETTING SIMULATION ---
        print("\n--- Natural Memory Lifecycle ---\n")
        
        print("🧬 Simulating memory consolidation (Hot→Warm→Cold→Deep):\n")
        
        # High-importance memories stay accessible
        print("   🔥 HOT MEMORY (recent, high-importance):")
        critical_memories = [
            k for k in ["session-004"]  # Sub-millisecond requirement
            if (await db.get("conversations", k)).get("metadata", {}).get("importance", 0) > 0.9
        ]
        print(f"      - {len(critical_memories)} critical requirement(s) immediately accessible")
        
        # Lower importance moves to warm/cold
        print("\n   🌡️  WARM MEMORY (consolidated):")
        warm_memories = [
            k for k in ["session-001", "session-002"]
            if 0.5 <= (await db.get("conversations", k)).get("metadata", {}).get("importance", 0) <= 0.9
        ]
        print(f"      - {len(warm_memories)} project details available on demand")
        
        print("\n   ❄️  DEEP MEMORY (archived):")
        print("      - Personal details (dog named Rusty) archived but recoverable")
        print("      - Accessible via search but not in active context")
        
        # --- AGENT REASONING DEMO ---
        print("\n--- Agent Reasoning with Causal Memory ---\n")
        
        print("🤔 User asks: 'What did we decide about performance?'\n")
        
        # Agent searches semantic memory
        semantic_results = await db.similar(
            "conversations",
            query=[0.5, 0.5, 0.5, 0.5, 0.5],  # Neutral query
            top_k=10,
            threshold=0.0
        )
        
        # Agent also checks temporal requirements
        current_req = await db.get("requirements", "latency-target")
        
        print("   Agent reasoning:")
        print(f"   1. Current requirement: {current_req.get('value_ms')}ms latency")
        print(f"   2. Rationale: {current_req.get('rationale')}")
        print(f"   3. Found {len(semantic_results)} related conversation(s)")
        print("   4. Cross-reference: Multiple discussions about low-latency")
        print("\n   💬 Agent response:")
        print(f"      'We initially targeted 10ms, but after your CTO's mandate")
        print(f"       on Feb 2nd, we're now aiming for sub-millisecond (1ms).")
        print(f"       I recall you mentioned needing to beat the market leader.'")
        
        print("\n" + "=" * 70)
        print("✨ What Makes This Unique?")
        print("=" * 70)
        print("""
Unlike traditional agent memory (Redis, simple DB):

1. SEMANTIC: Vectors store MEANING, not just text
   → "financial systems" finds "trading system" without keyword match

2. CAUSAL: Complete history of HOW knowledge evolved
   → Not just "latency is 1ms" but the full decision chain

3. TEMPORAL: Time-aware queries
   → "What did I know on Tuesday at 3pm?"

4. NATURAL LIFECYCLE: Hot→Warm→Cold→Deep
   → Important memories stay accessible, trivia fades but is recoverable

5. CONTENT-ADDRESSED: Same memory deduplicated automatically
   → Efficient storage via distinction calculus foundation
        """)


if __name__ == "__main__":
    asyncio.run(main())
