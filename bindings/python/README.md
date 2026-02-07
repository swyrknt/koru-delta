# KoruDelta Python Bindings

Python bindings for KoruDelta - The Causal Database.

## Installation

```bash
pip install koru-delta
```

## Quick Start

```python
import asyncio
from koru_delta import Database

async def main():
    async with Database() as db:
        # Store data
        await db.put("users", "alice", {"name": "Alice"})
        
        # Retrieve
        user = await db.get("users", "alice")
        print(user["name"])  # "Alice"

asyncio.run(main())
```

## Features

- **Causal Storage**: Every change is versioned and auditable
- **Vector Search**: Built-in semantic search with embeddings
- **Workspaces**: Isolated storage with natural lifecycle
- **Edge-Ready**: 8MB binary, runs anywhere

## Documentation

See the [main repository](https://github.com/swyrknt/koru-delta) for full documentation.
