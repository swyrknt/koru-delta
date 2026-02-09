/**
 * KoruDelta Node.js Example
 * 
 * This example demonstrates how to use KoruDelta in a Node.js environment.
 * 
 * Prerequisites:
 *   npm install koru-delta
 * 
 * Run:
 *   node index.js
 */

const { KoruDeltaWasm } = require('koru-delta');

async function main() {
    console.log('🌊 KoruDelta Node.js Example\n');

    // Initialize the database
    console.log('Initializing database...');
    const db = await KoruDeltaWasm.new();
    console.log('✓ Database ready\n');

    // Basic CRUD operations
    console.log('=== Basic Operations ===');
    
    // Store some data
    await db.put('users', 'alice', {
        name: 'Alice',
        email: 'alice@example.com',
        age: 30,
        city: 'San Francisco'
    });
    console.log('✓ Stored user: alice');

    await db.put('users', 'bob', {
        name: 'Bob',
        email: 'bob@example.com',
        age: 25,
        city: 'New York'
    });
    console.log('✓ Stored user: bob');

    // Retrieve data
    const alice = await db.get('users', 'alice');
    console.log('\nRetrieved alice:', JSON.stringify(alice, null, 2));

    // Update data (creates new version)
    await db.put('users', 'alice', {
        name: 'Alice',
        email: 'alice@example.com',
        age: 31,  // Changed age
        city: 'San Francisco'
    });
    console.log('✓ Updated alice (age 30 -> 31)\n');

    // View history
    console.log('=== Version History ===');
    const history = await db.history('users', 'alice');
    console.log(`Alice has ${history.length} versions:`);
    history.forEach((entry, i) => {
        console.log(`  ${i + 1}. ${entry.timestamp}: ${JSON.stringify(entry.value)}`);
    });

    // Query with filters
    console.log('\n=== Query with Filters ===');
    const results = await db.query('users', { age: 31 }, 10);
    console.log(`Found ${results.length} users with age = 31:`);
    results.forEach(r => {
        console.log(`  - ${r.key}: ${JSON.stringify(r.value)}`);
    });

    // Vector embeddings
    console.log('\n=== Vector Embeddings ===');
    
    // Store some document embeddings
    const docs = [
        { key: 'doc1', vec: [0.9, 0.1, 0.0, 0.0], text: 'Machine learning tutorial' },
        { key: 'doc2', vec: [0.1, 0.9, 0.0, 0.0], text: 'Cooking recipes' },
        { key: 'doc3', vec: [0.8, 0.2, 0.0, 0.0], text: 'AI deep learning guide' },
    ];

    for (const doc of docs) {
        await db.embed('documents', doc.key, doc.vec, 'embedding-model');
        console.log(`✓ Stored embedding for ${doc.key}: "${doc.text}"`);
    }

    // Search for similar documents
    const queryVec = [0.95, 0.05, 0.0, 0.0];  // Similar to doc1 and doc3
    console.log('\nSearching for documents similar to ML query...');
    const similar = await db.embedSearch('documents', queryVec, 3);
    
    console.log('Results:');
    similar.forEach((result, i) => {
        const doc = docs.find(d => d.key === result.key);
        console.log(`  ${i + 1}. ${result.key} (score: ${result.score.toFixed(4)}): "${doc?.text}"`);
    });

    // Views (materialized queries)
    console.log('\n=== Views ===');
    
    await db.createView('all-users', 'users');
    console.log('✓ Created view: all-users');
    
    const viewResult = await db.queryView('all-users');
    console.log(`View has ${viewResult.totalCount} records`);

    // Database stats
    console.log('\n=== Database Stats ===');
    const stats = await db.stats();
    console.log('Stats:', JSON.stringify(stats, null, 2));

    // List all namespaces
    console.log('\n=== Namespaces ===');
    const namespaces = await db.listNamespaces();
    console.log('Namespaces:', namespaces.join(', '));

    // Cleanup
    console.log('\n=== Cleanup ===');
    await db.delete('users', 'alice');
    await db.delete('users', 'bob');
    console.log('✓ Deleted test data');

    console.log('\n✨ Example complete!');
}

// Run the example
main().catch(err => {
    console.error('Error:', err);
    process.exit(1);
});
