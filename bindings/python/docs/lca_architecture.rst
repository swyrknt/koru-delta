LCA Architecture Guide
======================

This guide explains the Local Causal Agent (LCA) architecture that powers
KoruDelta 3.0.

What is LCA?
------------

The **Local Causal Agent (LCA)** architecture is a unified model where:

1. **Everything is an Agent** - The database itself is an agent with a causal perspective
2. **Everything is Synthesis** - All state changes follow the formula: ``ΔNew = ΔLocal ⊕ ΔAction``
3. **Everything is Content-Addressed** - Same content = same identity (via SHA256 hashes)
4. **Everything is Causal** - Complete history preserved in an immutable causal graph

The Synthesis Formula
---------------------

All operations in KoruDelta follow this universal pattern:

.. code-block:: text

    ΔNew = ΔLocal_Root ⊕ ΔAction_Data

Where:

- **ΔLocal_Root** - The agent's current causal perspective (a distinction)
- **ΔAction_Data** - The canonical form of the action being performed
- **⊕** - Synthesis (the fundamental operation of distinction calculus)
- **ΔNew** - The new local root after synthesis

Example: Storing Data
^^^^^^^^^^^^^^^^^^^^^

When you call ``db.put("users", "alice", {"name": "Alice"})``:

1. A ``StorageAction::Store`` is created
2. The action is canonicalized to bytes
3. The bytes are folded through synthesis starting from the Storage root
4. The result becomes the new Storage agent's local root
5. The value is stored with its distinction ID (content hash)

This creates a causal chain: ``Storage_Root → Action_1 → Action_2 → ... → Current_Root``

Canonical Roots
---------------

All agents derive from 19 canonical roots, synthesized from the primordial
distinctions (d0, d1):

.. list-table:: Canonical Roots
   :header-rows: 1

   * - Root
     - Symbol
     - Purpose
   * - FIELD
     - 🌌
     - The unified field itself
   * - ORCHESTRATOR
     - 🎼
     - Agent coordination
   * - STORAGE
     - 💾
     - Memory operations
   * - TEMPERATURE
     - 🔥
     - Activity tracking
   * - CHRONICLE
     - 📜
     - Recent history
   * - ARCHIVE
     - 🗄️
     - Long-term storage
   * - ESSENCE
     - 💎
     - Causal topology
   * - SLEEP
     - 🌙
     - Rhythmic consolidation
   * - EVOLUTION
     - 🧬
     - Natural selection
   * - LINEAGE
     - 👁️
     - Causal ancestry
   * - PERSPECTIVE
     - 🔮
     - View management
   * - IDENTITY
     - 🎭
     - Authentication
   * - NETWORK
     - 🌐
     - Distributed awareness
   * - WORKSPACE
     - 📁
     - Memory isolation
   * - VECTOR
     - 🔢
     - Embeddings
   * - LIFECYCLE
     - 🔄
     - Tier transitions
   * - SESSION
     - 🔑
     - Auth sessions
   * - SUBSCRIPTION
     - 📡
     - Pub/sub
   * - PROCESS
     - ⚙️
     - Background tasks
   * - RECONCILIATION
     - 🤝
     - Distributed sync

Action Types
------------

All 19 action types implement ``Canonicalizable``:

.. code-block:: python

    # Storage actions
    StorageAction::Store { namespace, key, value_json }
    StorageAction::Retrieve { namespace, key }
    StorageAction::History { namespace, key }
    StorageAction::Query { pattern_json }
    StorageAction::Delete { namespace, key }

    # Temperature actions
    TemperatureAction::Heat { key }
    TemperatureAction::Cool { key }
    TemperatureAction::Access { key }
    TemperatureAction::Evict { key }

    # And 17 more action types...

Content Addressing
------------------

Every value in KoruDelta is content-addressed:

.. code-block:: python

    # These two stores create the same distinction_id
    await db.put("ns1", "key1", {"name": "Alice"})
    await db.put("ns2", "key2", {"name": "Alice"})

    # Both point to the same underlying value storage
    # This provides automatic deduplication

The distinction_id is computed as:

1. Serialize value to canonical JSON
2. Compute SHA256 hash
3. The hash is the distinction_id

Benefits of LCA
---------------

1. **Determinism**
   Same action + same root = same distinction. Operations are reproducible.

2. **Auditability**
   Every operation leaves a causal trace. Complete history is preserved.

3. **Composability**
   Agents can be combined through synthesis. Cross-agent operations are natural.

4. **Distributed-Ready**
   Distinctions are universal identifiers. Same content has same ID everywhere.

5. **Time Travel**
   Query any past state by traversing causal chains.

6. **Automatic Deduplication**
   Content-addressing means identical values share storage.

Comparison with Traditional Databases
-------------------------------------

.. list-table:: LCA vs Traditional
   :header-rows: 1

   * - Aspect
     - Traditional DB
     - LCA (KoruDelta)
   * - State Changes
     - UPDATE/DELETE (destructive)
     - Synthesis (append-only)
   * - Identity
     - Row ID (arbitrary)
     - Content hash (meaningful)
   * - History
     - Optional/logging
     - Core/fundamental
   * - Distribution
     - Complex consensus
     - Natural content-addressing
   * - Query Model
     - Current state only
     - Any point in time
   * - Storage Model
     - Tabular
     - Causal graph

Further Reading
---------------

- `KoruDelta Architecture <https://github.com/swyrknt/koru-delta/blob/main/ARCHITECTURE.md>`_
- `Distinction Calculus <https://github.com/swyrknt/koru-lambda-core>`_
- `Python API Reference <api.html>`_
