Examples
========

This section provides example code for common use cases with the KoruDelta
Python bindings.

Basic Operations
----------------

.. code-block:: python

    import asyncio
    from koru_delta import Database

    async def basic_operations():
        async with Database() as db:
            # Store a value
            await db.put("users", "alice", {
                "name": "Alice",
                "email": "alice@example.com",
                "age": 30
            })
            
            # Retrieve the value
            user = await db.get("users", "alice")
            print(f"Name: {user['name']}")
            
            # Check if key exists
            if await db.contains("users", "alice"):
                print("Alice exists!")
            
            # Delete the key
            await db.delete("users", "alice")

    asyncio.run(basic_operations())

Time Travel and History
-----------------------

.. code-block:: python

    import asyncio
    from koru_delta import Database

    async def time_travel():
        async with Database() as db:
            # Store initial value
            await db.put("config", "settings", {"version": 1, "theme": "light"})
            
            # Update the value
            await db.put("config", "settings", {"version": 2, "theme": "dark"})
            
            # View complete history
            history = await db.history("config", "settings")
            for entry in history:
                print(f"{entry.timestamp}: {entry.value}")
            
            # Get value at specific time
            import datetime
            yesterday = (datetime.datetime.now() - datetime.timedelta(days=1)).isoformat()
            try:
                old_value = await db.get_at("config", "settings", yesterday)
                print(f"Yesterday's value: {old_value}")
            except Exception as e:
                print(f"No value at that time: {e}")

    asyncio.run(time_travel())

Semantic Search
---------------

.. code-block:: python

    import asyncio
    from koru_delta import Database

    async def semantic_search():
        async with Database() as db:
            # Store documents with automatic embeddings
            documents = [
                ("doc1", "Machine learning is transforming software development"),
                ("doc2", "Rust provides memory safety without garbage collection"),
                ("doc3", "Python is great for data science and AI"),
                ("doc4", "WebAssembly enables high-performance web applications"),
            ]
            
            for key, content in documents:
                await db.put_similar("docs", key, content, {"type": "article"})
            
            # Search for similar content
            results = await db.find_similar("docs", "programming languages", top_k=3)
            
            print("Similar documents:")
            for result in results:
                print(f"  - {result.key}: {result.score:.2f}")
                print(f"    {result.value}")

    asyncio.run(semantic_search())

Batch Operations
----------------

.. code-block:: python

    import asyncio
    from koru_delta import Database

    async def batch_operations():
        async with Database() as db:
            # Batch insert in single namespace
            items = [
                ("user1", {"name": "Alice", "role": "admin"}),
                ("user2", {"name": "Bob", "role": "user"}),
                ("user3", {"name": "Carol", "role": "user"}),
            ]
            await db.put_batch_in_ns("users", items)
            
            # Cross-namespace batch
            from koru_delta import BatchItem
            
            cross_ns_items = [
                BatchItem(namespace="users", key="user4", value={"name": "David"}),
                BatchItem(namespace="products", key="prod1", value={"name": "Widget"}),
                BatchItem(namespace="orders", key="order1", value={"total": 100}),
            ]
            await db.put_batch(cross_ns_items)

    asyncio.run(batch_operations())

Querying with Filters
---------------------

.. code-block:: python

    import asyncio
    from koru_delta import Database

    async def querying():
        async with Database() as db:
            # Insert test data
            users = [
                ("user1", {"name": "Alice", "age": 30, "active": True}),
                ("user2", {"name": "Bob", "age": 25, "active": False}),
                ("user3", {"name": "Carol", "age": 35, "active": True}),
                ("user4", {"name": "David", "age": 28, "active": True}),
            ]
            await db.put_batch_in_ns("users", users)
            
            # Query active users
            result = await db.query(
                "users",
                filters={"active": True},
                sort=["-age"],  # Sort by age descending
                limit=10
            )
            
            print(f"Found {result.total_count} active users")
            for record in result.records:
                print(f"  - {record.value['name']} (age {record.value['age']})")

    asyncio.run(querying())

Identity Management
-------------------

.. code-block:: python

    import asyncio
    from koru_delta import Database

    async def identity_management():
        async with Database() as db:
            # Get identity manager
            id_mgr = db.identities()
            
            # Create a new identity
            identity = await id_mgr.create(
                display_name="Alice Admin",
                description="System administrator"
            )
            
            print(f"Identity created: {identity.id}")
            print(f"Display name: {identity.display_name}")
            print(f"Created at: {identity.created_at}")
            
            # Verify the identity
            is_valid = await id_mgr.verify(identity.id)
            print(f"Identity valid: {is_valid}")
            
            # Get identity info
            info = await id_mgr.get(identity.id)
            print(f"Retrieved: {info.display_name}")

    asyncio.run(identity_management())

Workspaces
----------

.. code-block:: python

    import asyncio
    from koru_delta import Database

    async def workspaces():
        async with Database() as db:
            # Create isolated workspaces
            project_a = db.workspace("project_a")
            project_b = db.workspace("project_b")
            
            # Store in project A
            await project_a.put("config", {"setting": "value_a"})
            
            # Store in project B (same key, different value)
            await project_b.put("config", {"setting": "value_b"})
            
            # Values are isolated
            val_a = await project_a.get("config")
            val_b = await project_b.get("config")
            
            print(f"Project A: {val_a}")
            print(f"Project B: {val_b}")
            
            # List keys in each workspace
            keys_a = await project_a.list_keys()
            keys_b = await project_b.list_keys()
            
            print(f"Project A keys: {keys_a}")
            print(f"Project B keys: {keys_b}")

    asyncio.run(workspaces())

Materialized Views
------------------

.. code-block:: python

    import asyncio
    from koru_delta import Database

    async def materialized_views():
        async with Database() as db:
            # Insert test data
            for i in range(100):
                await db.put("events", f"event{i}", {
                    "type": "click" if i % 2 == 0 else "view",
                    "user_id": f"user{i % 10}",
                    "value": i
                })
            
            # Create a view for click events
            view = await db.create_view(
                name="click_events",
                source_namespace="events",
                filters={"type": "click"}
            )
            
            # Query the view
            result = await db.query_view("click_events")
            print(f"Click events: {result.total_count}")

    asyncio.run(materialized_views())

Complete AI Agent Example
-------------------------

.. code-block:: python

    import asyncio
    from koru_delta import Database

    async def ai_agent():
        """
        Example AI agent that stores perceptions and queries similar memories.
        """
        async with Database() as db:
            # Create identity for the agent
            id_mgr = db.identities()
            agent_identity = await id_mgr.create(
                display_name="AI Assistant",
                description="Helpful AI agent with semantic memory"
            )
            
            # Workspace for agent's memory
            memory = db.workspace("agent_memory")
            
            # Store perceptions (automatically embedded)
            perceptions = [
                "User asked about Python async patterns",
                "User prefers dark mode interface",
                "User works in Pacific timezone",
                "User is interested in machine learning",
            ]
            
            for i, perception in enumerate(perceptions):
                await db.put_similar(
                    "agent_memory",
                    f"perception_{i}",
                    perception,
                    {"type": "user_preference"}
                )
            
            # Later: Query similar memories
            query = "What does the user like about interfaces?"
            similar = await db.find_similar("agent_memory", query, top_k=3)
            
            print(f"Query: {query}")
            print("Relevant memories:")
            for result in similar:
                print(f"  - {result.score:.2f}: {result.value}")

    asyncio.run(ai_agent())
