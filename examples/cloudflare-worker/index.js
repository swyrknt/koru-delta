/**
 * KoruDelta Cloudflare Worker Example
 * 
 * This example shows how to use KoruDelta in a Cloudflare Worker edge environment.
 * The database runs entirely in the worker with no external dependencies.
 * 
 * Deploy:
 *   wrangler deploy
 */

import { KoruDeltaWasm } from 'koru-delta';

// Initialize database (runs on first request)
let db;
async function getDb() {
    if (!db) {
        db = await KoruDeltaWasm.new();
    }
    return db;
}

export default {
    async fetch(request, env, ctx) {
        const url = new URL(request.url);
        const path = url.pathname;
        
        try {
            // Route handling
            if (path === '/api/put' && request.method === 'POST') {
                return await handlePut(request);
            }
            if (path === '/api/get') {
                return await handleGet(url);
            }
            if (path === '/api/history') {
                return await handleHistory(url);
            }
            if (path === '/api/query') {
                return await handleQuery(url);
            }
            if (path === '/api/namespaces') {
                return await handleListNamespaces();
            }
            if (path === '/api/stats') {
                return await handleStats();
            }
            
            // Default response
            return jsonResponse({
                message: 'KoruDelta Cloudflare Worker',
                endpoints: [
                    'POST /api/put - Store data (body: {namespace, key, value})',
                    'GET /api/get?namespace=&key= - Retrieve data',
                    'GET /api/history?namespace=&key= - Get version history',
                    'GET /api/query?namespace=&filter={} - Query with filters',
                    'GET /api/namespaces - List all namespaces',
                    'GET /api/stats - Database statistics'
                ]
            });
        } catch (err) {
            return jsonResponse({ error: err.message }, 500);
        }
    }
};

async function handlePut(request) {
    const { namespace, key, value } = await request.json();
    
    if (!namespace || !key || value === undefined) {
        return jsonResponse({ error: 'Missing namespace, key, or value' }, 400);
    }
    
    const db = await getDb();
    const result = await db.put(namespace, key, value);
    
    return jsonResponse({
        success: true,
        namespace,
        key,
        versionId: result.versionId,
        timestamp: result.timestamp
    });
}

async function handleGet(url) {
    const namespace = url.searchParams.get('namespace');
    const key = url.searchParams.get('key');
    
    if (!namespace || !key) {
        return jsonResponse({ error: 'Missing namespace or key parameter' }, 400);
    }
    
    const db = await getDb();
    
    try {
        const result = await db.get(namespace, key);
        return jsonResponse(result);
    } catch (e) {
        return jsonResponse({ error: 'Key not found' }, 404);
    }
}

async function handleHistory(url) {
    const namespace = url.searchParams.get('namespace');
    const key = url.searchParams.get('key');
    
    if (!namespace || !key) {
        return jsonResponse({ error: 'Missing namespace or key parameter' }, 400);
    }
    
    const db = await getDb();
    
    try {
        const history = await db.history(namespace, key);
        return jsonResponse({ namespace, key, history });
    } catch (e) {
        return jsonResponse({ error: 'Key not found' }, 404);
    }
}

async function handleQuery(url) {
    const namespace = url.searchParams.get('namespace');
    const filterParam = url.searchParams.get('filter');
    const limit = parseInt(url.searchParams.get('limit') || '10');
    
    if (!namespace) {
        return jsonResponse({ error: 'Missing namespace parameter' }, 400);
    }
    
    let filter = {};
    if (filterParam) {
        try {
            filter = JSON.parse(filterParam);
        } catch (e) {
            return jsonResponse({ error: 'Invalid filter JSON' }, 400);
        }
    }
    
    const db = await getDb();
    const results = await db.query(namespace, filter, limit);
    
    return jsonResponse({
        namespace,
        filter,
        count: results.length,
        results
    });
}

async function handleListNamespaces() {
    const db = await getDb();
    const namespaces = await db.listNamespaces();
    return jsonResponse({ namespaces });
}

async function handleStats() {
    const db = await getDb();
    const stats = await db.stats();
    return jsonResponse(stats);
}

function jsonResponse(data, status = 200) {
    return new Response(JSON.stringify(data, null, 2), {
        status,
        headers: {
            'Content-Type': 'application/json',
            'Access-Control-Allow-Origin': '*',
            'Access-Control-Allow-Methods': 'GET, POST, OPTIONS',
            'Access-Control-Allow-Headers': 'Content-Type'
        }
    });
}
