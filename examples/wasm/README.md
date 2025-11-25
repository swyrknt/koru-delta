# KoruDelta WASM Example

This example demonstrates using KoruDelta in a web browser via WebAssembly.

## Prerequisites

Install `wasm-pack` if you haven't already:

```bash
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```

## Building

From the project root, run the WASM build script:

```bash
./scripts/build-wasm.sh
```

This will create three WASM builds in `pkg/`:
- `pkg/web/` - ES modules for direct browser use
- `pkg/bundler/` - For webpack and other bundlers
- `pkg/nodejs/` - For Node.js environments

## Running the Demo

After building, serve the example directory with any HTTP server:

```bash
# Using Python
python3 -m http.server 8000

# Or using Node.js http-server
npx http-server -p 8000
```

Then open your browser to:
```
http://localhost:8000/examples/wasm/
```

## What the Demo Shows

The interactive demo lets you:

1. **Store Data** - Put JSON values into namespaced keys
2. **Retrieve Data** - Get current values with version metadata
3. **View History** - See complete change history for any key
4. **Database Stats** - Monitor keys, versions, and namespaces

All operations run entirely in your browser with no backend required!

## Using in Your Project

### Web (ES Modules)

```html
<script type="module">
  import init, { KoruDeltaWasm } from './pkg/web/koru_delta.js';

  async function main() {
    await init();
    const db = await KoruDeltaWasm.new();

    // Store data
    await db.put('users', 'alice', { name: 'Alice', age: 30 });

    // Retrieve data
    const result = await db.get('users', 'alice');
    console.log(result.value);

    // View history
    const history = await db.history('users', 'alice');
    console.log(history);
  }

  main();
</script>
```

### Bundler (Webpack, Vite, etc.)

```javascript
import { KoruDeltaWasm } from 'koru-delta';

const db = await KoruDeltaWasm.new();

// Store data
await db.put('users', 'alice', { name: 'Alice', age: 30 });

// Retrieve data
const result = await db.get('users', 'alice');
console.log(result);
```

### Node.js

```javascript
const { KoruDeltaWasm } = require('./pkg/nodejs/koru_delta');

async function main() {
  const db = await KoruDeltaWasm.new();

  await db.put('users', 'alice', { name: 'Alice', age: 30 });
  const result = await db.get('users', 'alice');
  console.log(result);
}

main();
```

## API Reference

### `KoruDeltaWasm.new()`
Create a new database instance (returns a Promise).

### `put(namespace, key, value)`
Store a JSON value. Returns version metadata.

### `get(namespace, key)`
Retrieve the current value with metadata.

### `history(namespace, key)`
Get the complete version history as an array.

### `getAt(namespace, key, timestampISO)`
Time travel to retrieve value at a specific timestamp.

### `listNamespaces()`
List all namespaces in the database.

### `listKeys(namespace)`
List all keys in a namespace.

### `stats()`
Get database statistics (key count, versions, namespaces).

## Performance Notes

The WASM build is optimized for size and performance:
- Typical package size: ~200-500KB (compressed)
- Operations are in-memory and very fast
- No network latency - everything runs locally
- Perfect for edge computing and offline-first apps

## Use Cases

- **Edge Computing** - Deploy to Cloudflare Workers, Deno Deploy
- **Offline-First Apps** - Full database in the browser
- **Local-First Software** - Keep data on user's device
- **Serverless Functions** - Stateful logic without external DB
- **Development/Testing** - No database setup required
