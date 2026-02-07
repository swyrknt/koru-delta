"""
KoruDelta Quickstart Example

This example shows the basics of using KoruDelta from Python.
"""

import asyncio
from koru_delta import Database


async def main():
    # Create an in-memory database
    # (use Config(path="...") for persistence)
    async with Database() as db:
        print("✓ Database connected")
        
        # Store some data
        await db.put("users", "alice", {
            "name": "Alice",
            "email": "alice@example.com",
            "tags": ["developer", "vip"]
        })
        print("✓ Stored user 'alice'")
        
        # Retrieve it
        user = await db.get("users", "alice")
        print(f"✓ Retrieved: {user['name']} ({user['email']})")
        
        # Check if key exists
        exists = await db.contains("users", "alice")
        print(f"✓ Key exists: {exists}")
        
        # List all keys in namespace
        keys = await db.list_keys("users")
        print(f"✓ Keys in 'users': {keys}")
        
        # Store a vector embedding
        await db.embed(
            "documents", "doc1",
            embedding=[0.1, 0.2, 0.3, 0.4, 0.5],
            model="text-embedding-3-small",
            metadata={"title": "Introduction to AI"}
        )
        print("✓ Stored embedding")
        
        # Search for similar vectors
        results = await db.similar(
            "documents",
            query=[0.1, 0.2, 0.3, 0.4, 0.5],
            top_k=5
        )
        print(f"✓ Found {len(results)} similar vectors")
        
        # Get database stats
        stats = await db.stats()
        print(f"✓ Stats: {stats}")
        
        print("\n✓ Demo complete!")


if __name__ == "__main__":
    asyncio.run(main())
