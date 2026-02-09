/// Crisis Coordination Demo - KoruDelta Full Feature Showcase
///
/// Run with: cargo run --example crisis_coordination_demo

use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use koru_delta::prelude::*;
    use koru_delta::vector::VectorSearchOptions;
    use koru_delta::auth::{mine_identity, IdentityUserData};
    use koru_delta::query::{Query, Filter, SortBy, SortOrder};
    use koru_delta::subscriptions::{ChangeType, Subscription};
    use chrono::Utc;
    use colored::*;
    use serde_json::json;
    use std::collections::HashMap;

    println!("{}", "╔═══════════════════════════════════════════════════════════════╗".bold().cyan());
    println!("{}", "║     CRISIS COORDINATION SYSTEM - KoruDelta Demo               ║".bold().cyan());
    println!("{}", "║     Full Feature Showcase with Real-World Scenario            ║".bold().cyan());
    println!("{}", "╚═══════════════════════════════════════════════════════════════╝".bold().cyan());

    // PHASE 1: Initialize Database
    println!("\n{}", "📦 PHASE 1: Initializing KoruDelta Database".bold().yellow());
    let db_path = std::env::temp_dir().join("crisis_coordination_db");
    let _ = tokio::fs::remove_dir_all(&db_path).await;
    let db = KoruDelta::start_with_path(&db_path).await?;
    println!("   ✓ Database initialized at: {}", db_path.display());

    // PHASE 2: Self-Sovereign Identity
    println!("\n{}", "🔐 PHASE 2: Self-Sovereign Identity Setup".bold().yellow());
    println!("   Mining cryptographically-verified identities...");
    
    let fire_user = IdentityUserData {
        display_name: Some("Fire Department".to_string()),
        bio: Some("Emergency fire response".to_string()),
        avatar_hash: None,
        metadata: HashMap::new(),
    };
    let police_user = IdentityUserData {
        display_name: Some("Police Department".to_string()),
        bio: Some("Law enforcement".to_string()),
        avatar_hash: None,
        metadata: HashMap::new(),
    };
    let medical_user = IdentityUserData {
        display_name: Some("Medical Department".to_string()),
        bio: Some("Medical emergency response".to_string()),
        avatar_hash: None,
        metadata: HashMap::new(),
    };
    
    let fire_id = mine_identity(fire_user, 2).await;
    let police_id = mine_identity(police_user, 2).await;
    let medical_id = mine_identity(medical_user, 2).await;
    
    println!("   ✓ Fire: {}...", &fire_id.identity.public_key[..16]);
    println!("   ✓ Police: {}...", &police_id.identity.public_key[..16]);
    println!("   ✓ Medical: {}...", &medical_id.identity.public_key[..16]);
    
    db.put("identities", "fire-dept", json!(fire_id.identity)).await?;
    db.put("identities", "police-dept", json!(police_id.identity)).await?;
    db.put("identities", "medical-dept", json!(medical_id.identity)).await?;

    // PHASE 3: Real-Time Subscriptions
    println!("\n{}", "🚨 PHASE 3: Real-Time Incident Coordination".bold().yellow());
    println!("   Setting up real-time subscriptions...");
    
    let (sub_id, mut rx) = db.subscribe(Subscription {
        collection: Some("incidents".to_string()),
        key: None,
        filter: None,
        change_types: vec![ChangeType::Insert, ChangeType::Update, ChangeType::Delete],
        name: Some("incident-monitor".to_string()),
    }).await;
    println!("   ✓ Subscription active (ID: {})", sub_id);
    
    let subscription_handle = tokio::spawn(async move {
        let mut count = 0;
        while let Ok(event) = rx.recv().await {
            count += 1;
            match event.change_type {
                ChangeType::Insert => {
                    println!("     📡 LIVE #{}: '{}' reported at {}", 
                        count, event.key.cyan(), event.timestamp.format("%H:%M:%S"));
                }
                ChangeType::Update => {
                    println!("     📡 LIVE #{}: '{}' updated", count, event.key.cyan());
                }
                ChangeType::Delete => {
                    println!("     📡 LIVE #{}: '{}' resolved", count, event.key);
                }
            }
        }
    });

    // Create incidents
    let incidents = vec![
        ("inc-001", "Building Fire - Downtown", "fire-dept", "critical", 
         json!({"type": "fire", "casualties": 0, "units": 5})),
        ("inc-002", "Traffic Accident - Highway", "police-dept", "high",
         json!({"type": "accident", "vehicles": 3, "injuries": 2})),
        ("inc-003", "Medical Emergency", "medical-dept", "high",
         json!({"type": "medical", "patients": 1})),
        ("inc-004", "Gas Leak - Residential", "fire-dept", "critical",
         json!({"type": "hazmat", "evacuation": "200m", "homes": 45})),
        ("inc-005", "Protest - City Hall", "police-dept", "medium",
         json!({"type": "crowd_control", "crowd": 200})),
    ];

    println!("   Creating {} incidents with causal timestamps...", incidents.len());
    for (id, desc, agency, priority, details) in incidents {
        let incident = json!({
            "id": id, "description": desc, "agency": agency,
            "priority": priority, "status": "active",
            "details": details, "created_at": Utc::now().to_rfc3339(),
        });
        db.put("incidents", id, incident).await?;
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
    tokio::time::sleep(Duration::from_millis(500)).await;
    println!("   ✓ All incidents recorded");

    // PHASE 4: Materialized Views
    println!("\n{}", "📊 PHASE 4: Materialized Dashboard Views".bold().yellow());
    
    let critical_view = ViewDefinition {
        name: "critical_incidents".to_string(),
        source_collection: "incidents".to_string(),
        query: Query {
            filters: vec![Filter::eq("priority", "critical")],
            sort: vec![SortBy { field: "created_at".to_string(), order: SortOrder::Desc }],
            ..Default::default()
        },
        created_at: Utc::now(),
        description: Some("Critical incidents".to_string()),
        auto_refresh: true,
    };
    db.create_view(critical_view).await?;
    println!("   ✓ Created 'critical_incidents' view");
    
    let fire_view = ViewDefinition {
        name: "fire_dashboard".to_string(),
        source_collection: "incidents".to_string(),
        query: Query {
            filters: vec![Filter::eq("agency", "fire-dept")],
            sort: vec![SortBy { field: "priority".to_string(), order: SortOrder::Asc }],
            ..Default::default()
        },
        created_at: Utc::now(),
        description: Some("Fire dept incidents".to_string()),
        auto_refresh: true,
    };
    db.create_view(fire_view).await?;
    println!("   ✓ Created 'fire_dashboard' view");
    
    println!("\n   📋 Critical Incidents:");
    let critical = db.query_view("critical_incidents").await?;
    for r in &critical.records {
        println!("      🔴 {}: {}", r.key.cyan(),
            r.value.get("description").and_then(|v| v.as_str()).unwrap_or(""));
    }
    
    println!("\n   📋 Fire Dashboard:");
    let fire = db.query_view("fire_dashboard").await?;
    for r in &fire.records {
        println!("      🚒 {}: {}", r.key.cyan(),
            r.value.get("priority").and_then(|v| v.as_str()).unwrap_or(""));
    }

    // PHASE 5: Time Travel
    println!("\n{}", "⏰ PHASE 5: Time Travel Investigation".bold().yellow());
    
    let current = db.get("incidents", "inc-001").await?;
    println!("   Current status: {}", 
        current.value.get("status").and_then(|v| v.as_str()).unwrap_or("unknown"));
    
    let mut updated = (*current.value).clone();
    updated["status"] = json!("contained");
    updated["contained_at"] = json!(Utc::now().to_rfc3339());
    db.put("incidents", "inc-001", updated).await?;
    
    println!("\n   📜 Version history:");
    let history = db.history("incidents", "inc-001").await?;
    for (i, entry) in history.iter().enumerate() {
        let status = entry.value.get("status").and_then(|v| v.as_str()).unwrap_or("?");
        let colored = if status == "active" { status.red() } else { status.green() };
        println!("      v{}: {} at {}", i + 1, colored, entry.timestamp.format("%H:%M:%S"));
    }
    
    let past_time = history.first().map(|h| h.timestamp).unwrap_or_else(Utc::now);
    let past = db.get_at("incidents", "inc-001", past_time).await?;
    println!("\n   ⏮️  Time travel to initial report:");
    println!("      Status was: {}", 
        past.value.get("status").and_then(|v| v.as_str()).unwrap_or("unknown"));

    // PHASE 6: Vector Search
    println!("\n{}", "🔍 PHASE 6: Semantic Search".bold().yellow());
    
    let descriptions = vec![
        ("inc-001", "Large building fire downtown with smoke hazards"),
        ("inc-002", "Multi-vehicle collision blocking highway"),
        ("inc-003", "Elderly patient cardiac distress"),
        ("inc-004", "Natural gas leak residential evacuation"),
        ("inc-005", "Peaceful protest at city government"),
    ];
    
    println!("   Storing vector embeddings...");
    for (id, desc) in &descriptions {
        let embedding = create_simple_embedding(*desc);
        db.embed("vectors", *id, embedding, Some(json!({"desc": *desc}))).await?;
    }
    println!("   ✓ {} vectors stored", descriptions.len());
    
    let query = "fire and smoke emergency";
    println!("\n   Searching: \"{}\"", query.cyan());
    let query_vec = create_simple_embedding(query);
    let opts = VectorSearchOptions { top_k: 3, threshold: 0.0, model_filter: None };
    let results = db.embed_search(Some("vectors"), &query_vec, opts).await?;
    
    for (i, r) in results.iter().enumerate() {
        println!("      #{}: {} (score: {:.2})", i + 1, r.key.yellow(), r.score);
    }

    // PHASE 7: Persistence & Recovery
    println!("\n{}", "💾 PHASE 7: Persistence Test".bold().yellow());
    println!("   Simulating restart...");
    db.shutdown().await?;
    println!("   ✓ Shutdown complete");
    
    let db2 = KoruDelta::start_with_path(&db_path).await?;
    println!("   ✓ Restarted from WAL");
    println!("   ✓ Views restored: {}", db2.list_views().await.len());
    println!("   ✓ Incidents restored: {}", db2.list_keys("incidents").await.len());

    // PHASE 8: Query Engine
    println!("\n{}", "🔎 PHASE 8: Advanced Query".bold().yellow());
    println!("   Query: critical + fire-dept");
    
    let query = Query {
        filters: vec![Filter::And(vec![
            Filter::eq("priority", "critical"),
            Filter::eq("agency", "fire-dept"),
        ])],
        ..Default::default()
    };
    
    let results = db2.query("incidents", query).await?;
    println!("   Found {} incidents:", results.total_count);
    for r in &results.records {
        println!("      • {}: {}", r.key.cyan(),
            r.value.get("description").and_then(|v| v.as_str()).unwrap_or(""));
    }

    // Cleanup
    println!("\n{}", "✅ Demo Complete!".bold().green());
    println!("{}", "   Features demonstrated:".green());
    println!("   • Persistence with WAL recovery");
    println!("   • Self-sovereign identity (proof-of-work)");
    println!("   • Real-time subscriptions");
    println!("   • Materialized views");
    println!("   • Time travel / audit trails");
    println!("   • Vector semantic search");
    println!("   • Query engine with filters");
    
    drop(subscription_handle);
    let _ = tokio::fs::remove_dir_all(&db_path).await;
    Ok(())
}

fn create_simple_embedding(text: &str) -> koru_delta::vector::Vector {
    let text = text.to_lowercase();
    let words: Vec<&str> = text.split_whitespace().collect();
    let keywords = vec!["fire", "smoke", "building", "emergency", "traffic", "accident", 
        "vehicle", "medical", "patient", "gas", "leak", "evacuation", "protest", "crowd"];
    
    let mut vec = vec![0.0f32; 16];
    for (i, kw) in keywords.iter().enumerate().take(16) {
        vec[i] = words.iter().filter(|&&w| w.contains(kw)).count() as f32;
    }
    
    let mag: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    if mag > 0.0 { for v in &mut vec { *v /= mag; } }
    
    koru_delta::vector::Vector::new(vec, "demo-model")
}

#[cfg(target_arch = "wasm32")]
fn main() {
    println!("This example requires native features.");
}
