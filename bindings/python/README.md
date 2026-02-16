# KoruDelta Python Bindings

Python bindings for KoruDelta - The Causal Database with LCA Architecture v3.0.0.

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
    # Create database
    db = await Database.create()
    
    # Store data
    await db.put("users", "alice", {"name": "Alice"})
    
    # Retrieve
    user = await db.get("users", "alice")
    print(user["name"])  # "Alice"

asyncio.run(main())
```

## Cluster Mode (Distributed)

KoruDelta Python supports full cluster mode with automatic write replication:

```python
import asyncio
from koru_delta import ClusterConfig, ClusterNode

async def main():
    # Create first node (seed)
    config1 = ClusterConfig(bind_addr="0.0.0.0:7878")
    node1, db1 = await ClusterNode.start_with_db("/tmp/db1", config1)
    print(f"Node 1: {node1.node_id()} @ {node1.bind_addr()}")
    
    # Create second node joining the first
    config2 = ClusterConfig(
        bind_addr="0.0.0.0:7879",
        join_addr="127.0.0.1:7878"
    )
    node2, db2 = await ClusterNode.start_with_db("/tmp/db2", config2)
    print(f"Node 2: {node2.node_id()} @ {node2.bind_addr()}")
    
    # Write to node 1 - automatically replicates to node 2
    await db1.put("test", "key", {"message": "Hello from node 1"})
    
    # Read from node 2 - sees the replicated data
    import time
    time.sleep(1)  # Wait for replication
    value = await db2.get("test", "key")
    print(value)  # {"message": "Hello from node 1"}
    
    # Check peers
    print(f"Node 1 peers: {node1.peers()}")
    print(f"Node 2 peers: {node2.peers()}")
    
    # Cleanup
    await node1.stop()
    await node2.stop()

asyncio.run(main())
```

## Features

- **Causal Storage**: Every change is versioned and auditable
- **Time-Travel**: Query any historical state with `get_at()`
- **Vector Search**: Built-in semantic search with embeddings
- **Distributed Cluster**: Multi-node with automatic replication
- **Natural Lifecycle**: Hot→Warm→Cold→Deep memory tiers
- **Edge-Ready**: 8MB binary, runs anywhere

## API Reference

### Database Operations

```python
# Basic CRUD
db = await Database.create()
await db.put("namespace", "key", {"data": "value"})
value = await db.get("namespace", "key")
history = await db.history("namespace", "key")
past_value = await db.get_at("namespace", "key", "2024-01-01T00:00:00Z")

# Vector search (semantic similarity)
await db.put_similar("docs", "doc1", "Hello world", {"type": "greeting"})
results = await db.find_similar("docs", "hello query", top_k=5)

# Batch operations
items = [
    {"namespace": "users", "key": "alice", "value": {"name": "Alice"}},
    {"namespace": "users", "key": "bob", "value": {"name": "Bob"}},
]
await db.put_batch(items)

# Queries
results = await db.query("users", filters={"age": {"gt": 18}}, sort="name")
```

### Cluster Operations

```python
from koru_delta import ClusterConfig, ClusterNode

# Create cluster configuration
config = ClusterConfig(
    bind_addr="0.0.0.0:7878",      # Address to bind
    join_addr="192.168.1.100:7878"  # Optional: join existing cluster
)

# Start clustered database
node, db = await ClusterNode.start_with_db("/path/to/db", config)

# Node info
node_id = node.node_id()      # Unique node identifier
address = node.bind_addr()    # Actual bound address
peers = node.peers()          # List of connected peers

# Shutdown
await node.stop()
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

# Cluster - distributed mode (NEW)
python examples/05_cluster.py
```

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

## Platform Support

| Feature | Python Support |
|---------|----------------|
| Core Operations (put/get/history) | ✅ Full |
| Time Travel | ✅ Full |
| Vector Search | ✅ Full |
| Views | ✅ Full |
| Identity/Auth | ✅ Full |
| Cluster/Distributed | ✅ Full |
| Workspaces | ✅ Full |

## Version

Python bindings version: **3.0.0** (matches Rust core)
