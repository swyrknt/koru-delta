/// KoruDelta CLI - The Invisible Database Command Line Tool
///
/// This is the main CLI interface for KoruDelta, providing simple commands
/// for interacting with the causal database.
///
/// Usage:
///   kdelta set <namespace>/<key> <value>  - Store a value
///   kdelta get <namespace>/<key>          - Retrieve a value
///   kdelta log <namespace>/<key>          - Show history
///   kdelta status                         - Show database stats
///   kdelta list [namespace]               - List namespaces or keys
///   kdelta start [--join <addr>]          - Start a cluster node
///   kdelta peers                          - Show peer nodes
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use colored::*;
use koru_delta::cluster::{ClusterConfig, ClusterNode};
use koru_delta::network::{PeerStatus, DEFAULT_PORT};
use koru_delta::query::{Aggregation, Filter, Query};
use koru_delta::subscriptions::{ChangeType, Subscription};
use koru_delta::views::ViewDefinition;
use koru_delta::{DeltaError, KoruDelta};
use serde_json::Value as JsonValue;
use similar::{ChangeTag, TextDiff};
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::signal;

// ============================================================================
// HTTP Client for Remote Operations
// ============================================================================

/// HTTP client for remote KoruDelta operations.
struct HttpClient {
    base_url: String,
    client: reqwest::Client,
}

impl HttpClient {
    /// Create a new HTTP client.
    fn new(base_url: String) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Get a value from the remote server.
    async fn get(&self, namespace: &str, key: &str) -> Result<serde_json::Value> {
        let url = format!("{}/api/v1/{}/{}", self.base_url, namespace, key);
        let response = self.client.get(&url).send().await?;
        
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            anyhow::bail!("Key not found: {}/{}", namespace, key);
        }
        
        let data: serde_json::Value = response.error_for_status()?.json().await?;
        Ok(data.get("value").cloned().unwrap_or(serde_json::Value::Null))
    }

    /// Store a value on the remote server.
    async fn put(&self, namespace: &str, key: &str, value: serde_json::Value) -> Result<serde_json::Value> {
        let url = format!("{}/api/v1/{}/{}", self.base_url, namespace, key);
        let body = serde_json::json!({ "value": value });
        let response = self.client.put(&url).json(&body).send().await?;
        let data: serde_json::Value = response.error_for_status()?.json().await?;
        Ok(data)
    }

    /// Get history from the remote server.
    async fn history(&self, namespace: &str, key: &str) -> Result<Vec<serde_json::Value>> {
        let url = format!("{}/api/v1/{}/{}/history", self.base_url, namespace, key);
        let response = self.client.get(&url).send().await?;
        
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            anyhow::bail!("Key not found: {}/{}", namespace, key);
        }
        
        let data: serde_json::Value = response.error_for_status()?.json().await?;
        Ok(data.get("versions")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default())
    }

    /// Get value at a specific timestamp.
    async fn get_at(&self, namespace: &str, key: &str, timestamp: &str) -> Result<serde_json::Value> {
        let url = format!("{}/api/v1/{}/{}/at/{}", self.base_url, namespace, key, timestamp);
        let response = self.client.get(&url).send().await?;
        
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            anyhow::bail!("Key not found at timestamp: {}/{}", namespace, key);
        }
        
        let data: serde_json::Value = response.error_for_status()?.json().await?;
        Ok(data)
    }

    /// Query the remote server.
    async fn query(&self, namespace: &str, query: serde_json::Value) -> Result<serde_json::Value> {
        let url = format!("{}/api/v1/{}/query", self.base_url, namespace);
        let response = self.client.post(&url).json(&query).send().await?;
        let data: serde_json::Value = response.error_for_status()?.json().await?;
        Ok(data)
    }

    /// List namespaces from the remote server.
    async fn list_namespaces(&self) -> Result<Vec<String>> {
        let url = format!("{}/api/v1/namespaces", self.base_url);
        let response = self.client.get(&url).send().await?;
        let data: serde_json::Value = response.error_for_status()?.json().await?;
        Ok(data.get("namespaces")
            .and_then(|n| n.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default())
    }

    /// List keys from the remote server.
    async fn list_keys(&self, namespace: &str) -> Result<Vec<String>> {
        let url = format!("{}/api/v1/{}/keys", self.base_url, namespace);
        let response = self.client.get(&url).send().await?;
        let data: serde_json::Value = response.error_for_status()?.json().await?;
        Ok(data.get("keys")
            .and_then(|k| k.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default())
    }

    /// Get status from the remote server.
    async fn status(&self) -> Result<serde_json::Value> {
        let url = format!("{}/api/v1/status", self.base_url);
        let response = self.client.get(&url).send().await?;
        let data: serde_json::Value = response.error_for_status()?.json().await?;
        Ok(data)
    }

    /// List views from the remote server.
    async fn list_views(&self) -> Result<Vec<serde_json::Value>> {
        let url = format!("{}/api/v1/views", self.base_url);
        let response = self.client.get(&url).send().await?;
        let data: serde_json::Value = response.error_for_status()?.json().await?;
        Ok(data.get("views")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default())
    }

    /// Create a view on the remote server.
    async fn create_view(&self, view_def: serde_json::Value) -> Result<()> {
        let url = format!("{}/api/v1/views", self.base_url);
        let response = self.client.post(&url).json(&view_def).send().await?;
        response.error_for_status()?;
        Ok(())
    }

    /// Query a view on the remote server.
    async fn query_view(&self, name: &str) -> Result<serde_json::Value> {
        let url = format!("{}/api/v1/views/{}", self.base_url, name);
        let response = self.client.get(&url).send().await?;
        
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            anyhow::bail!("View not found: {}", name);
        }
        
        let data: serde_json::Value = response.error_for_status()?.json().await?;
        Ok(data)
    }

    /// Refresh a view on the remote server.
    async fn refresh_view(&self, name: &str) -> Result<()> {
        let url = format!("{}/api/v1/views/{}/refresh", self.base_url, name);
        let response = self.client.post(&url).send().await?;
        
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            anyhow::bail!("View not found: {}", name);
        }
        
        response.error_for_status()?;
        Ok(())
    }

    /// Delete a view on the remote server.
    async fn delete_view(&self, name: &str) -> Result<()> {
        let url = format!("{}/api/v1/views/{}", self.base_url, name);
        let response = self.client.delete(&url).send().await?;
        
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            anyhow::bail!("View not found: {}", name);
        }
        
        response.error_for_status()?;
        Ok(())
    }
}

/// KoruDelta - The Invisible Database
///
/// A causal, consistent database with Git-like history and Redis-like simplicity.
#[derive(Parser)]
#[command(name = "kdelta")]
#[command(version, about, long_about = None)]
struct Cli {
    /// Database file path (default: ~/.korudelta/db)
    #[arg(short, long, global = true)]
    db_path: Option<PathBuf>,

    /// Server URL for remote operations (e.g., http://localhost:8080)
    #[arg(short, long, global = true)]
    url: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Store a value in the database
    ///
    /// Format: `kdelta set <namespace>/<key> <value>`
    ///
    /// Examples:
    ///   kdelta set users/alice '{"name": "Alice", "age": 30}'
    ///   kdelta set counters/visits 42
    ///   kdelta set config/theme '"dark"'
    Set {
        /// Key in format: namespace/key (e.g., users/alice)
        key: String,

        /// Value to store (JSON format)
        value: String,
    },

    /// Retrieve a value from the database
    ///
    /// Format: `kdelta get <namespace>/<key>`
    ///
    /// Examples:
    ///   kdelta get users/alice
    ///   kdelta get counters/visits
    ///   kdelta get users/alice --at "2026-02-04T12:00:00Z"
    Get {
        /// Key in format: namespace/key
        key: String,

        /// Show metadata (timestamp, version ID)
        #[arg(short, long)]
        verbose: bool,

        /// Get value at specific timestamp (ISO 8601 format)
        #[arg(short, long)]
        at: Option<String>,
    },

    /// Show the history of changes for a key
    ///
    /// Format: `kdelta log <namespace>/<key>`
    ///
    /// Example:
    ///   kdelta log users/alice
    Log {
        /// Key in format: namespace/key
        key: String,

        /// Limit number of entries shown
        #[arg(short, long)]
        limit: Option<usize>,
    },

    /// Show database and cluster status
    ///
    /// Displays information about keys, versions, namespaces, and cluster state.
    Status,

    /// List namespaces or keys
    ///
    /// Examples:
    ///   kdelta list              # List all namespaces
    ///   kdelta list users        # List all keys in 'users' namespace
    List {
        /// Optional namespace to list keys from
        namespace: Option<String>,
    },

    /// Compare two versions of a key
    ///
    /// Format: `kdelta diff <namespace>/<key> [--at <timestamp>]`
    ///
    /// Examples:
    ///   kdelta diff users/alice                              # Compare latest with previous
    ///   kdelta diff users/alice --at "2025-11-24T17:00:00Z"  # Compare at timestamp with previous
    ///   kdelta diff users/alice --from 0 --to 1              # Compare version indices
    Diff {
        /// Key in format: namespace/key
        key: String,

        /// Compare value at this timestamp with its previous version
        #[arg(long)]
        at: Option<String>,

        /// Compare from this version index (0 = oldest)
        #[arg(long)]
        from: Option<usize>,

        /// Compare to this version index
        #[arg(long)]
        to: Option<usize>,
    },

    /// Start a KoruDelta node (server mode)
    ///
    /// This starts the node as a long-running server that can accept cluster
    /// connections and sync with other nodes.
    ///
    /// Examples:
    ///   kdelta start                           # Start a standalone node
    ///   kdelta start --join 192.168.1.100:7878 # Join an existing cluster
    ///   kdelta start --port 8080               # Use a custom port
    Start {
        /// Port to listen on (default: 7878)
        #[arg(short, long, default_value_t = DEFAULT_PORT)]
        port: u16,

        /// Address of an existing node to join
        #[arg(short, long)]
        join: Option<String>,

        /// Bind to a specific address (default: 0.0.0.0)
        #[arg(short, long, default_value = "0.0.0.0")]
        bind: String,
    },

    /// Start an HTTP API server
    ///
    /// This starts an HTTP server that exposes the KoruDelta API over REST.
    /// Unlike 'start' which starts a cluster node, this provides an HTTP interface
    /// for remote clients to interact with the database.
    ///
    /// Examples:
    ///   kdelta serve                        # Start HTTP server on default port 8080
    ///   kdelta serve --port 3000            # Start on port 3000
    ///   kdelta serve --bind 127.0.0.1       # Bind to localhost only
    Serve {
        /// Port to listen on (default: 8080)
        #[arg(short, long, default_value_t = 8080)]
        port: u16,

        /// Bind to a specific address (default: 0.0.0.0)
        #[arg(short, long, default_value = "0.0.0.0")]
        bind: String,
    },

    /// Show cluster peers
    ///
    /// Lists all known peer nodes in the cluster.
    Peers,

    /// Query data in a namespace
    ///
    /// Filter, sort, and aggregate data with powerful queries.
    ///
    /// Examples:
    ///   kdelta query users                              # Get all users
    ///   kdelta query users --filter 'age > 30'          # Filter by age
    ///   kdelta query users --sort age                   # Sort by age
    ///   kdelta query users --limit 10                   # Limit results
    ///   kdelta query users --count                      # Count records
    #[command(name = "query")]
    Query {
        /// Namespace to query
        namespace: String,

        /// Filter expression (e.g., 'age > 30', 'status = "active"')
        #[arg(short, long)]
        filter: Option<String>,

        /// Field to sort by
        #[arg(short, long)]
        sort: Option<String>,

        /// Sort descending (default: ascending)
        #[arg(long)]
        desc: bool,

        /// Limit number of results
        #[arg(short, long)]
        limit: Option<usize>,

        /// Count records instead of showing them
        #[arg(long)]
        count: bool,

        /// Sum a numeric field
        #[arg(long)]
        sum: Option<String>,

        /// Average a numeric field
        #[arg(long)]
        avg: Option<String>,
    },

    /// Manage materialized views
    ///
    /// Create, list, refresh, and query cached views.
    #[command(subcommand)]
    View(ViewCommands),

    /// Watch for changes in real-time
    ///
    /// Subscribe to changes and receive notifications.
    ///
    /// Examples:
    ///   kdelta watch users                    # Watch all changes in users
    ///   kdelta watch users/alice              # Watch specific key
    ///   kdelta watch --all                    # Watch all changes
    Watch {
        /// Namespace or key to watch (format: namespace or namespace/key)
        target: Option<String>,

        /// Watch all changes across all namespaces
        #[arg(long)]
        all: bool,

        /// Only show inserts
        #[arg(long)]
        inserts_only: bool,

        /// Only show updates
        #[arg(long)]
        updates_only: bool,
    },
}

/// View management subcommands
#[derive(Subcommand)]
enum ViewCommands {
    /// Create a new materialized view
    ///
    /// Example:
    ///   kdelta view create active_users users --filter 'status = "active"'
    Create {
        /// View name
        name: String,

        /// Source namespace
        source: String,

        /// Filter expression
        #[arg(short, long)]
        filter: Option<String>,

        /// Description
        #[arg(short, long)]
        description: Option<String>,

        /// Auto-refresh on writes
        #[arg(long)]
        auto_refresh: bool,
    },

    /// List all views
    List,

    /// Refresh a view
    Refresh {
        /// View name
        name: String,
    },

    /// Query a view
    #[command(name = "query")]
    Query {
        /// View name
        name: String,

        /// Limit results
        #[arg(short, long)]
        limit: Option<usize>,
    },

    /// Delete a view
    Delete {
        /// View name
        name: String,
    },
}

/// Parse a key in format "namespace/key" into (namespace, key)
fn parse_key(key: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = key.splitn(2, '/').collect();
    if parts.len() != 2 {
        anyhow::bail!(
            "Invalid key format. Expected 'namespace/key', got '{}'.\n\
             Example: users/alice",
            key
        );
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

/// Parse a simple filter expression into a Filter
///
/// Supports: field = value, field > value, field < value, field >= value, field <= value
fn parse_filter(expr: &str) -> Result<Filter> {
    let expr = expr.trim();

    // Try different operators
    let operators = [">=", "<=", "!=", "=", ">", "<"];

    for op in operators {
        if let Some(idx) = expr.find(op) {
            let field = expr[..idx].trim().to_string();
            let value_str = expr[idx + op.len()..].trim();

            // Parse the value as JSON
            let value: JsonValue = if value_str.starts_with('"') && value_str.ends_with('"') {
                // It's a string
                serde_json::from_str(value_str)
                    .with_context(|| format!("Invalid value: {}", value_str))?
            } else if value_str == "true" || value_str == "false" {
                // It's a boolean
                serde_json::from_str(value_str)?
            } else if let Ok(num) = value_str.parse::<i64>() {
                // It's an integer
                serde_json::json!(num)
            } else if let Ok(num) = value_str.parse::<f64>() {
                // It's a float
                serde_json::json!(num)
            } else {
                // Treat as string without quotes
                serde_json::json!(value_str)
            };

            return match op {
                "=" => Ok(Filter::eq(field, value)),
                "!=" => Ok(Filter::ne(field, value)),
                ">" => Ok(Filter::gt(field, value)),
                "<" => Ok(Filter::lt(field, value)),
                ">=" => Ok(Filter::gte(field, value)),
                "<=" => Ok(Filter::lte(field, value)),
                _ => unreachable!(),
            };
        }
    }

    anyhow::bail!(
        "Invalid filter expression: '{}'\n\
         Supported formats: field = value, field > value, field < value, etc.",
        expr
    )
}

/// Get the default database path (~/.korudelta/db)
fn default_db_path() -> PathBuf {
    dirs::home_dir()
        .expect("Could not determine home directory")
        .join(".korudelta")
        .join("db")
}

/// Load or create a database
async fn load_database(path: &std::path::Path) -> Result<KoruDelta> {
    KoruDelta::start_with_path(path)
        .await
        .context("Failed to initialize database")
}

/// Save is now handled automatically in put() - this function is kept for API compatibility
async fn save_database(_db: &KoruDelta, _path: &std::path::Path) -> Result<()> {
    // Persistence is handled incrementally in put() via WAL
    Ok(())
}

/// Format a timestamp in a human-readable way
fn format_timestamp(ts: &DateTime<Utc>) -> String {
    ts.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

/// Format JSON for pretty printing
fn format_json(value: &JsonValue) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

/// Show a colored diff between two JSON values
fn show_diff(old_value: &JsonValue, new_value: &JsonValue, old_label: &str, new_label: &str) {
    let old_str = format_json(old_value);
    let new_str = format_json(new_value);

    // Use similar crate to compute line-by-line diff
    let diff = TextDiff::from_lines(&old_str, &new_str);

    println!("{}", "Diff:".bold());
    println!();
    println!("  {} {}", "-".red().bold(), old_label.bright_black());
    println!("  {} {}", "+".green().bold(), new_label.bright_black());
    println!();

    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => "-".red(),
            ChangeTag::Insert => "+".green(),
            ChangeTag::Equal => " ".normal(),
        };

        let line = format!("  {sign} {}", change.value().trim_end());

        match change.tag() {
            ChangeTag::Delete => println!("{}", line.red()),
            ChangeTag::Insert => println!("{}", line.green()),
            ChangeTag::Equal => println!("{}", line.bright_black()),
        }
    }
}

/// Format peer status
fn format_peer_status(status: PeerStatus) -> ColoredString {
    match status {
        PeerStatus::Unknown => "unknown".yellow(),
        PeerStatus::Healthy => "healthy".green(),
        PeerStatus::Syncing => "syncing".cyan(),
        PeerStatus::Unreachable => "unreachable".red(),
    }
}

/// Handle remote commands via HTTP
async fn handle_remote_command(command: &Commands, url: &str) -> Result<()> {
    let client = HttpClient::new(url.to_string());

    match command {
        Commands::Set { key, value } => {
            let (namespace, key_name) = parse_key(key)?;
            let json_value: JsonValue = serde_json::from_str(value)
                .with_context(|| format!("Invalid JSON value: {}", value))?;

            let result = client.put(&namespace, &key_name, json_value).await?;
            
            println!("{}", "OK".green().bold());
            println!("  Stored: {}/{}", namespace.cyan(), key_name.cyan());
            println!("  Version: {}", result.get("versionId").and_then(|v| v.as_str()).unwrap_or("unknown").bright_black());
            if let Some(ts) = result.get("timestamp").and_then(|v| v.as_str()) {
                println!("  Timestamp: {}", ts);
            }
            Ok(())
        }

        Commands::Get { key, verbose: _, at } => {
            let (namespace, key_name) = parse_key(key)?;
            
            if let Some(timestamp_str) = at {
                // Time travel query via HTTP
                let value = client.get_at(&namespace, &key_name, timestamp_str).await?;
                println!("{}", format_json(&value));
                println!();
                println!("{}", format!("(value at {})", timestamp_str).bright_black());
            } else {
                let value = client.get(&namespace, &key_name).await?;
                println!("{}", format_json(&value));
            }
            Ok(())
        }

        Commands::Log { key, limit } => {
            let (namespace, key_name) = parse_key(key)?;
            let history = client.history(&namespace, &key_name).await?;
            
            let entries: Vec<_> = if let Some(lim) = limit {
                history.iter().rev().take(*lim).cloned().collect()
            } else {
                history.iter().rev().cloned().collect()
            };

            if entries.is_empty() {
                println!("{}", "No history found".yellow());
                return Ok(());
            }

            println!("{}", "History:".bold());
            println!();

            for entry in entries {
                if let Some(ts) = entry.get("timestamp").and_then(|v| v.as_str()) {
                    println!("  {} {}", "*".cyan(), ts.bright_black());
                }
                if let Some(value) = entry.get("value") {
                    println!("    {}", format_json(value).bright_white());
                }
                if let Some(vid) = entry.get("versionId").and_then(|v| v.as_str()) {
                    println!("    Version: {}", vid.bright_black());
                }
                println!();
            }

            println!("  {} {} total", history.len(), if history.len() == 1 { "version" } else { "versions" });
            Ok(())
        }

        Commands::Status => {
            let status = client.status().await?;
            println!("{}", "Database Status".bold().cyan());
            println!();
            println!("  {} {}", "Keys:".bright_white(), status.get("keyCount").and_then(|v| v.as_u64()).unwrap_or(0));
            println!("  {} {}", "Versions:".bright_white(), status.get("totalVersions").and_then(|v| v.as_u64()).unwrap_or(0));
            println!("  {} {}", "Namespaces:".bright_white(), status.get("namespaceCount").and_then(|v| v.as_u64()).unwrap_or(0));
            println!();

            if let Some(namespaces) = status.get("namespaces").and_then(|v| v.as_array()) {
                if !namespaces.is_empty() {
                    println!("{}", "Namespaces:".bright_white());
                    for ns in namespaces {
                        if let Some(name) = ns.as_str() {
                            println!("  {} {}", "*".cyan(), name);
                        }
                    }
                }
            }
            Ok(())
        }

        Commands::List { namespace } => {
            match namespace {
                Some(ns) => {
                    let keys = client.list_keys(ns).await?;
                    if keys.is_empty() {
                        println!("{}", format!("No keys in namespace '{}'", ns).yellow());
                    } else {
                        println!("{}", format!("Keys in '{}':", ns).bold());
                        println!();
                        for key in keys {
                            println!("  {} {}/{}", "*".cyan(), ns.bright_black(), key);
                        }
                    }
                }
                None => {
                    let namespaces = client.list_namespaces().await?;
                    if namespaces.is_empty() {
                        println!("{}", "No namespaces found".yellow());
                    } else {
                        println!("{}", "Namespaces:".bold());
                        println!();
                        for ns in namespaces {
                            println!("  {} {}", "*".cyan(), ns);
                        }
                    }
                }
            }
            Ok(())
        }

        Commands::Query { namespace, filter, sort, desc, limit, count: _count, sum: _sum, avg: _avg } => {
            let mut query = serde_json::json!({});
            
            if let Some(f) = filter {
                // Parse simple filter expressions like "age > 30"
                query["filter"] = serde_json::json!({
                    "field": f.split_whitespace().next().unwrap_or(""),
                    "op": "eq",
                    "value": f
                });
            }
            
            if let Some(s) = sort {
                query["sort"] = serde_json::json!({
                    "field": s,
                    "descending": *desc
                });
            }
            
            if let Some(l) = limit {
                query["limit"] = serde_json::json!(l);
            }

            let result = client.query(namespace, query).await?;
            
            if let Some(records) = result.get("results").and_then(|v| v.as_array()) {
                println!("{} ({} records)", "Query results:".bold(), records.len());
                println!();
                
                for record in records {
                    if let Some(key) = record.get("key").and_then(|v| v.as_str()) {
                        println!("  {} {}", "*".cyan(), key.bright_white());
                    }
                    if let Some(value) = record.get("value") {
                        println!("    {}", format_json(value).bright_black());
                    }
                }
            }
            
            Ok(())
        }

        Commands::View(view_cmd) => match view_cmd {
            ViewCommands::Create { name, source, filter, description, auto_refresh } => {
                let mut view_def = serde_json::json!({
                    "name": name,
                    "source": source,
                    "auto_refresh": *auto_refresh
                });
                
                if let Some(desc) = description {
                    view_def["description"] = serde_json::json!(desc);
                }
                
                if let Some(f) = filter {
                    view_def["filter"] = serde_json::json!({
                        "field": f.split_whitespace().next().unwrap_or(""),
                        "op": "eq",
                        "value": f
                    });
                }

                client.create_view(view_def).await?;
                println!("{}", format!("View '{}' created.", name).green().bold());
                Ok(())
            }

            ViewCommands::List => {
                let views = client.list_views().await?;
                if views.is_empty() {
                    println!("{}", "No views found".yellow());
                } else {
                    println!("{}", "Materialized views:".bold());
                    println!();
                    for view in views {
                        if let Some(name) = view.get("name").and_then(|v| v.as_str()) {
                            println!("  {} {}", "*".cyan(), name.bright_white());
                        }
                    }
                }
                Ok(())
            }

            ViewCommands::Refresh { name } => {
                client.refresh_view(name).await?;
                println!("{}", format!("View '{}' refreshed.", name).green());
                Ok(())
            }

            ViewCommands::Query { name, limit } => {
                let result = client.query_view(name).await?;
                
                if let Some(records) = result.get("results").and_then(|v| v.as_array()) {
                    let records: Vec<_> = if let Some(l) = limit {
                        records.iter().take(*l).cloned().collect()
                    } else {
                        records.clone()
                    };
                    
                    println!("{} ({} records)", format!("View '{}' results:", name).bold(), records.len());
                    println!();
                    
                    for record in records {
                        if let Some(key) = record.get("key").and_then(|v| v.as_str()) {
                            println!("  {} {}", "*".cyan(), key.bright_white());
                        }
                        if let Some(value) = record.get("value") {
                            println!("    {}", format_json(value).bright_black());
                        }
                    }
                }
                Ok(())
            }

            ViewCommands::Delete { name } => {
                client.delete_view(name).await?;
                println!("{}", format!("View '{}' deleted.", name).green());
                Ok(())
            }
        },

        Commands::Diff { key, at: _at, from, to } => {
            let (namespace, key_name) = parse_key(key)?;
            let history = client.history(&namespace, &key_name).await?;

            if history.len() < 2 {
                println!("{}", "Need at least 2 versions to compare".yellow());
                return Ok(());
            }

            // For now, just compare first and last
            let old_idx = from.unwrap_or(0);
            let new_idx = to.unwrap_or(history.len() - 1);

            if old_idx >= history.len() || new_idx >= history.len() {
                anyhow::bail!("Invalid version indices");
            }

            let old_val = history[old_idx].get("value").cloned().unwrap_or(serde_json::Value::Null);
            let new_val = history[new_idx].get("value").cloned().unwrap_or(serde_json::Value::Null);

            show_diff(&old_val, &new_val, &format!("Version {}", old_idx), &format!("Version {}", new_idx));
            Ok(())
        }

        Commands::Start { .. } => {
            println!("{}", "Cannot start cluster node with --url flag".red());
            println!("  Remove --url to start a local server.");
            std::process::exit(1);
        }

        Commands::Serve { .. } => {
            println!("{}", "Cannot start HTTP server with --url flag".red());
            println!("  Remove --url to start a local server.");
            std::process::exit(1);
        }

        Commands::Peers => {
            println!("{}", "Peers command not available via HTTP API yet".yellow());
            Ok(())
        }

        Commands::Watch { .. } => {
            println!("{}", "Watch command not available via HTTP API (use websockets for streaming)".yellow());
            Ok(())
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Handle remote operations via HTTP
    if let Some(url) = &cli.url {
        return handle_remote_command(&cli.command, url).await;
    }

    // Determine database path
    let db_path = cli.db_path.unwrap_or_else(default_db_path);

    // Handle start command specially (cluster node mode)
    if let Commands::Start { port, join, bind } = &cli.command {
        return run_server(&db_path, bind, *port, join.as_deref()).await;
    }

    // Handle serve command specially (HTTP API mode)
    if let Commands::Serve { port, bind } = &cli.command {
        return run_http_server(&db_path, bind, *port).await;
    }

    // Load database for other commands
    let db = load_database(&db_path)
        .await
        .context("Failed to initialize database")?;

    // Execute command - wrap in async block to ensure shutdown is called
    let result = async {
        match cli.command {
        Commands::Set { key, value } => {
            let (namespace, key_name) = parse_key(&key)?;

            // Parse value as JSON
            let json_value: JsonValue = serde_json::from_str(&value)
                .with_context(|| format!("Invalid JSON value: {}", value))?;

            // Store the value
            let versioned = db
                .put(&namespace, &key_name, json_value)
                .await
                .context("Failed to store value")?;

            // Save database to disk
            save_database(&db, &db_path)
                .await
                .context("Failed to persist changes")?;

            // Output success message
            println!("{}", "OK".green().bold());
            println!("  Stored: {}/{}", namespace.cyan(), key_name.cyan());
            println!("  Version: {}", versioned.version_id().bright_black());
            println!("  Timestamp: {}", format_timestamp(&versioned.timestamp()));

            Ok(())
        }

        Commands::Get { key, verbose, at } => {
            let (namespace, key_name) = parse_key(&key)?;

            // Handle time travel query
            if let Some(timestamp_str) = at {
                let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                    .with_context(|| format!("Invalid timestamp format: {}", timestamp_str))?
                    .with_timezone(&Utc);

                match db.get_at(&namespace, &key_name, timestamp).await {
                    Ok(versioned) => {
                        println!("{}", format_json(versioned.value()));
                        println!();
                        println!("{}", format!("(value at {})", timestamp_str).bright_black());
                        Ok(())
                    }
                    Err(DeltaError::KeyNotFound { .. }) => {
                        eprintln!("{}", "Error".red().bold());
                        eprintln!("  Key not found: {}/{}", namespace, key_name);
                        std::process::exit(1);
                    }
                    Err(DeltaError::NoValueAtTimestamp { .. }) => {
                        eprintln!("{}", "Error".red().bold());
                        eprintln!("  No value at timestamp: {}", timestamp_str);
                        std::process::exit(1);
                    }
                    Err(e) => Err(e.into()),
                }
            } else {
                match db.get(&namespace, &key_name).await {
                    Ok(versioned) => {
                        // Pretty print the value
                        println!("{}", format_json(versioned.value()));

                        if verbose {
                            println!();
                            println!("{}", "Metadata:".bright_black());
                            println!("  Version: {}", versioned.version_id().bright_black());
                            println!(
                                "  Timestamp: {}",
                                format_timestamp(&versioned.timestamp()).bright_black()
                            );
                            if let Some(prev) = versioned.previous_version() {
                                println!("  Previous: {}", prev.bright_black());
                            }
                        }

                        Ok(())
                    }
                    Err(DeltaError::KeyNotFound { .. }) => {
                        eprintln!("{}", "Error".red().bold());
                        eprintln!("  Key not found: {}/{}", namespace, key_name);
                        std::process::exit(1);
                    }
                    Err(e) => Err(e.into()),
                }
            }
        }

        Commands::Log { key, limit } => {
            let (namespace, key_name) = parse_key(&key)?;

            match db.history(&namespace, &key_name).await {
                Ok(history) => {
                    let entries = if let Some(limit) = limit {
                        history.iter().rev().take(limit).collect::<Vec<_>>()
                    } else {
                        history.iter().rev().collect::<Vec<_>>()
                    };

                    if entries.is_empty() {
                        println!("{}", "No history found".yellow());
                        return Ok(());
                    }

                    println!("{}", "History:".bold());
                    println!();

                    for entry in entries {
                        let ts = format_timestamp(&entry.timestamp);
                        println!("  {} {}", "*".cyan(), ts.bright_black());
                        println!("    {}", format_json(&entry.value).bright_white());
                        println!("    Version: {}", entry.version_id.bright_black());
                        println!();
                    }

                    println!(
                        "  {} {} total",
                        history.len(),
                        if history.len() == 1 {
                            "version"
                        } else {
                            "versions"
                        }
                    );

                    Ok(())
                }
                Err(DeltaError::KeyNotFound { .. }) => {
                    eprintln!("{}", "Error".red().bold());
                    eprintln!("  Key not found: {}/{}", namespace, key_name);
                    std::process::exit(1);
                }
                Err(e) => Err(e.into()),
            }
        }

        Commands::Status => {
            let stats = db.stats().await;
            let namespaces = db.list_namespaces().await;

            println!("{}", "Database Status".bold().cyan());
            println!();
            println!("  {} {}", "Keys:".bright_white(), stats.key_count);
            println!("  {} {}", "Versions:".bright_white(), stats.total_versions);
            println!(
                "  {} {}",
                "Namespaces:".bright_white(),
                stats.namespace_count
            );
            println!();

            if !namespaces.is_empty() {
                println!("{}", "Namespaces:".bright_white());
                for ns in &namespaces {
                    let keys = db.list_keys(ns).await;
                    println!(
                        "  {} {} ({} {})",
                        "*".cyan(),
                        ns,
                        keys.len(),
                        if keys.len() == 1 { "key" } else { "keys" }
                    );
                }
                println!();
            }

            println!("  {} {}", "Database:".bright_black(), db_path.display());

            Ok(())
        }

        Commands::List { namespace } => {
            match namespace {
                Some(ns) => {
                    // List keys in namespace
                    let keys = db.list_keys(&ns).await;

                    if keys.is_empty() {
                        println!("{}", format!("No keys in namespace '{}'", ns).yellow());
                        return Ok(());
                    }

                    println!("{}", format!("Keys in '{}':", ns).bold());
                    println!();
                    for key in keys {
                        println!("  {} {}/{}", "*".cyan(), ns.bright_black(), key);
                    }
                }
                None => {
                    // List all namespaces
                    let namespaces = db.list_namespaces().await;

                    if namespaces.is_empty() {
                        println!("{}", "No namespaces found".yellow());
                        return Ok(());
                    }

                    println!("{}", "Namespaces:".bold());
                    println!();
                    for ns in namespaces {
                        let keys = db.list_keys(&ns).await;
                        println!(
                            "  {} {} ({} {})",
                            "*".cyan(),
                            ns,
                            keys.len(),
                            if keys.len() == 1 { "key" } else { "keys" }
                        );
                    }
                }
            }

            Ok(())
        }

        Commands::Diff { key, at, from, to } => {
            let (namespace, key_name) = parse_key(&key)?;

            // Get history for this key
            let history = match db.history(&namespace, &key_name).await {
                Ok(h) => h,
                Err(DeltaError::KeyNotFound { .. }) => {
                    eprintln!("{}", "Error".red().bold());
                    eprintln!("  Key not found: {}/{}", namespace, key_name);
                    std::process::exit(1);
                }
                Err(e) => return Err(e.into()),
            };

            if history.is_empty() {
                println!("{}", "No history found".yellow());
                return Ok(());
            }

            // Determine which versions to compare
            let (old_idx, new_idx) = if let (Some(f), Some(t)) = (from, to) {
                // Explicit indices provided
                if f >= history.len() || t >= history.len() {
                    anyhow::bail!(
                        "Invalid version indices. History has {} versions (0-{})",
                        history.len(),
                        history.len() - 1
                    );
                }
                (f, t)
            } else if let Some(timestamp_str) = at {
                // Find version at timestamp and compare with previous
                let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                    .with_context(|| format!("Invalid timestamp format: {}", timestamp_str))?
                    .with_timezone(&Utc);

                // Find version at or before this timestamp
                let idx = history
                    .iter()
                    .position(|e| e.timestamp >= timestamp)
                    .unwrap_or(history.len() - 1);

                if idx == 0 {
                    anyhow::bail!("No previous version to compare with");
                }
                (idx - 1, idx)
            } else {
                // Default: compare latest with previous
                if history.len() < 2 {
                    println!("{}", "Only one version exists, nothing to compare".yellow());
                    return Ok(());
                }
                (history.len() - 2, history.len() - 1)
            };

            let old_entry = &history[old_idx];
            let new_entry = &history[new_idx];

            // Show metadata
            println!("{}", "Comparing versions:".bold());
            println!();
            println!(
                "  {} Version {} ({})",
                "-".red().bold(),
                old_idx,
                format_timestamp(&old_entry.timestamp).bright_black()
            );
            println!(
                "  {} Version {} ({})",
                "+".green().bold(),
                new_idx,
                format_timestamp(&new_entry.timestamp).bright_black()
            );
            println!();

            // Show diff
            show_diff(
                &old_entry.value,
                &new_entry.value,
                &format!("Version {}", old_idx),
                &format!("Version {}", new_idx),
            );

            Ok(())
        }

        Commands::Peers => {
            println!("{}", "Cluster peers not available in client mode.".yellow());
            println!("  Use 'kdelta start' to run in server mode and see peers.");
            Ok(())
        }

        Commands::Query {
            namespace,
            filter,
            sort,
            desc,
            limit,
            count,
            sum,
            avg,
        } => {
            // Build the query
            let mut query = Query::new();

            // Add filter if provided
            if let Some(filter_expr) = filter {
                let filter = parse_filter(&filter_expr)?;
                query = query.filter(filter);
            }

            // Add sort if provided
            if let Some(sort_field) = sort {
                query = query.sort_by(sort_field, !desc);
            }

            // Add limit if provided
            if let Some(lim) = limit {
                query = query.limit(lim);
            }

            // Add aggregation
            if count {
                query = query.aggregate(Aggregation::count());
            } else if let Some(field) = sum {
                query = query.aggregate(Aggregation::sum(field));
            } else if let Some(field) = avg {
                query = query.aggregate(Aggregation::avg(field));
            }

            // Execute query
            let results = db
                .query(&namespace, query)
                .await
                .context("Failed to execute query")?;

            // Show results
            if count {
                println!("{}", "Query result:".bold());
                println!("  Count: {}", results.total_count.to_string().cyan());
            } else {
                println!(
                    "{} ({} {})",
                    "Query results:".bold(),
                    results.records.len(),
                    if results.records.len() == 1 {
                        "record"
                    } else {
                        "records"
                    }
                );
                println!();

                for record in &results.records {
                    println!("  {} {}", "*".cyan(), record.key.bright_white());
                    println!("    {}", format_json(&record.value).bright_black());
                }
            }

            Ok(())
        }

        Commands::View(view_cmd) => match view_cmd {
            ViewCommands::Create {
                name,
                source,
                filter,
                description,
                auto_refresh,
            } => {
                let mut definition = ViewDefinition::new(&name, &source);

                if let Some(filter_expr) = filter {
                    let filter = parse_filter(&filter_expr)?;
                    definition = definition.with_query(Query::new().filter(filter));
                }

                if let Some(desc) = description {
                    definition = definition.with_description(desc);
                }

                if auto_refresh {
                    definition = definition.auto_refresh(true);
                }

                let info = db
                    .create_view(definition)
                    .await
                    .context("Failed to create view")?;

                println!("{}", "View created:".bold().green());
                println!("  Name: {}", info.name.cyan());
                println!("  Source: {}", info.source_collection);
                println!("  Records: {}", info.record_count);
                if let Some(desc) = &info.description {
                    println!("  Description: {}", desc.bright_black());
                }

                Ok(())
            }

            ViewCommands::List => {
                let views = db.list_views().await;

                if views.is_empty() {
                    println!("{}", "No views found".yellow());
                    return Ok(());
                }

                println!("{}", "Materialized views:".bold());
                println!();

                for view in views {
                    println!("  {} {}", "*".cyan(), view.name.bright_white());
                    println!("    Source: {}", view.source_collection);
                    println!("    Records: {}", view.record_count);
                    println!(
                        "    Last refreshed: {}",
                        format_timestamp(&view.last_refreshed).bright_black()
                    );
                    if view.auto_refresh {
                        println!("    Auto-refresh: {}", "enabled".green());
                    }
                    if let Some(desc) = &view.description {
                        println!("    Description: {}", desc.bright_black());
                    }
                    println!();
                }

                Ok(())
            }

            ViewCommands::Refresh { name } => {
                let info = db
                    .refresh_view(&name)
                    .await
                    .context("Failed to refresh view")?;

                println!("{}", "View refreshed:".bold().green());
                println!("  Name: {}", info.name.cyan());
                println!("  Records: {}", info.record_count);
                println!(
                    "  Last refreshed: {}",
                    format_timestamp(&info.last_refreshed)
                );

                Ok(())
            }

            ViewCommands::Query { name, limit } => {
                let result = db.query_view(&name).await.context("Failed to query view")?;

                let records = if let Some(lim) = limit {
                    result.records.into_iter().take(lim).collect::<Vec<_>>()
                } else {
                    result.records
                };

                println!(
                    "{} ({} {})",
                    format!("View '{}' results:", name).bold(),
                    records.len(),
                    if records.len() == 1 {
                        "record"
                    } else {
                        "records"
                    }
                );
                println!();

                for record in records {
                    println!("  {} {}", "*".cyan(), record.key.bright_white());
                    println!("    {}", format_json(&record.value).bright_black());
                }

                Ok(())
            }

            ViewCommands::Delete { name } => {
                db.delete_view(&name)
                    .await
                    .context("Failed to delete view")?;

                println!("{}", format!("View '{}' deleted.", name).green());

                Ok(())
            }
        },

        Commands::Watch {
            target,
            all,
            inserts_only,
            updates_only,
        } => {
            // Build subscription
            let subscription = if all {
                Subscription::all()
            } else if let Some(t) = target {
                // Check if it's a key (contains /) or just a namespace
                if t.contains('/') {
                    let (ns, key) = parse_key(&t)?;
                    Subscription::key(ns, key)
                } else {
                    Subscription::collection(t)
                }
            } else {
                Subscription::all()
            };

            // Apply change type filters
            let subscription = if inserts_only {
                subscription.inserts_only()
            } else if updates_only {
                subscription.updates_only()
            } else {
                subscription
            };

            let (_id, mut rx) = db.subscribe(subscription).await;

            println!("{}", "Watching for changes...".bold().cyan());
            println!("  Press Ctrl+C to stop.");
            println!();

            // Watch loop
            loop {
                tokio::select! {
                    event = rx.recv() => {
                        match event {
                            Ok(change) => {
                                let change_type = match change.change_type {
                                    ChangeType::Insert => "INSERT".green(),
                                    ChangeType::Update => "UPDATE".yellow(),
                                    ChangeType::Delete => "DELETE".red(),
                                };

                                println!(
                                    "{} {}/{} {}",
                                    change_type,
                                    change.collection.cyan(),
                                    change.key.bright_white(),
                                    format_timestamp(&change.timestamp).bright_black()
                                );

                                if let Some(value) = &change.value {
                                    println!("  {}", format_json(value).bright_black());
                                }
                                println!();
                            }
                            Err(_) => {
                                // Channel lagged, continue
                                continue;
                            }
                        }
                    }
                    _ = signal::ctrl_c() => {
                        println!();
                        println!("{}", "Stopped watching.".yellow());
                        break;
                    }
                }
            }

            Ok(())
        }

        // Start and Serve are handled above
        Commands::Start { .. } => unreachable!(),
        Commands::Serve { .. } => unreachable!(),
    }
    }.await;
    
    // Shutdown database to release lock
    db.shutdown().await.ok();
    
    result
}

/// Run the server (cluster node mode)
async fn run_server(
    db_path: &std::path::Path,
    bind: &str,
    port: u16,
    join: Option<&str>,
) -> Result<()> {
    // Parse bind address
    let bind_addr: SocketAddr = format!("{}:{}", bind, port)
        .parse()
        .with_context(|| format!("Invalid bind address: {}:{}", bind, port))?;

    // Parse join address if provided
    let join_addr: Option<SocketAddr> = match join {
        Some(addr_str) => {
            let addr = if addr_str.contains(':') {
                addr_str.to_string()
            } else {
                format!("{}:{}", addr_str, DEFAULT_PORT)
            };
            Some(
                addr.parse()
                    .with_context(|| format!("Invalid join address: {}", addr_str))?,
            )
        }
        None => None,
    };

    // Load or create database
    let db = load_database(db_path)
        .await
        .context("Failed to initialize database")?;

    // Create cluster config
    let mut config = ClusterConfig::new().bind_addr(bind_addr);
    if let Some(addr) = join_addr {
        config = config.join(addr);
    }

    // Create cluster node and attach to database for write broadcasting
    let node = std::sync::Arc::new(ClusterNode::new(db.storage().clone(), db.engine().clone(), config));
    let db = db.with_cluster(node.clone());

    println!("{}", "Starting KoruDelta node...".bold().cyan());
    println!();

    node.start().await.context("Failed to start cluster node")?;

    println!("  {} {}", "Node ID:".bright_white(), node.node_id());
    println!("  {} {}", "Address:".bright_white(), bind_addr);

    if let Some(addr) = join_addr {
        println!("  {} {}", "Joined:".bright_white(), addr);
    }

    println!();
    println!("{}", "Node is running. Press Ctrl+C to stop.".green());
    println!();

    // Show peers periodically and handle Ctrl+C
    let shutdown = async {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;

            let peers = node.peers();
            if !peers.is_empty() {
                println!("{}", "Cluster peers:".bright_black());
                for peer in &peers {
                    println!(
                        "  {} {} ({}) - {}",
                        "*".cyan(),
                        peer.node_id,
                        peer.address,
                        format_peer_status(peer.status)
                    );
                }
                println!();
            }

            // Auto-save periodically
            if let Err(e) = save_database(&db, db_path).await {
                eprintln!("Warning: Failed to auto-save: {}", e);
            }
        }
    };

    tokio::select! {
        _ = shutdown => {}
        _ = signal::ctrl_c() => {
            println!();
            println!("{}", "Shutting down...".yellow());
        }
    }

    // Stop the node
    node.stop().await.context("Failed to stop cluster node")?;

    // Final save
    save_database(&db, db_path)
        .await
        .context("Failed to save database on shutdown")?;

    println!("{}", "Node stopped.".green());

    Ok(())
}

/// Run the HTTP API server
async fn run_http_server(
    db_path: &std::path::Path,
    bind: &str,
    port: u16,
) -> Result<()> {
    use koru_delta::http::HttpServer;

    // Parse bind address
    let bind_addr = format!("{}:{}", bind, port);

    // Load or create database
    let db = load_database(db_path)
        .await
        .context("Failed to initialize database")?;

    println!("{}", "Starting KoruDelta HTTP server...".bold().cyan());
    println!();
    println!("  {} {}", "Bind:".bright_white(), bind_addr);
    println!();
    println!("  {}", "Endpoints:".bright_black());
    println!("    GET    /api/v1/:namespace/:key              - Get value");
    println!("    PUT    /api/v1/:namespace/:key              - Store value");
    println!("    GET    /api/v1/:namespace/:key/history      - Get history");
    println!("    GET    /api/v1/:namespace/:key/at/:timestamp - Time travel");
    println!("    POST   /api/v1/:namespace/query             - Execute query");
    println!("    GET    /api/v1/views                        - List views");
    println!("    POST   /api/v1/views                        - Create view");
    println!("    GET    /api/v1/status                       - Database status");
    println!();
    println!("{}", "Server is running. Press Ctrl+C to stop.".green());
    println!();

    // Create and start HTTP server
    let server = HttpServer::new(db);
    
    // Handle Ctrl+C for graceful shutdown
    let shutdown = async {
        signal::ctrl_c().await.ok();
        println!();
        println!("{}", "Shutting down...".yellow());
    };

    tokio::select! {
        result = server.bind(&bind_addr) => {
            if let Err(e) = result {
                eprintln!("{} {}", "Server error:".red(), e);
            }
        }
        _ = shutdown => {}
    }

    println!("{}", "Server stopped.".green());
    Ok(())
}
