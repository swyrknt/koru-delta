/**
 * KoruDelta Deno Example
 * 
 * This example demonstrates using KoruDelta with Deno.
 * 
 * Run:
 *   deno run --allow-read --allow-net index.ts
 */

// Import the WASM module (adjust path as needed)
import init, { KoruDeltaWasm } from "../../bindings/js/pkg-web/koru_delta.js";

// Initialize WASM
await init();

console.log("🌊 KoruDelta Deno Example\n");

// Create database instance
const db = await KoruDeltaWasm.new();
console.log("✓ Database initialized\n");

// Store some data
console.log("=== Storing Data ===");
await db.put("products", "laptop", {
    name: "Gaming Laptop",
    price: 1299.99,
    category: "electronics",
    inStock: true
});
console.log("✓ Stored: laptop");

await db.put("products", "mouse", {
    name: "Wireless Mouse",
    price: 29.99,
    category: "electronics",
    inStock: true
});
console.log("✓ Stored: mouse");

await db.put("products", "desk", {
    name: "Standing Desk",
    price: 499.99,
    category: "furniture",
    inStock: false
});
console.log("✓ Stored: desk\n");

// Query all products
console.log("=== All Products ===");
const allProducts = await db.query("products", {}, 10);
console.log(`Found ${allProducts.length} products:`);
for (const product of allProducts) {
    console.log(`  - ${product.key}: ${product.value.name} ($${product.value.price})`);
}

// Query with filter
console.log("\n=== Electronics Only ===");
const electronics = await db.query("products", { category: "electronics" }, 10);
console.log(`Found ${electronics.length} electronics:`);
for (const product of electronics) {
    console.log(`  - ${product.value.name}`);
}

// Demonstrate versioning
console.log("\n=== Versioning Demo ===");
await db.put("inventory", "widget", { count: 100, lastUpdated: new Date().toISOString() });
console.log("✓ Initial: 100 widgets");

await db.put("inventory", "widget", { count: 95, lastUpdated: new Date().toISOString() });
console.log("✓ Updated: 95 widgets");

await db.put("inventory", "widget", { count: 87, lastUpdated: new Date().toISOString() });
console.log("✓ Updated: 87 widgets\n");

const history = await db.history("inventory", "widget");
console.log(`History shows ${history.length} versions:`);
for (const entry of history) {
    console.log(`  ${entry.timestamp}: ${entry.value.count} units`);
}

// Vector embeddings
console.log("\n=== Vector Embeddings ===");

// Store some embeddings
const embeddings = [
    { key: "article1", vec: [0.9, 0.1, 0.0], title: "Rust Programming" },
    { key: "article2", vec: [0.1, 0.9, 0.0], title: "Cooking Basics" },
    { key: "article3", vec: [0.85, 0.15, 0.0], title: "Rust for Systems" },
];

for (const emb of embeddings) {
    await db.embed("articles", emb.key, emb.vec, "text-embedding-3-small");
    console.log(`✓ Embedded: ${emb.title}`);
}

// Search for similar articles
console.log("\nSearching for articles about Rust...");
const queryVec = [0.95, 0.05, 0.0];
const searchResults = await db.embedSearch("articles", queryVec, 5);

console.log("Results:");
for (const result of searchResults) {
    const article = embeddings.find(e => e.key === result.key);
    console.log(`  - ${article?.title} (score: ${result.score.toFixed(4)})`);
}

// Namespaces
console.log("\n=== Namespaces ===");
const namespaces = await db.listNamespaces();
console.log(`Database contains ${namespaces.length} namespaces:`);
for (const ns of namespaces) {
    console.log(`  - ${ns}`);
}

// Stats
console.log("\n=== Database Stats ===");
const stats = await db.stats();
console.log(JSON.stringify(stats, null, 2));

console.log("\n✨ Deno example complete!");
