KoruDelta Python Documentation
==============================

**Version:** 3.0.0 (LCA Architecture)

KoruDelta is a zero-configuration, causal database that gives you:

- **Git-like history** - Every change is versioned and auditable
- **Redis-like simplicity** - Minimal API, zero configuration  
- **Mathematical guarantees** - Built on distinction calculus
- **Natural distribution** - Designed for multi-node sync
- **Vector search** - Built-in semantic search with embeddings

The LCA (Local Causal Agent) Architecture
-----------------------------------------

KoruDelta 3.0 implements the **Local Causal Agent** architecture where all state 
transformations follow the synthesis formula:

.. code-block:: text

    ΔNew = ΔLocal_Root ⊕ ΔAction_Data

This means:
- Every operation is a **synthesis** of distinctions
- All state changes are **content-addressed**
- The entire database history is **causally linked**
- Operations are **deterministic** and **reproducible**

Quick Start
-----------

.. code-block:: python

    import asyncio
    from koru_delta import Database

    async def main():
        # Create database (zero configuration)
        db = await Database.create()
        
        # Store data
        await db.put("users", "alice", {"name": "Alice", "age": 30})
        
        # Retrieve data
        user = await db.get("users", "alice")
        print(user)  # {"name": "Alice", "age": 30}
        
        # View history
        history = await db.history("users", "alice")
        for entry in history:
            print(f"{entry.timestamp}: {entry.value}")
        
        # Semantic search
        await db.put_similar("docs", "doc1", "Hello world")
        results = await db.find_similar("docs", "greeting", top_k=5)

    asyncio.run(main())

Table of Contents
-----------------

.. toctree::
   :maxdepth: 2
   :caption: Contents:

   api
   examples
   lca_architecture

API Reference
-------------

Core Classes
^^^^^^^^^^^^

* :class:`koru_delta.Database` - Main database interface
* :class:`koru_delta.IdentityManager` - Self-sovereign identity management
* :class:`koru_delta.Workspace` - Isolated memory spaces

Exceptions
^^^^^^^^^^

* :class:`koru_delta.KoruDeltaError` - Base exception
* :class:`koru_delta.KeyNotFoundError` - Key not found
* :class:`koru_delta.InvalidDataError` - Invalid data format

Indices and tables
==================

* :ref:`genindex`
* :ref:`modindex`
* :ref:`search`
