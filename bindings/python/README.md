# KoruDelta Python Bindings

Python bindings for KoruDelta - The Causal Database.

## Installation

```bash
pip install koru-delta
```

Or install from source:

```bash
git clone https://github.com/swyrknt/koru-delta.git
cd koru-delta/bindings/python
pip install maturin
maturin develop
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

## Run the Examples

```bash
# Quick start - basic operations
python examples/01_quickstart.py

# AI Agent - semantic memory with vectors
python examples/02_ai_agent.py

# Audit Trail - fraud detection with time-travel
python examples/03_audit_trail.py

# Config Management - incident post-mortem
python examples/04_config_management.py
```

## Features

- **Causal Storage**: Every change is versioned and auditable
- **Time-Travel**: Query any historical state with `get_at()`
- **Vector Search**: Built-in semantic search with embeddings
- **Natural Lifecycle**: Hot→Warm→Cold→Deep memory tiers
- **Edge-Ready**: 8MB binary, runs anywhere

## Documentation

- **Python API Reference**: See `docs/` directory for Sphinx documentation
- **LCA Architecture Guide**: Understanding the Local Causal Agent architecture
- **Examples**: Check the `examples/` directory for complete working examples
- **Main Repository**: https://github.com/swyrknt/koru-delta

## Building Documentation

```bash
cd docs
pip install sphinx sphinx-rtd-theme
make html
# Documentation will be in _build/html/
```
