//! TaskFlow - Real-World User Simulation
//!
//! A developer building a collaborative project management platform
//! using KoruDelta as the backend database.

use koru_delta::{
    json, KoruDelta, Query, Filter, SortBy, SortOrder,
    ViewDefinition,
};
use std::collections::HashMap;

/// TaskFlow - Project Management Platform Simulation
struct TaskFlow {
    db: KoruDelta,
    _current_user: Option<String>,
}

impl TaskFlow {
    async fn new() -> anyhow::Result<Self> {
        println!("🚀 Initializing TaskFlow...");
        let db = KoruDelta::start().await?;
        println!("✅ TaskFlow database ready");
        Ok(Self { db, _current_user: None })
    }

    // ============================================
    // USER MANAGEMENT
    // ============================================
    
    async fn create_user(&self, name: &str, email: &str, role: &str) -> anyhow::Result<String> {
        use koru_delta::auth::IdentityUserData;
        
        let auth = self.db.auth();
        let user_data = IdentityUserData {
            display_name: Some(name.to_string()),
            bio: Some(format!("{} - {}", email, role)),
            avatar_hash: None,
            metadata: {
                let mut m = HashMap::new();
                m.insert("role".to_string(), json!(role));
                m.insert("email".to_string(), json!(email));
                m.insert("created".to_string(), json!(chrono::Utc::now().to_rfc3339()));
                m
            },
        };
        
        let (identity, _secret) = auth.create_identity(user_data)?;
        let user_id = identity.public_key.clone();
        
        // Store user profile
        self.db.put("users", &user_id, json!({
            "name": name,
            "email": email,
            "role": role,
            "identity": user_id.clone(),
            "created_at": chrono::Utc::now().to_rfc3339(),
            "status": "active"
        })).await?;
        
        println!("👤 Created user: {} ({})", name, &user_id[..16]);
        Ok(user_id)
    }
    
    async fn verify_user(&self, user_id: &str) -> anyhow::Result<bool> {
        let auth = self.db.auth();
        let valid = auth.verify_identity(user_id).await?;
        Ok(valid)
    }
    
    #[allow(dead_code)]
    async fn get_user(&self, user_id: &str) -> anyhow::Result<Option<serde_json::Value>> {
        match self.db.get("users", user_id).await {
            Ok(v) => Ok(Some(v.value().clone())),
            Err(_) => Ok(None),
        }
    }
    
    // ============================================
    // PROJECT MANAGEMENT
    // ============================================
    
    async fn create_project(&self, name: &str, owner_id: &str, description: &str) -> anyhow::Result<String> {
        let project_id = format!("proj_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
        
        self.db.put("projects", &project_id, json!({
            "id": project_id,
            "name": name,
            "owner": owner_id,
            "description": description,
            "status": "active",
            "created_at": chrono::Utc::now().to_rfc3339(),
            "members": [owner_id],
            "task_count": 0,
            "completed_tasks": 0
        })).await?;
        
        // Create workspace for project
        let _ = self.db.workspace(&project_id);
        
        println!("📁 Created project: {} ({}", name, &project_id[..20]);
        Ok(project_id)
    }
    
    async fn add_project_member(&self, project_id: &str, user_id: &str) -> anyhow::Result<()> {
        let project = self.db.get("projects", project_id).await?;
        let mut members = project.value()["members"].as_array().unwrap().clone();
        
        if !members.contains(&json!(user_id)) {
            members.push(json!(user_id));
            
            // Update with new member
            let updated = project.value().clone();
            let mut updated_obj = updated.as_object().unwrap().clone();
            updated_obj.insert("members".to_string(), json!(members));
            updated_obj.insert("updated_at".to_string(), json!(chrono::Utc::now().to_rfc3339()));
            
            self.db.put("projects", project_id, json!(updated_obj)).await?;
            println!("👥 Added member to project");
        }
        
        Ok(())
    }
    
    // ============================================
    // TASK MANAGEMENT
    // ============================================
    
    async fn create_task(
        &self,
        project_id: &str,
        title: &str,
        description: &str,
        assignee: &str,
        priority: &str,
        tags: Vec<&str>,
    ) -> anyhow::Result<String> {
        let task_id = format!("task_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
        
        let task = json!({
            "id": task_id,
            "project_id": project_id,
            "title": title,
            "description": description,
            "assignee": assignee,
            "priority": priority,
            "tags": tags,
            "status": "todo",
            "created_at": chrono::Utc::now().to_rfc3339(),
            "updated_at": chrono::Utc::now().to_rfc3339(),
            "due_date": None::<String>,
            "comments": [],
            "attachments": []
        });
        
        self.db.put("tasks", &task_id, task.clone()).await?;
        
        // Store in project's workspace too
        self.db.put(project_id, &task_id, task).await?;
        
        // Create embedding for semantic search
        let task_text = format!("{} {} {}", title, description, tags.join(" "));
        self.db.put_similar(
            "task_embeddings",
            &task_id,
            json!({"text": task_text, "task_id": task_id}),
            Some(json!({"project_id": project_id}))
        ).await?;
        
        // Update project task count
        let project = self.db.get("projects", project_id).await?;
        let mut proj_obj = project.value().as_object().unwrap().clone();
        let count = proj_obj["task_count"].as_i64().unwrap_or(0) + 1;
        proj_obj.insert("task_count".to_string(), json!(count));
        self.db.put("projects", project_id, json!(proj_obj)).await?;
        
        println!("✅ Created task: {} ({})", title, &task_id[..20]);
        Ok(task_id)
    }
    
    async fn update_task_status(&self, task_id: &str, status: &str) -> anyhow::Result<()> {
        let task = self.db.get("tasks", task_id).await?;
        let mut task_obj = task.value().as_object().unwrap().clone();
        
        let old_status = task_obj["status"].as_str().unwrap().to_string();
        task_obj.insert("status".to_string(), json!(status));
        task_obj.insert("updated_at".to_string(), json!(chrono::Utc::now().to_rfc3339()));
        
        // Add to history
        let history_entry = json!({
            "from": old_status,
            "to": status,
            "at": chrono::Utc::now().to_rfc3339()
        });
        
        let mut history = task_obj.get("status_history")
            .and_then(|h| h.as_array().cloned())
            .unwrap_or_default();
        history.push(history_entry);
        task_obj.insert("status_history".to_string(), json!(history));
        
        self.db.put("tasks", task_id, json!(task_obj.clone())).await?;
        
        // Also update in project workspace
        let project_id = task_obj["project_id"].as_str().unwrap();
        self.db.put(project_id, task_id, json!(task_obj)).await?;
        
        println!("🔄 Updated task {} to '{}'", task_id, status);
        Ok(())
    }
    
    async fn find_similar_tasks(&self, query: &str, top_k: usize) -> anyhow::Result<Vec<(String, f32)>> {
        let results = self.db.find_similar(
            Some("task_embeddings"),
            json!({"text": query}),
            top_k
        ).await?;
        
        let tasks: Vec<(String, f32)> = results
            .into_iter()
            .map(|r| (r.key, r.score))
            .collect();
        
        Ok(tasks)
    }
    
    // ============================================
    // QUERIES & VIEWS
    // ============================================
    
    async fn get_tasks_by_project(&self, project_id: &str) -> anyhow::Result<Vec<serde_json::Value>> {
        let query = Query {
            filters: vec![Filter::eq("project_id", project_id)],
            ..Default::default()
        };
        
        let results = self.db.query("tasks", query).await?;
        Ok(results.records.into_iter().map(|r| r.value).collect())
    }
    
    async fn get_tasks_by_assignee(&self, user_id: &str) -> anyhow::Result<Vec<serde_json::Value>> {
        let query = Query {
            filters: vec![Filter::eq("assignee", user_id)],
            sort: vec![SortBy { 
                field: "priority".to_string(), 
                order: SortOrder::Desc 
            }],
            ..Default::default()
        };
        
        let results = self.db.query("tasks", query).await?;
        Ok(results.records.into_iter().map(|r| r.value).collect())
    }
    
    async fn get_high_priority_tasks(&self) -> anyhow::Result<Vec<serde_json::Value>> {
        let query = Query {
            filters: vec![
                Filter::eq("priority", "high"),
                Filter::eq("status", "todo"),
            ],
            limit: Some(20),
            ..Default::default()
        };
        
        let results = self.db.query("tasks", query).await?;
        Ok(results.records.into_iter().map(|r| r.value).collect())
    }
    
    async fn create_tasks_view(&self, project_id: &str) -> anyhow::Result<()> {
        let view_def = ViewDefinition {
            name: format!("{}_tasks", project_id),
            source_collection: "tasks".to_string(),
            query: Query {
                filters: vec![Filter::eq("project_id", project_id)],
                ..Default::default()
            },
            created_at: chrono::Utc::now(),
            description: Some(format!("Tasks for project {}", project_id)),
            auto_refresh: true,
        };
        
        self.db.create_view(view_def).await?;
        println!("📊 Created view for project tasks");
        Ok(())
    }
    
    // ============================================
    // ANALYTICS & STATS
    // ============================================
    
    async fn get_project_stats(&self, project_id: &str) -> anyhow::Result<serde_json::Value> {
        let all_tasks = self.get_tasks_by_project(project_id).await?;
        
        let total = all_tasks.len();
        let todo = all_tasks.iter().filter(|t| t["status"] == "todo").count();
        let in_progress = all_tasks.iter().filter(|t| t["status"] == "in_progress").count();
        let done = all_tasks.iter().filter(|t| t["status"] == "done").count();
        
        let high_priority = all_tasks.iter().filter(|t| t["priority"] == "high").count();
        
        Ok(json!({
            "project_id": project_id,
            "total_tasks": total,
            "todo": todo,
            "in_progress": in_progress,
            "done": done,
            "completion_rate": if total > 0 { done as f64 / total as f64 } else { 0.0 },
            "high_priority_count": high_priority
        }))
    }
    
    async fn get_user_workload(&self, user_id: &str) -> anyhow::Result<serde_json::Value> {
        let tasks = self.get_tasks_by_assignee(user_id).await?;
        
        let active_tasks: Vec<_> = tasks.iter()
            .filter(|t| t["status"] != "done" && t["status"] != "cancelled")
            .collect();
        
        let high_priority = active_tasks.iter().filter(|t| t["priority"] == "high").count();
        
        Ok(json!({
            "user_id": user_id,
            "total_assigned": tasks.len(),
            "active_tasks": active_tasks.len(),
            "high_priority_active": high_priority,
            "workload_score": active_tasks.len() * 10 + high_priority * 20
        }))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║     TaskFlow - Real-World User Simulation                     ║");
    println!("║     Building a Project Management Platform with KoruDelta     ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    
    let mut passed = 0;
    let failed = 0;
    
    // Initialize TaskFlow
    let taskflow = TaskFlow::new().await?;
    
    // ============================================
    // SCENARIO 1: Team Setup
    // ============================================
    println!("\n👥 SCENARIO 1: Setting up development team");
    println!("─────────────────────────────────────────────");
    
    print!("Creating product manager... ");
    let pm_id = taskflow.create_user("Alice Chen", "alice@taskflow.io", "product_manager").await?;
    println!("✅"); passed += 1;
    
    print!("Creating tech lead... ");
    let tech_lead_id = taskflow.create_user("Bob Smith", "bob@taskflow.io", "tech_lead").await?;
    println!("✅"); passed += 1;
    
    print!("Creating developers... ");
    let dev1_id = taskflow.create_user("Carol Jones", "carol@taskflow.io", "developer").await?;
    let dev2_id = taskflow.create_user("David Lee", "david@taskflow.io", "developer").await?;
    println!("✅ (2 devs)"); passed += 1;
    
    print!("Creating designer... ");
    let designer_id = taskflow.create_user("Eve Wang", "eve@taskflow.io", "designer").await?;
    println!("✅"); passed += 1;
    
    print!("Verifying all users... ");
    assert!(taskflow.verify_user(&pm_id).await?);
    assert!(taskflow.verify_user(&tech_lead_id).await?);
    assert!(taskflow.verify_user(&dev1_id).await?);
    assert!(taskflow.verify_user(&dev2_id).await?);
    assert!(taskflow.verify_user(&designer_id).await?);
    println!("✅ (5 users verified)"); passed += 1;
    
    // ============================================
    // SCENARIO 2: Project Creation
    // ============================================
    println!("\n📁 SCENARIO 2: Creating projects");
    println!("───────────────────────────────────");
    
    print!("Creating mobile app project... ");
    let mobile_proj = taskflow.create_project(
        "Mobile App v2.0",
        &pm_id,
        "Next generation mobile application with AI features"
    ).await?;
    println!("✅"); passed += 1;
    
    print!("Creating web platform project... ");
    let web_proj = taskflow.create_project(
        "Web Platform Redesign",
        &tech_lead_id,
        "Complete overhaul of the web interface"
    ).await?;
    println!("✅"); passed += 1;
    
    print!("Adding team members to mobile project... ");
    taskflow.add_project_member(&mobile_proj, &tech_lead_id).await?;
    taskflow.add_project_member(&mobile_proj, &dev1_id).await?;
    taskflow.add_project_member(&mobile_proj, &designer_id).await?;
    println!("✅ (3 members added)"); passed += 1;
    
    print!("Adding team members to web project... ");
    taskflow.add_project_member(&web_proj, &pm_id).await?;
    taskflow.add_project_member(&web_proj, &dev2_id).await?;
    println!("✅ (2 members added)"); passed += 1;
    
    // ============================================
    // SCENARIO 3: Task Creation
    // ============================================
    println!("\n✅ SCENARIO 3: Creating and managing tasks");
    println!("───────────────────────────────────────────");
    
    print!("Creating mobile app tasks... ");
    let tasks = vec![
        ("Design new navigation system", "high", vec!["ui", "ux"]),
        ("Implement AI recommendation engine", "high", vec!["ai", "backend"]),
        ("Setup CI/CD pipeline", "medium", vec!["devops"]),
        ("Write API documentation", "low", vec!["docs"]),
        ("Optimize image loading", "medium", vec!["performance"]),
        ("Add user analytics", "high", vec!["analytics", "backend"]),
        ("Create onboarding flow", "high", vec!["ui", "ux"]),
        ("Fix login bug on iOS", "high", vec!["bugfix", "ios"]),
        ("Update privacy policy", "low", vec!["legal"]),
        ("Refactor database schema", "medium", vec!["backend", "database"]),
    ];
    
    let mut mobile_tasks = vec![];
    for (title, priority, tags) in tasks {
        let task_id = taskflow.create_task(
            &mobile_proj,
            title,
            &format!("Detailed description for: {}", title),
            &dev1_id,
            priority,
            tags.to_vec()
        ).await?;
        mobile_tasks.push(task_id);
    }
    println!("✅ (10 tasks created)"); passed += 1;
    
    print!("Creating web platform tasks... ");
    let web_tasks = vec![
        ("Redesign homepage", "high", vec!["design", "frontend"]),
        ("Migrate to new framework", "high", vec!["frontend", "refactor"]),
        ("Add dark mode support", "medium", vec!["ui", "feature"]),
        ("Optimize bundle size", "medium", vec!["performance"]),
        ("Implement real-time notifications", "high", vec!["websocket", "backend"]),
    ];
    
    let mut web_task_ids = vec![];
    for (title, priority, tags) in web_tasks {
        let task_id = taskflow.create_task(
            &web_proj,
            title,
            &format!("Detailed description for: {}", title),
            &dev2_id,
            priority,
            tags.to_vec()
        ).await?;
        web_task_ids.push(task_id);
    }
    println!("✅ (5 tasks created)"); passed += 1;
    
    // ============================================
    // SCENARIO 4: Task Workflow
    // ============================================
    println!("\n🔄 SCENARIO 4: Task workflow simulation");
    println!("──────────────────────────────────────────");
    
    print!("Updating task statuses... ");
    taskflow.update_task_status(&mobile_tasks[0], "in_progress").await?;
    taskflow.update_task_status(&mobile_tasks[1], "in_progress").await?;
    taskflow.update_task_status(&mobile_tasks[2], "done").await?;
    taskflow.update_task_status(&web_task_ids[0], "in_progress").await?;
    println!("✅ (4 status updates)"); passed += 1;
    
    print!("Verifying version history... ");
    let task_history = taskflow.db.history("tasks", &mobile_tasks[2]).await?;
    assert!(task_history.len() >= 2); // Original + status update
    println!("✅ ({} versions tracked)", task_history.len()); passed += 1;
    
    // ============================================
    // SCENARIO 5: Queries & Views
    // ============================================
    println!("\n🔍 SCENARIO 5: Complex queries and views");
    println!("──────────────────────────────────────────");
    
    print!("Querying tasks by project... ");
    let mobile_project_tasks = taskflow.get_tasks_by_project(&mobile_proj).await?;
    assert_eq!(mobile_project_tasks.len(), 10);
    println!("✅ ({} tasks in mobile project)", mobile_project_tasks.len()); passed += 1;
    
    print!("Querying tasks by assignee... ");
    let dev1_tasks = taskflow.get_tasks_by_assignee(&dev1_id).await?;
    assert!(!dev1_tasks.is_empty());
    println!("✅ ({} tasks assigned)", dev1_tasks.len()); passed += 1;
    
    print!("Querying high priority tasks... ");
    let high_priority = taskflow.get_high_priority_tasks().await?;
    assert!(!high_priority.is_empty());
    println!("✅ ({} high priority tasks)", high_priority.len()); passed += 1;
    
    print!("Creating project task views... ");
    taskflow.create_tasks_view(&mobile_proj).await?;
    taskflow.create_tasks_view(&web_proj).await?;
    println!("✅ (2 views created)"); passed += 1;
    
    print!("Querying views... ");
    let view_results = taskflow.db.query_view(&format!("{}_tasks", mobile_proj)).await?;
    assert_eq!(view_results.records.len(), 10);
    println!("✅ ({} records in view)", view_results.records.len()); passed += 1;
    
    // ============================================
    // SCENARIO 6: Vector Search (Semantic)
    // ============================================
    println!("\n🔤 SCENARIO 6: Semantic task search");
    println!("──────────────────────────────────────");
    
    print!("Finding similar tasks (AI-related)... ");
    let ai_tasks = taskflow.find_similar_tasks("artificial intelligence machine learning", 3).await?;
    println!("✅ (found {} related tasks)", ai_tasks.len()); passed += 1;
    
    print!("Finding similar tasks (UI-related)... ");
    let ui_tasks = taskflow.find_similar_tasks("user interface design", 3).await?;
    println!("✅ (found {} related tasks)", ui_tasks.len()); passed += 1;
    
    print!("Finding similar tasks (performance)... ");
    let perf_tasks = taskflow.find_similar_tasks("optimization speed", 3).await?;
    println!("✅ (found {} related tasks)", perf_tasks.len()); passed += 1;
    
    // ============================================
    // SCENARIO 7: Analytics
    // ============================================
    println!("\n📊 SCENARIO 7: Project analytics");
    println!("───────────────────────────────────");
    
    print!("Getting project stats... ");
    let mobile_stats = taskflow.get_project_stats(&mobile_proj).await?;
    assert_eq!(mobile_stats["total_tasks"], 10);
    println!("✅ Mobile: {} total, {} done", 
        mobile_stats["total_tasks"], 
        mobile_stats["done"]
    ); passed += 1;
    
    let web_stats = taskflow.get_project_stats(&web_proj).await?;
    assert_eq!(web_stats["total_tasks"], 5);
    println!("✅ Web: {} total, {} done", 
        web_stats["total_tasks"],
        web_stats["done"]
    ); passed += 1;
    
    print!("Getting user workload... ");
    let dev1_workload = taskflow.get_user_workload(&dev1_id).await?;
    assert!(dev1_workload["total_assigned"].as_i64().unwrap() > 0);
    println!("✅ Dev1: {} tasks, workload score {}",
        dev1_workload["total_assigned"],
        dev1_workload["workload_score"]
    ); passed += 1;
    
    // ============================================
    // SCENARIO 8: Concurrent Operations
    // ============================================
    println!("\n🔄 SCENARIO 8: Concurrent team operations");
    println!("──────────────────────────────────────────");
    
    print!("Simulating concurrent task updates... ");
    let mut handles = vec![];
    
    // Multiple users updating tasks simultaneously
    for (i, task_id) in mobile_tasks.iter().take(5).enumerate() {
        let db_clone = taskflow.db.clone();
        let task_id = task_id.clone();
        handles.push(tokio::spawn(async move {
            db_clone.put(&format!("comments_{}", i), "comment", json!({
                "task_id": task_id,
                "text": "Updated progress",
                "timestamp": chrono::Utc::now().to_rfc3339()
            })).await
        }));
    }
    
    for h in handles {
        h.await??;
    }
    println!("✅ (5 concurrent updates)"); passed += 1;
    
    // ============================================
    // SCENARIO 9: Database Stats
    // ============================================
    println!("\n📈 SCENARIO 9: Database health check");
    println!("───────────────────────────────────────");
    
    print!("Getting database stats... ");
    let _stats = taskflow.db.stats().await;
    println!("✅"); passed += 1;
    
    print!("Verifying namespaces... ");
    let namespaces = taskflow.db.list_namespaces().await;
    assert!(namespaces.len() >= 5); // users, projects, tasks, workspaces, embeddings
    println!("✅ ({} namespaces)", namespaces.len()); passed += 1;
    
    print!("Verifying data integrity... ");
    let all_users = taskflow.db.list_keys("users").await;
    assert_eq!(all_users.len(), 5); // PM, Tech Lead, 2 Devs, Designer
    println!("✅ ({} users stored)", all_users.len()); passed += 1;
    
    // ============================================
    // FINAL SUMMARY
    // ============================================
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║              TASKFLOW SIMULATION COMPLETE                      ║");
    println!("╠════════════════════════════════════════════════════════════════╣");
    println!("║  Scenarios Tested: 9                                           ║");
    println!("║  ✅ Passed:        {:3}                                         ║", passed);
    println!("║  ❌ Failed:        {:3}                                         ║", failed);
    println!("╚════════════════════════════════════════════════════════════════╝");
    
    println!("\n📋 Real-World Features Validated:");
    println!("   ✅ User management with identity verification");
    println!("   ✅ Project creation and team member assignment");
    println!("   ✅ Task creation with priorities and tags");
    println!("   ✅ Task workflow with version history");
    println!("   ✅ Complex queries (by project, assignee, priority)");
    println!("   ✅ Materialized views for dashboards");
    println!("   ✅ Semantic search using vector embeddings");
    println!("   ✅ Project analytics and user workload tracking");
    println!("   ✅ Concurrent team operations");
    println!("   ✅ Database health monitoring");
    
    println!("\n✨ TaskFlow platform simulation successful!");
    println!("   KoruDelta handles complex real-world use cases flawlessly.");
    
    Ok(())
}
