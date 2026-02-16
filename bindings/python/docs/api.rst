API Reference
=============

This section provides detailed documentation for all public classes and functions
in the KoruDelta Python bindings.

Core Classes
------------

Database
^^^^^^^^

.. class:: Database()

   The main interface to the KoruDelta causal database.

   The Database class provides access to all KoruDelta functionality including
   storage, queries, vector search, and identity management. It follows the
   LCA (Local Causal Agent) architecture where all operations are synthesized
   distinctions.

   **Context Manager Usage:**

   .. code-block:: python

       async with Database() as db:
           await db.put("users", "alice", {"name": "Alice"})

   **Direct Usage:**

   .. code-block:: python

       db = await Database.create()
       await db.put("users", "alice", {"name": "Alice"})
       # ... use db ...
       await db.close()

   .. method:: create() -> Database

      Create a new Database instance.

      :returns: A new Database instance
      :rtype: Database

   .. method:: close()

      Close the database and release resources.

   .. method:: put(namespace: str, key: str, value: dict) -> VersionedValue

      Store a value in the database.

      :param str namespace: The namespace (collection) to store in
      :param str key: The key to store under
      :param dict value: The value to store (must be JSON-serializable)
      :returns: Version information for the stored value
      :rtype: VersionedValue
      :raises InvalidDataError: If the value cannot be serialized

   .. method:: get(namespace: str, key: str) -> dict

      Retrieve a value from the database.

      :param str namespace: The namespace to retrieve from
      :param str key: The key to retrieve
      :returns: The stored value
      :rtype: dict
      :raises KeyNotFoundError: If the key does not exist

   .. method:: delete(namespace: str, key: str) -> bool

      Delete a key from the database.

      :param str namespace: The namespace to delete from
      :param str key: The key to delete
      :returns: True if the key was deleted, False if it didn't exist
      :rtype: bool

   .. method:: contains(namespace: str, key: str) -> bool

      Check if a key exists in the database.

      :param str namespace: The namespace to check
      :param str key: The key to check
      :returns: True if the key exists, False otherwise
      :rtype: bool

   .. method:: history(namespace: str, key: str) -> List[HistoryEntry]

      Get the complete history of a key.

      :param str namespace: The namespace to query
      :param str key: The key to get history for
      :returns: List of history entries in chronological order
      :rtype: List[HistoryEntry]

   .. method:: get_at(namespace: str, key: str, timestamp: str) -> dict

      Get the value of a key at a specific point in time.

      :param str namespace: The namespace to query
      :param str key: The key to retrieve
      :param str timestamp: ISO 8601 timestamp (e.g., "2026-01-15T10:30:00Z")
      :returns: The value at that point in time
      :rtype: dict
      :raises KeyNotFoundError: If no value exists at that timestamp

   .. method:: list_keys(namespace: str) -> List[str]

      List all keys in a namespace.

      :param str namespace: The namespace to list
      :returns: List of keys
      :rtype: List[str]

   .. method:: list_namespaces() -> List[str]

      List all namespaces in the database.

      :returns: List of namespace names
      :rtype: List[str]

   .. method:: query(namespace: str, filters: dict = None, sort: List[str] = None, limit: int = None, offset: int = None) -> QueryResult

      Query the database with filters and sorting.

      :param str namespace: The namespace to query
      :param dict filters: Filter conditions (e.g., {"status": "active"})
      :param List[str] sort: Sort fields (prefix with - for descending)
      :param int limit: Maximum number of results
      :param int offset: Number of results to skip
      :returns: Query results
      :rtype: QueryResult

   .. method:: put_similar(namespace: str, key: str, content: str, metadata: dict = None) -> VersionedValue

      Store content with automatic semantic embedding.

      This method automatically generates a vector embedding from the content
      and stores both the content and its embedding for semantic search.

      :param str namespace: The namespace to store in
      :param str key: The key to store under
      :param str content: The text content to store and embed
      :param dict metadata: Optional metadata to store with the content
      :returns: Version information for the stored value
      :rtype: VersionedValue

   .. method:: find_similar(namespace: str, query: str, top_k: int = 5, threshold: float = 0.0) -> List[SimilarityResult]

      Find semantically similar content.

      :param str namespace: The namespace to search
      :param str query: The query text
      :param int top_k: Maximum number of results
      :param float threshold: Minimum similarity score (0.0 to 1.0)
      :returns: List of similar items with scores
      :rtype: List[SimilarityResult]

   .. method:: put_batch(items: List[BatchItem]) -> List[VersionedValue]

      Store multiple items in a batch (cross-namespace).

      :param List[BatchItem] items: List of items to store
      :returns: List of version information
      :rtype: List[VersionedValue]

   .. method:: put_batch_in_ns(namespace: str, items: List[Tuple[str, dict]]) -> List[VersionedValue]

      Store multiple items in a single namespace.

      :param str namespace: The namespace to store in
      :param items: List of (key, value) tuples
      :returns: List of version information
      :rtype: List[VersionedValue]

   .. method:: create_view(name: str, source_namespace: str, filters: dict = None) -> View

      Create a materialized view.

      :param str name: The view name
      :param str source_namespace: The source namespace
      :param dict filters: Optional filter conditions
      :returns: The created view
      :rtype: View

   .. method:: query_view(name: str) -> QueryResult

      Query a materialized view.

      :param str name: The view name
      :returns: Query results
      :rtype: QueryResult

   .. method:: identities() -> IdentityManager

      Get the identity manager for authentication operations.

      :returns: Identity manager instance
      :rtype: IdentityManager

   .. method:: workspace(name: str) -> Workspace

      Get a workspace handle for isolated memory space.

      :param str name: The workspace name
      :returns: Workspace instance
      :rtype: Workspace

IdentityManager
^^^^^^^^^^^^^^^

.. class:: IdentityManager()

   Manager for self-sovereign identity operations.

   The IdentityManager provides methods for creating, verifying, and managing
   cryptographic identities using proof-of-work mining and Ed25519 signatures.

   .. method:: create(display_name: str = None, description: str = None) -> Identity

      Create a new identity with proof-of-work.

      :param str display_name: Optional display name
      :param str description: Optional description
      :returns: The created identity
      :rtype: Identity

   .. method:: verify(identity_id: str) -> bool

      Verify an identity's validity.

      :param str identity_id: The identity ID to verify
      :returns: True if valid, False otherwise
      :rtype: bool

   .. method:: get(identity_id: str) -> Optional[Identity]

      Get identity information.

      :param str identity_id: The identity ID
      :returns: Identity information or None if not found
      :rtype: Optional[Identity]

Workspace
^^^^^^^^^

.. class:: Workspace()

   Isolated memory space within the database.

   Workspaces provide namespace isolation for different contexts or tenants.
   All operations within a workspace are independent of other workspaces.

   .. method:: put(key: str, value: dict) -> VersionedValue

      Store a value in the workspace.

      :param str key: The key to store under
      :param dict value: The value to store
      :returns: Version information
      :rtype: VersionedValue

   .. method:: get(key: str) -> dict

      Retrieve a value from the workspace.

      :param str key: The key to retrieve
      :returns: The stored value
      :rtype: dict
      :raises KeyNotFoundError: If the key does not exist

   .. method:: list_keys() -> List[str]

      List all keys in the workspace.

      :returns: List of keys
      :rtype: List[str]

   .. method:: delete(key: str) -> bool

      Delete a key from the workspace.

      :param str key: The key to delete
      :returns: True if deleted, False if not found
      :rtype: bool

Data Types
----------

.. class:: VersionedValue()

   Version information for a stored value.

   .. attribute:: value
      :type: dict
      The stored value

   .. attribute:: timestamp
      :type: datetime
      When the value was stored

   .. attribute:: write_id
      :type: str
      Unique identifier for this write

   .. attribute:: distinction_id
      :type: str
      Content hash of the value

.. class:: HistoryEntry()

   A single entry in a key's history.

   .. attribute:: timestamp
      :type: datetime
      When this version was created

   .. attribute:: value
      :type: dict
      The value at this point in time

   .. attribute:: write_id
      :type: str
      Unique identifier for this write

.. class:: SimilarityResult()

   Result from semantic similarity search.

   .. attribute:: namespace
      :type: str
      The namespace of the result

   .. attribute:: key
      :type: str
      The key of the result

   .. attribute:: score
      :type: float
      Similarity score (0.0 to 1.0)

   .. attribute:: value
      :type: dict
      The stored value

.. class:: Identity()

   Self-sovereign identity information.

   .. attribute:: id
      :type: str
      The identity public key

   .. attribute:: display_name
      :type: str
      Human-readable name

   .. attribute:: created_at
      :type: datetime
      When the identity was created

Exceptions
----------

.. exception:: KoruDeltaError()

   Base exception for all KoruDelta errors.

.. exception:: KeyNotFoundError()

   Raised when a requested key does not exist.

.. exception:: InvalidDataError()

   Raised when data cannot be serialized or is invalid.

.. exception:: StorageError()

   Raised when a storage operation fails.

.. exception:: SerializationError()

   Raised when JSON serialization/deserialization fails.

.. exception:: EngineError()

   Raised when the distinction engine encounters an error.

.. exception:: TimeError()

   Raised when a time-travel query fails.
