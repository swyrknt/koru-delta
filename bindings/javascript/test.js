/**
 * KoruDelta JavaScript Tests
 * 
 * Run with: node test.js
 * 
 * These tests verify the WASM bindings work correctly in Node.js.
 */

const { KoruDeltaWasm } = require('./pkg-node/koru_delta.js');

async function runTests() {
  console.log('🧪 KoruDelta JavaScript Tests\n');
  
  let passed = 0;
  let failed = 0;
  
  async function test(name, fn) {
    try {
      await fn();
      console.log(`✅ ${name}`);
      passed++;
    } catch (e) {
      console.log(`❌ ${name}: ${e.message}`);
      failed++;
    }
  }
  
  // Setup
  const db = await KoruDeltaWasm.new();
  
  // Test 1: Create database
  await test('Create database', async () => {
    if (!db) throw new Error('Database not created');
  });
  
  // Test 2: Put and get
  await test('Put and get value', async () => {
    await db.put('test', 'key1', { foo: 'bar', num: 42 });
    const result = await db.get('test', 'key1');
    if (result.value.foo !== 'bar') throw new Error('Value mismatch');
    if (result.value.num !== 42) throw new Error('Number mismatch');
    if (!result.versionId) throw new Error('Missing versionId');
    if (!result.timestamp) throw new Error('Missing timestamp');
  });
  
  // Test 3: Update creates new version
  await test('Update creates new version', async () => {
    await db.put('test', 'versioned', { v: 1 });
    const first = await db.get('test', 'versioned');
    
    await db.put('test', 'versioned', { v: 2 });
    const second = await db.get('test', 'versioned');
    
    if (first.versionId === second.versionId) {
      throw new Error('Version should change on update');
    }
    if (second.value.v !== 2) throw new Error('New value not stored');
  });
  
  // Test 4: History
  await test('History returns versions', async () => {
    await db.put('history', 'item', { step: 1 });
    await db.put('history', 'item', { step: 2 });
    await db.put('history', 'item', { step: 3 });
    
    const history = await db.history('history', 'item');
    if (history.length !== 3) throw new Error(`Expected 3 versions, got ${history.length}`);
    if (history[0].value.step !== 3) throw new Error('History should be newest first');
  });
  
  // Test 5: List namespaces
  await test('List namespaces', async () => {
    await db.put('ns1', 'key', 'value');
    await db.put('ns2', 'key', 'value');
    
    const namespaces = await db.listNamespaces();
    if (!namespaces.includes('ns1')) throw new Error('ns1 not found');
    if (!namespaces.includes('ns2')) throw new Error('ns2 not found');
  });
  
  // Test 6: List keys
  await test('List keys in namespace', async () => {
    await db.put('keys', 'a', '1');
    await db.put('keys', 'b', '2');
    await db.put('keys', 'c', '3');
    
    const keys = await db.listKeys('keys');
    if (keys.length !== 3) throw new Error(`Expected 3 keys, got ${keys.length}`);
  });
  
  // Test 7: Stats
  await test('Database stats', async () => {
    const stats = await db.stats();
    if (typeof stats.keyCount !== 'number') throw new Error('keyCount missing');
    if (typeof stats.totalVersions !== 'number') throw new Error('totalVersions missing');
    if (typeof stats.namespaceCount !== 'number') throw new Error('namespaceCount missing');
    if (stats.keyCount < 1) throw new Error('Should have at least 1 key');
  });
  
  // Test 8: Complex data types
  await test('Store complex data', async () => {
    const complex = {
      string: 'hello',
      number: 42,
      float: 3.14,
      bool: true,
      null: null,
      array: [1, 2, 3],
      nested: { a: { b: { c: 'deep' } } }
    };
    
    await db.put('complex', 'data', complex);
    const result = await db.get('complex', 'data');
    
    if (result.value.nested.a.b.c !== 'deep') {
      throw new Error('Nested data corrupted');
    }
    if (result.value.array.length !== 3) {
      throw new Error('Array corrupted');
    }
  });
  
  // Summary
  console.log('\n' + '='.repeat(40));
  console.log(`Results: ${passed} passed, ${failed} failed`);
  
  if (failed > 0) {
    process.exit(1);
  }
}

runTests().catch(e => {
  console.error('Test runner failed:', e);
  process.exit(1);
});
