# KoruDelta CLI Guide

Complete guide to using the `kdelta` command-line tool.

## Installation

### From Source

```bash
cargo install --path .
```

Or build the binary:

```bash
cargo build --release
# Binary will be at: target/release/kdelta
```

## Quick Start

```bash
# Store data
kdelta set users/alice '{"name": "Alice", "age": 30}'

# Retrieve data
kdelta get users/alice

# View history
kdelta log users/alice

# Show database statistics
kdelta status
```

## Commands

### `kdelta set` - Store a Value

Store JSON data in the database.

**Format:**
```bash
kdelta set <namespace>/<key> <value>
```

**Examples:**

```bash
# Store a user object
kdelta set users/alice '{"name": "Alice", "age": 30, "email": "alice@example.com"}'

# Store a simple number
kdelta set counters/visits 42

# Store a string (must be valid JSON)
kdelta set config/theme '"dark"'

# Store an array
kdelta set tags/favorites '["rust", "database", "distributed"]'

# Store nested objects
kdelta set profiles/alice '{
  "name": "Alice",
  "settings": {
    "theme": "dark",
    "notifications": true
  }
}'
```

**Output:**
```
✓
  Stored: users/alice
  Version: 3312c469326df7ef845afecc14c1a22d4e2a46091a8c9e14b1b66f279f524aea
  Timestamp: 2025-11-24 17:00:22 UTC
```

**Notes:**
- Values must be valid JSON
- Strings must be quoted: `'"value"'` not `'value'`
- Updates create new versions (doesn't overwrite history)
- Data is automatically persisted to disk

### `kdelta get` - Retrieve a Value

Retrieve the current value for a key.

**Format:**
```bash
kdelta get <namespace>/<key> [--verbose]
```

**Examples:**

```bash
# Get a value
kdelta get users/alice

# Get with metadata
kdelta get users/alice --verbose
kdelta get users/alice -v
```

**Output (normal):**
```json
{
  "age": 30,
  "name": "Alice"
}
```

**Output (verbose):**
```json
{
  "age": 30,
  "name": "Alice"
}

Metadata:
  Version: 3312c469326df7ef845afecc14c1a22d4e2a46091a8c9e14b1b66f279f524aea
  Timestamp: 2025-11-24 17:00:22 UTC
  Previous: a1b2c3d4...
```

**Error Handling:**
```bash
$ kdelta get users/nonexistent
✗
  Key not found: users/nonexistent
```

### `kdelta log` - View History

Show the complete change history for a key.

**Format:**
```bash
kdelta log <namespace>/<key> [--limit N]
```

**Examples:**

```bash
# Show all history
kdelta log users/alice

# Show last 5 changes
kdelta log users/alice --limit 5
kdelta log users/alice -l 5
```

**Output:**
```
History:

  ● 2025-11-24 17:01:03 UTC
    {
  "age": 31,
  "city": "SF",
  "name": "Alice"
}
    Version: debcd55b6a671c4d2b8c06155a890f1c8d0d4aebac8b70d86d4b7e4fde931f08

  ● 2025-11-24 17:00:22 UTC
    {
  "age": 30,
  "name": "Alice"
}
    Version: 3312c469326df7ef845afecc14c1a22d4e2a46091a8c9e14b1b66f279f524aea

  2 versions total
```

**Notes:**
- History is shown in reverse chronological order (newest first)
- Each entry shows the full value at that point in time
- Version IDs are content-addressed (SHA256 hashes)

### `kdelta diff` - Compare Versions

Show differences between two versions of a key.

**Format:**
```bash
kdelta diff <namespace>/<key> [OPTIONS]
```

**Options:**
- `--at <timestamp>` - Compare value at timestamp with its previous version
- `--from <index>` - Compare from this version index (0 = oldest)
- `--to <index>` - Compare to this version index

**Examples:**

```bash
# Compare latest version with previous (default)
kdelta diff users/alice

# Compare value at specific timestamp with its previous version
kdelta diff users/alice --at "2025-11-24T17:00:00Z"

# Compare specific version indices (0 = oldest)
kdelta diff users/alice --from 0 --to 2
```

**Output:**
```
Comparing versions:

  − Version 1 (2025-11-24 17:23:23 UTC)
  + Version 2 (2025-11-24 17:23:23 UTC)

Diff:

  − Version 1
  + Version 2

    {
      "age": 31,
      "city": "SF",
  −   "name": "Alice",
  +   "name": "Alice Smith",
      "skills": [
        "rust",
  −     "python"
  +     "python",
  +     "go"
      ]
    }
```

**Notes:**
- Red lines (−) show deletions from the old version
- Green lines (+) show additions in the new version
- Gray lines show unchanged content
- Uses line-by-line diff algorithm for JSON values

### `kdelta status` - Database Statistics

Show information about the database.

**Format:**
```bash
kdelta status
```

**Output:**
```
Database Status

  Keys: 3
  Versions: 4
  Namespaces: 2

Namespaces:
  ● counters (1 key)
  ● users (2 keys)

  Database: /Users/sawyerkent/.korudelta/db
```

**Information Shown:**
- Total number of unique keys
- Total number of versions (including history)
- Number of namespaces
- List of namespaces with key counts
- Database file location

### `kdelta list` - List Namespaces or Keys

List all namespaces, or list keys within a namespace.

**Format:**
```bash
kdelta list [namespace]
```

**Examples:**

```bash
# List all namespaces
kdelta list

# List keys in a namespace
kdelta list users
```

**Output (list all):**
```
Namespaces:

  ● counters (1 key)
  ● users (2 keys)
```

**Output (list users):**
```
Keys in 'users':

  ● users/alice
  ● users/bob
```

### `kdelta query` - Query Engine

Execute filtered, sorted, and aggregated queries.

**Format:**
```bash
kdelta query <namespace> [OPTIONS]
```

**Options:**
- `-f, --filter <expr>` - Filter expression
- `-s, --sort <field>` - Sort field
- `--desc` - Sort descending
- `-l, --limit <n>` - Limit results
- `-c, --count` - Count only (no data)

**Filter Syntax:**
- `field = "value"` - Equality
- `field != "value"` - Not equal
- `field > 10` - Greater than
- `field >= 10` - Greater or equal
- `field < 10` - Less than
- `field <= 10` - Less or equal
- `field ~ "pattern"` - Contains substring

**Examples:**

```bash
# Query all users
kdelta query users

# Filter active users
kdelta query users --filter 'status = "active"'

# Filter with multiple conditions (AND)
kdelta query users --filter 'age > 30' --filter 'status = "active"'

# Sort and limit
kdelta query users --sort age --desc --limit 10

# Count only
kdelta query users --filter 'status = "active"' --count
```

**Output:**
```
Query results: (2 records)

  ● alice
    {
      "age": 30,
      "name": "Alice",
      "status": "active"
    }
  ● bob
    {
      "age": 35,
      "name": "Bob",
      "status": "active"
    }
```

### `kdelta view` - Materialized Views

Create and manage materialized views for cached query results.

**Subcommands:**
- `view create <name> <source> [OPTIONS]` - Create a new view
- `view list` - List all views
- `view query <name>` - Query a view
- `view refresh <name>` - Manually refresh a view
- `view delete <name>` - Delete a view

**Create Options:**
- `-f, --filter <filter>` - Filter expression (e.g., 'status = "active"')
- `-s, --sort <field>` - Sort field
- `--desc` - Sort descending
- `-d, --description <text>` - View description

**Examples:**

```bash
# Create a view of active users
kdelta view create active_users users --filter 'status = "active"'

# Create with description and sorting
kdelta view create premium_customers users \
  --filter 'tier = "premium"' \
  --sort created_at --desc \
  --description "High-value customers"

# List all views
kdelta view list

# Query a view (instant, cached results)
kdelta view query active_users

# Refresh view (auto-refreshes on writes by default)
kdelta view refresh active_users

# Delete a view
kdelta view delete active_users
```

**View Output:**
```
View 'active_users' results: (42 records)

  ● alice
    {
      "name": "Alice",
      "status": "active"
    }
  ● bob
    {
      "name": "Bob",
      "status": "active"
    }
```

**Notes:**
- Views persist across database restarts
- Auto-refresh on writes (can be disabled)
- Filter syntax: `field = "value"`, `field > 10`, `field != null`
- Views are stored in the `__views` namespace

## Global Options

### `--db-path` - Custom Database Location

By default, `kdelta` stores data in `~/.korudelta/db`. You can specify a custom location:

```bash
# Use custom database file
kdelta --db-path /tmp/mydb.json set test/key '{"value": 1}'
kdelta --db-path /tmp/mydb.json get test/key

# Short form
kdelta -d /tmp/mydb.json status
```

**Use Cases:**
- Multiple independent databases
- Testing without affecting main database
- Project-specific databases
- Temporary databases

## Key Format

Keys in KoruDelta use the format `namespace/key`.

### Namespace

A logical grouping of related data (similar to tables or collections):

- `users` - User data
- `sessions` - Session information
- `config` - Configuration
- `counters` - Numeric counters
- `cache` - Cached data

### Key

Unique identifier within the namespace:

- `alice`, `bob`, `charlie` - User IDs
- `app`, `database`, `logging` - Config keys
- `visits`, `clicks`, `errors` - Counter names

### Examples

```bash
users/alice          # User alice
users/bob            # User bob
sessions/xyz123      # Session xyz123
config/app           # App config
counters/visits      # Visit counter
cache/user:123       # Cached user data
```

**Naming Guidelines:**
- Use lowercase for consistency
- Avoid special characters (stick to alphanumeric, dash, underscore)
- Use descriptive names
- Keep namespace names short and key names specific

## Data Persistence

### Automatic Saving

Data is automatically saved to disk after every `set` operation. No manual save command needed.

### Database Location

**Default:** `~/.korudelta/db`

**Custom:** Use `--db-path` flag

### Format

Data is stored as JSON (human-readable):

```bash
# You can inspect the database file directly
cat ~/.korudelta/db | jq '.'
```

**Structure:**
```json
{
  "version": 1,
  "current_state": [...],
  "history_log": [...]
}
```

## Advanced Usage

### Multiple Databases

```bash
# Development database
kdelta -d ~/dev.db set test/key '1'

# Production database
kdelta -d ~/prod.db set test/key '2'

# Testing database
kdelta -d /tmp/test.db set test/key '3'
```

### Scripting

```bash
#!/bin/bash

# Store metrics
for i in {1..100}; do
  kdelta set counters/visits "$i"
done

# Read and process
kdelta get counters/visits | jq '.value'
```

### Backup and Restore

```bash
# Backup
cp ~/.korudelta/db ~/backup/korudelta-$(date +%Y%m%d).db

# Restore
cp ~/backup/korudelta-20251124.db ~/.korudelta/db
```

### Querying History Programmatically

```bash
# Get all history as JSON
kdelta log users/alice --limit 1000 > history.txt

# Process with jq or other tools
# (Note: Current output is human-readable, not JSON)
```

## Performance Tips

### Batch Operations

For better performance when inserting many keys:

```bash
# Shell loop (automatic persistence)
for i in {1..1000}; do
  kdelta set items/item$i "{\"id\": $i}"
done
```

### Large Values

KoruDelta handles JSON of any size, but keep in mind:
- Larger values → longer serialization time
- All history is kept (consider data lifecycle)

## Troubleshooting

### "Invalid JSON value"

**Problem:** Value isn't valid JSON

**Solution:**
```bash
# Wrong: bare strings aren't valid JSON
kdelta set config/name hello  # ✗ Error

# Right: strings must be quoted
kdelta set config/name '"hello"'  # ✓ Works
```

### "Invalid key format"

**Problem:** Key doesn't follow `namespace/key` format

**Solution:**
```bash
# Wrong: missing namespace
kdelta set mykey '{"value": 1}'  # ✗ Error

# Right: includes namespace
kdelta set data/mykey '{"value": 1}'  # ✓ Works
```

### "Key not found"

**Problem:** Trying to get a key that doesn't exist

**Solution:**
```bash
# Check if key exists first
kdelta list mynamespace

# Or handle error in scripts
kdelta get myns/mykey || echo "Key not found"
```

## Examples & Recipes

### User Management

```bash
# Create users
kdelta set users/alice '{"name": "Alice", "role": "admin"}'
kdelta set users/bob '{"name": "Bob", "role": "user"}'

# List all users
kdelta list users

# Get user details
kdelta get users/alice

# Update user
kdelta set users/alice '{"name": "Alice", "role": "superadmin"}'

# View change history
kdelta log users/alice
```

### Configuration Management

```bash
# Set configuration
kdelta set config/database '{"host": "localhost", "port": 5432}'
kdelta set config/cache '{"ttl": 3600, "max_size": 1000}'

# Read config
kdelta get config/database | jq '.host'

# Update config
kdelta set config/database '{"host": "db.example.com", "port": 5432}'
```

### Counters and Metrics

```bash
# Initialize counter
kdelta set metrics/requests 0

# Increment (requires reading, incrementing, and writing)
requests=$(kdelta get metrics/requests)
new_count=$((requests + 1))
kdelta set metrics/requests "$new_count"

# View counter history
kdelta log metrics/requests
```

### Session Storage

```bash
# Create session
kdelta set sessions/xyz123 '{
  "user_id": "alice",
  "created_at": "2025-11-24T17:00:00Z",
  "expires_at": "2025-11-24T18:00:00Z"
}'

# Read session
kdelta get sessions/xyz123

# List all sessions
kdelta list sessions
```

## Version Information

```bash
kdelta --version
```

## Help

```bash
# General help
kdelta --help

# Command-specific help
kdelta set --help
kdelta get --help
kdelta log --help
```

## See Also

- [README.md](README.md) - Project overview and quick start
- [ARCHITECTURE.md](ARCHITECTURE.md) - Technical architecture details
- [CONTRIBUTING.md](CONTRIBUTING.md) - Contribution guidelines
