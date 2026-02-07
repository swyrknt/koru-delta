"""
AI Agent Memory Example

Demonstrates KoruDelta's agent memory system with:
- Episodic memory (conversation history)
- Semantic memory (facts about users)
- Procedural memory (how-to knowledge)
- Memory recall with relevance scoring
"""

import asyncio
from koru_delta import Database


async def main():
    """Demonstrate AI agent memory capabilities."""
    async with Database() as db:
        print("=" * 60)
        print("AI Agent Memory Demo")
        print("=" * 60)
        
        # Create an agent memory interface
        memory = db.agent_memory("assistant-42")
        print("\n✓ Agent memory initialized")
        
        # --- EPISODIC MEMORY: Store conversation events ---
        print("\n--- Episodic Memory (Events) ---")
        
        await memory.episodes.remember(
            "User asked about Python bindings for KoruDelta",
            importance=0.8,
            tags=["python", "bindings", "question"]
        )
        print("✓ Stored: User question about Python bindings")
        
        await memory.episodes.remember(
            "User mentioned they work at a fintech startup",
            importance=0.7,
            tags=["personal", "work", "fintech"]
        )
        print("✓ Stored: User works at fintech startup")
        
        await memory.episodes.remember(
            "User expressed concern about data privacy",
            importance=0.9,
            tags=["privacy", "security", "concern"]
        )
        print("✓ Stored: User privacy concern")
        
        # --- SEMANTIC MEMORY: Store facts ---
        print("\n--- Semantic Memory (Facts) ---")
        
        await memory.facts.learn(
            "user_name",
            "User's name is Alice Johnson",
            tags=["personal", "identity"]
        )
        print("✓ Learned: User's name is Alice Johnson")
        
        await memory.facts.learn(
            "tech_stack",
            "User's team uses Python, Rust, and Kubernetes",
            tags=["tech", "stack"]
        )
        print("✓ Learned: User's tech stack")
        
        # --- PROCEDURAL MEMORY: Store how-to knowledge ---
        print("\n--- Procedural Memory (How-To) ---")
        
        await memory.procedures.learn(
            "explain_causal_db",
            steps=[
                "1. Define causal database concept",
                "2. Give example of time travel queries",
                "3. Show edge deployment benefits",
                "4. Mention vector search capabilities"
            ],
            success_rate=0.95
        )
        print("✓ Learned: How to explain causal databases")
        
        await memory.procedures.learn(
            "handle_privacy_question",
            steps=[
                "1. Acknowledge privacy concern",
                "2. Explain local-first architecture",
                "3. Mention no cloud dependency",
                "4. Offer audit trail capabilities"
            ],
            success_rate=0.90
        )
        print("✓ Learned: How to handle privacy questions")
        
        # --- MEMORY STATS ---
        print("\n--- Memory Stats ---")
        stats = await memory.stats()
        print(f"Total memories: {stats['total']}")
        print(f"  - Episodic: {stats.get('episodic', 0)}")
        print(f"  - Semantic: {stats.get('semantic', 0)}")
        print(f"  - Procedural: {stats.get('procedural', 0)}")
        
        # --- MEMORY RECALL ---
        print("\n--- Memory Recall ---")
        
        # Query 1: Recall about Python
        print("\nQuery: 'Tell me about Python'")
        results = await memory.recall("Python", limit=3)
        for r in results:
            print(f"  [{r.relevance:.2f}] {r.content[:60]}...")
        
        # Query 2: Recall about privacy
        print("\nQuery: 'What about privacy concerns?'")
        results = await memory.recall("privacy", limit=3)
        for r in results:
            print(f"  [{r.relevance:.2f}] {r.content[:60]}...")
        
        # Query 3: Recall tech stack info
        print("\nQuery: 'What technology do they use?'")
        results = await memory.recall("technology stack", limit=3)
        for r in results:
            print(f"  [{r.relevance:.2f}] {r.content[:60]}...")
        
        print("\n" + "=" * 60)
        print("✓ AI Agent Memory Demo Complete!")
        print("=" * 60)


if __name__ == "__main__":
    asyncio.run(main())
