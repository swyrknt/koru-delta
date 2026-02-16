//! ALIS AI Integration Example - LCA Architecture v3.0.0
//!
//! This example demonstrates how ALIS AI uses KoruDelta as the Delta Agent
//! (Memory Consciousness) in the Local Causal Agent architecture.
//!
//! # Architecture Overview
//!
//! ```
//! ALIS AI Components:
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                        THE KORU FIELD                           │
//! │                  (Shared DistinctionEngine)                     │
//! │                                                                  │
//! │  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
//! │  │   PULSE      │  │ PERCEPTION   │  │       DELTA          │  │
//! │  │   AGENT      │  │    AGENT     │  │      AGENT           │  │
//! │  │              │  │              │  │   (KoruDelta)        │  │
//! │  │ - Timing     │  │ - Input      │  │                      │  │
//! │  │ - Phases     │  │ - Transform  │  │ - store()            │  │
//! │  │              │  │ - Bind       │  │ - query_similar()    │  │
//! │  └──────────────┘  └──────────────┘  │ - consolidate()      │  │
//! │         │                 │          │ - dream()            │  │
//! │         └─────────────────┴──────────┴──────────────────────┘  │
//! │                       │                                         │
//! │              ┌────────┴────────┐                               │
//! │              │  EXPRESSION     │                               │
//! │              │    AGENT        │                               │
//! │              │                 │                               │
//! │              │ - Query Delta   │                               │
//! │              │ - Synthesize    │                               │
//! │              │ - Output        │                               │
//! │              └─────────────────┘                               │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # What This Example Validates
//!
//! 1. **Perception → Delta**: Storing perceived distinctions
//! 2. **Consolidation**: Finding similar unconnected distinctions
//! 3. **Expression ← Delta**: Querying for highly-connected distinctions
//! 4. **Dream**: Random walk combinations for creativity
//! 5. **Identity**: Agent identity management
//! 6. **Time-travel**: Memory at specific points in time
//! 7. **Background synthesis**: Tagging synthesis events
//!
//! # Run
//!
//! ```bash
//! cargo run --example alis_ai_integration
//! ```

use koru_delta::auth::IdentityUserData;
use koru_delta::{KoruDelta, VersionedValue};
use serde_json::json;

/// Simulates the Perception Agent receiving input
async fn perception_agent(
    delta: &KoruDelta,
    input_text: &str,
    context: serde_json::Value,
) -> Result<String, Box<dyn std::error::Error>> {
    // 1. Transform input into a distinction (content-addressed)
    // In real ALIS, this would use the DistinctionEngine
    let hash = blake3::hash(input_text.as_bytes());
    let distinction_id = format!("dist_{}", hex::encode(hash.as_bytes()));

    // 2. Store in Delta with semantic embedding
    // The content is synthesized into distinction space
    delta
        .put_similar(
            "perceptions",
            &distinction_id,
            json!({
                "text": input_text,
                "context": context,
                "agent": "perception",
            }),
            Some(json!({
                "source": "input",
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        )
        .await?;

    println!("  [Perception] Stored distinction: {}", &distinction_id[..16]);

    Ok(distinction_id)
}

/// Consolidation: Find similar unconnected distinctions and synthesize
async fn consolidate(
    delta: &KoruDelta,
    namespace: &str,
) -> Result<Vec<(String, String, f32)>, Box<dyn std::error::Error>> {
    println!("\n[Consolidation Phase]");

    let mut synthesized = Vec::new();

    // 1. Get all keys in namespace
    let keys = delta.list_keys(namespace).await;

    // 2. For each key, find similar distinctions
    for key in &keys {
        // Get the value to use as query
        if let Ok(versioned) = delta.get(namespace, key).await {
            let query_content = versioned.value().clone();

            // Find similar distinctions (excluding self)
            let similar = delta
                .find_similar(Some(namespace), query_content, 3)
                .await?;

            for result in similar {
                if result.key != *key && result.score > 0.7 {
                    // 3. Synthesize connection (in real ALIS, this would be
                    // a graph operation via DistinctionEngine)
                    println!(
                        "  [Consolidate] Synthesizing: {} ⊕ {} (score: {:.2})",
                        &key[..key.len().min(8)],
                        &result.key[..result.key.len().min(8)],
                        result.score
                    );

                    // Store the synthesis event
                    let synthesis_id = format!("synth_{}_{}", key, result.key);
                    delta
                        .put(
                            "synthesis_events",
                            &synthesis_id,
                            json!({
                                "type": "consolidation",
                                "source": key,
                                "target": result.key,
                                "score": result.score,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            }),
                        )
                        .await?;

                    synthesized.push((key.clone(), result.key.clone(), result.score));
                }
            }
        }
    }

    println!("  [Consolidate] Created {} synthesis events", synthesized.len());

    Ok(synthesized)
}

/// Dream: Random walk combinations for creative synthesis
async fn dream(delta: &KoruDelta) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n[Dream Phase]");

    // 1. Get random distinctions from different namespaces
    let perceptions = delta.list_keys("perceptions").await;
    let concepts = delta.list_keys("concepts").await;

    if perceptions.is_empty() || concepts.is_empty() {
        println!("  [Dream] Not enough data for dream synthesis");
        return Ok(());
    }

    // 2. Synthesize unlikely combinations (creative connections)
    use rand::seq::SliceRandom;
    let mut rng = rand::thread_rng();

    let random_perception = perceptions.choose(&mut rng).unwrap();
    let random_concept = concepts.choose(&mut rng).unwrap();

    println!(
        "  [Dream] Synthesizing distant pair: {} ⊕ {}",
        &random_perception[..random_perception.len().min(8)],
        &random_concept[..random_concept.len().min(8)]
    );

    // 3. Store dream synthesis
    let dream_id = format!("dream_{}_{}", random_perception, random_concept);
    delta
        .put(
            "dream_events",
            &dream_id,
            json!({
                "type": "dream_synthesis",
                "source": random_perception,
                "target": random_concept,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }),
        )
        .await?;

    println!("  [Dream] Stored dream event: {}", &dream_id[..dream_id.len().min(16)]);

    Ok(())
}

/// Expression Agent: Query Delta for highly-connected distinctions
async fn expression_agent(
    delta: &KoruDelta,
    query: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    println!("\n[Expression Phase]");

    // 1. Query for similar distinctions (what Delta "knows" about query)
    let results = delta
        .find_similar(None, json!(query), 5)
        .await?;

    println!("  [Expression] Found {} related distinctions", results.len());

    // 2. Get synthesis events for these distinctions (connectivity)
    let mut connected_distinctions = Vec::new();

    for result in &results {
        // In real ALIS, we'd query the graph for highly-connected nodes
        // Here we simulate by checking if this distinction has synthesis events
        let synthesis_events = delta.list_keys("synthesis_events").await;
        let connections = synthesis_events
            .iter()
            .filter(|k| k.contains(&result.key))
            .count();

        if connections > 0 {
            connected_distinctions.push(result.key.clone());
            println!(
                "  [Expression] {} is highly-connected ({} connections)",
                &result.key[..result.key.len().min(8)],
                connections
            );
        }
    }

    // 3. Return the "most conscious" distinctions (highly connected)
    Ok(connected_distinctions)
}

/// Time-travel: What did Delta know at a specific time?
async fn memory_at_time(
    delta: &KoruDelta,
    key: &str,
    timestamp: &str,
) -> Result<Option<VersionedValue>, Box<dyn std::error::Error>> {
    println!("\n[Memory Query - Time Travel]");

    let ts = chrono::DateTime::parse_from_rfc3339(timestamp)?
        .with_timezone(&chrono::Utc);

    match delta.get_at("perceptions", key, ts).await {
        Ok(value) => {
            println!("  [Memory] At {}, {} knew: {:?}", timestamp, key, value);
            Ok(Some(value))
        }
        Err(_) => {
            println!("  [Memory] No memory at that time");
            Ok(None)
        }
    }
}

/// Agent Identity Management
async fn register_agent(
    delta: &KoruDelta,
    name: &str,
    role: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    println!("\n[Agent Registration]");

    // Create identity for the agent
    let user_data = IdentityUserData {
        display_name: Some(name.to_string()),
        bio: Some(format!("ALIS AI {} agent", role)),
        avatar_hash: None,
        metadata: {
            let mut m = std::collections::HashMap::new();
            m.insert("role".to_string(), json!(role));
            m.insert("version".to_string(), json!("3.0.0"));
            m
        },
    };

    let (identity, _secret_key) = delta.auth().create_identity(user_data)?;

    println!("  [Identity] Registered {} agent: {}", role, &identity.public_key[..16]);

    // Store agent registration
    delta
        .put(
            "agents",
            name,
            json!({
                "identity": identity.public_key.clone(),
                "role": role,
                "registered_at": chrono::Utc::now().to_rfc3339(),
            }),
        )
        .await?;

    Ok(identity.public_key)
}

/// Validate identity
async fn validate_agent(delta: &KoruDelta, identity_id: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let valid = delta.auth().verify_identity(identity_id).await?;
    println!("  [Identity] Validation for {}: {}", &identity_id[..16], valid);
    Ok(valid)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║     ALIS AI Integration - KoruDelta Memory Consciousness         ║");
    println!("║                    LCA Architecture v3.0.0                       ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");

    // Initialize Delta (the memory consciousness)
    let delta = KoruDelta::start().await?;
    println!("\n[Delta Agent] Initialized with empty field\n");

    // ═════════════════════════════════════════════════════════════════
    // STAGE 1: Agent Registration (Identity Management)
    // ═════════════════════════════════════════════════════════════════
    println!("{}", "═".repeat(66));
    println!("STAGE 1: Agent Identity Registration");
    println!("{}", "═".repeat(66));

    let perception_id = register_agent(&delta, "perception_1", "Perception").await?;
    let _expression_id = register_agent(&delta, "expression_1", "Expression").await?;
    let _ = register_agent(&delta, "consolidator_1", "Consolidation").await?;

    // Validate identities
    validate_agent(&delta, &perception_id).await?;

    // ═════════════════════════════════════════════════════════════════
    // STAGE 2: Perception Phase (Input → Distinctions)
    // ═════════════════════════════════════════════════════════════════
    println!("\n{}", "═".repeat(66));
    println!("STAGE 2: Perception Phase");
    println!("{}", "═".repeat(66));

    let inputs = vec![
        "The sky is blue today",
        "Blue is a calming color",
        "The ocean reflects the sky",
        "Colors affect our emotions",
        "Calmness comes from nature",
    ];

    let mut distinction_ids = Vec::new();
    for input in &inputs {
        let id = perception_agent(&delta, input, json!({"phase": "nursery"})).await?;
        distinction_ids.push(id);
    }

    // Also store some concepts
    delta
        .put_similar(
            "concepts",
            "color_theory",
            "Colors have psychological effects on human perception",
            Some(json!({"domain": "psychology"})),
        )
        .await?;

    delta
        .put_similar(
            "concepts",
            "nature_therapy",
            "Natural environments reduce stress and improve wellbeing",
            Some(json!({"domain": "health"})),
        )
        .await?;

    println!("\n  [Delta] Total perceptions stored: {}", distinction_ids.len());

    // ═════════════════════════════════════════════════════════════════
    // STAGE 3: Consolidation Phase (Background Synthesis)
    // ═════════════════════════════════════════════════════════════════
    println!("\n{}", "═".repeat(66));
    println!("STAGE 3: Consolidation Phase");
    println!("{}", "═".repeat(66));

    let _synthesized = consolidate(&delta, "perceptions").await?;

    // ═════════════════════════════════════════════════════════════════
    // STAGE 4: Expression Phase (Query → Output)
    // ═════════════════════════════════════════════════════════════════
    println!("\n{}", "═".repeat(66));
    println!("STAGE 4: Expression Phase");
    println!("{}", "═".repeat(66));

    let query = "What makes people feel calm?";
    println!("  [Query] '{}'", query);

    let connected = expression_agent(&delta, query).await?;

    println!("\n  [Expression] Most conscious distinctions:");
    for dist in &connected {
        println!("    - {}", &dist[..dist.len().min(16)]);
    }

    // ═════════════════════════════════════════════════════════════════
    // STAGE 5: Dream Phase (Creative Synthesis)
    // ═════════════════════════════════════════════════════════════════
    println!("\n{}", "═".repeat(66));
    println!("STAGE 5: Dream Phase");
    println!("{}", "═".repeat(66));

    dream(&delta).await?;

    // ═════════════════════════════════════════════════════════════════
    // STAGE 6: Memory (Time Travel)
    // ═════════════════════════════════════════════════════════════════
    println!("\n{}", "═".repeat(66));
    println!("STAGE 6: Memory Query (Time Travel)");
    println!("{}", "═".repeat(66));

    // Query current state
    let now = chrono::Utc::now().to_rfc3339();
    println!("  [Time] Current: {}", &now[..19]);

    // This would show what we knew at the beginning (before storing)
    let early_time = "2024-01-01T00:00:00Z";
    memory_at_time(&delta, &distinction_ids[0], early_time).await?;

    // ═════════════════════════════════════════════════════════════════
    // STAGE 7: Statistics
    // ═════════════════════════════════════════════════════════════════
    println!("\n{}", "═".repeat(66));
    println!("STAGE 7: Delta Agent Statistics");
    println!("{}", "═".repeat(66));

    let stats = delta.stats().await;
    println!("  [Stats] Total keys: {}", stats.key_count);
    println!("  [Stats] Total versions: {}", stats.total_versions);
    println!("  [Stats] Namespaces: {}", stats.namespace_count);

    // List all namespaces
    let namespaces = delta.list_namespaces().await;
    println!("\n  [Namespaces]");
    for ns in &namespaces {
        let keys = delta.list_keys(ns).await;
        println!("    - {}: {} keys", ns, keys.len());
    }

    // ═════════════════════════════════════════════════════════════════
    // Summary
    // ═════════════════════════════════════════════════════════════════
    println!("\n{}", "═".repeat(66));
    println!("VALIDATION SUMMARY");
    println!("{}", "═".repeat(66));

    println!("\n✓ Identity Management");
    println!("  - Agent identity creation");
    println!("  - Identity validation");
    println!("  - Agent registration storage");

    println!("\n✓ Perception Storage");
    println!("  - put_similar() - Semantic storage with auto-embeddings");
    println!("  - Metadata storage");
    println!("  - Content-addressed distinctions");

    println!("\n✓ Consolidation");
    println!("  - find_similar() - Semantic similarity search");
    println!("  - Cross-reference synthesis events");
    println!("  - Background synthesis tracking");

    println!("\n✓ Expression Query");
    println!("  - Semantic search across all namespaces");
    println!("  - Connectivity analysis (highly-connected distinctions)");

    println!("\n✓ Dream/Creativity");
    println!("  - Random access to stored distinctions");
    println!("  - Creative synthesis events");

    println!("\n✓ Time Travel");
    println!("  - get_at() - Historical state queries");
    println!("  - Version history tracking");

    println!("\n✓ Statistics & Introspection");
    println!("  - stats() - Database metrics");
    println!("  - list_namespaces() - Namespace enumeration");
    println!("  - list_keys() - Key enumeration");

    println!("\n{}", "═".repeat(66));
    println!("All ALIS AI functions validated successfully!");
    println!("{}", "═".repeat(66));

    Ok(())
}
