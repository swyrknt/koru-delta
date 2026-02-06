/// HTTP API for KoruDelta.
///
/// This module provides a RESTful HTTP interface for interacting with
/// KoruDelta over the network. It enables remote clients to:
///
/// - Store and retrieve values
/// - Query history and perform time-travel queries
/// - Execute filtered queries
/// - Manage views
/// - Monitor database status
///
/// # Example
///
/// ```ignore
/// use koru_delta::http::HttpServer;
///
/// let db = KoruDelta::start().await?;
/// let server = HttpServer::new(db);
/// server.bind("0.0.0.0:8080").await?;
/// ```
///
/// # API Endpoints
///
/// ## Key-Value Operations
/// - `GET /api/v1/:namespace/:key` - Get current value
/// - `PUT /api/v1/:namespace/:key` - Store value
/// - `GET /api/v1/:namespace/:key/history` - Get history
/// - `GET /api/v1/:namespace/:key/at/:timestamp` - Time travel
///
/// ## Queries
/// - `POST /api/v1/:namespace/query` - Execute query
///
/// ## Views
/// - `GET /api/v1/views` - List views
/// - `POST /api/v1/views` - Create view
/// - `GET /api/v1/views/:name` - Query view
/// - `POST /api/v1/views/:name/refresh` - Refresh view
/// - `DELETE /api/v1/views/:name` - Delete view
///
/// ## Status
/// - `GET /api/v1/status` - Database status
/// - `GET /api/v1/namespaces` - List namespaces
/// - `GET /api/v1/:namespace/keys` - List keys
use crate::core::KoruDelta;
use crate::error::DeltaResult;
use crate::query::{Filter, Query};
use crate::views::ViewDefinition;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::net::SocketAddr;
use std::sync::Arc;

/// HTTP server for KoruDelta.
pub struct HttpServer {
    db: KoruDelta,
}

impl HttpServer {
    /// Create a new HTTP server with the given database.
    pub fn new(db: KoruDelta) -> Self {
        Self { db }
    }

    /// Start the HTTP server on the given address.
    ///
    /// # Example
    ///
    /// ```ignore
    /// server.bind("0.0.0.0:8080").await?;
    /// ```
    pub async fn bind(self, addr: &str) -> DeltaResult<()> {
        let addr: SocketAddr = addr.parse()
            .map_err(|e| crate::error::DeltaError::StorageError(format!("Invalid address: {}", e)))?;
        let db = Arc::new(self.db);

        let app = create_router(db);

        let listener = tokio::net::TcpListener::bind(addr).await
            .map_err(|e| crate::error::DeltaError::StorageError(format!("Failed to bind: {}", e)))?;
        axum::serve(listener, app).await
            .map_err(|e| crate::error::DeltaError::StorageError(format!("Server error: {}", e)))?;

        Ok(())
    }
}

/// Create the Axum router with all routes.
fn create_router(db: Arc<KoruDelta>) -> axum::Router {
    use axum::routing::{delete, get, post, put};
    use axum::Router;

    Router::new()
        // Key-value operations
        .route("/api/v1/:namespace/:key", get(handle_get))
        .route("/api/v1/:namespace/:key", put(handle_put))
        .route("/api/v1/:namespace/:key/history", get(handle_history))
        .route("/api/v1/:namespace/:key/at/:timestamp", get(handle_get_at))
        // Queries
        .route("/api/v1/:namespace/query", post(handle_query))
        // Views
        .route("/api/v1/views", get(handle_list_views))
        .route("/api/v1/views", post(handle_create_view))
        .route("/api/v1/views/:name", get(handle_query_view))
        .route("/api/v1/views/:name/refresh", post(handle_refresh_view))
        .route("/api/v1/views/:name", delete(handle_delete_view))
        // Status
        .route("/api/v1/status", get(handle_status))
        .route("/api/v1/namespaces", get(handle_list_namespaces))
        .route("/api/v1/:namespace/keys", get(handle_list_keys))
        .with_state(db)
}

// State extractor type
use axum::extract::State;

/// Response for a versioned value.
#[derive(Debug, Serialize)]
struct VersionedResponse {
    value: JsonValue,
    version_id: String,
    timestamp: DateTime<Utc>,
    previous_version: Option<String>,
}

/// Request body for PUT /api/v1/:namespace/:key
#[derive(Debug, Deserialize)]
struct PutRequest {
    value: JsonValue,
}

/// Response for PUT /api/v1/:namespace/:key
#[derive(Debug, Serialize)]
struct PutResponse {
    version_id: String,
    timestamp: DateTime<Utc>,
    previous_version: Option<String>,
}

/// Response for history endpoint.
#[derive(Debug, Serialize)]
struct HistoryResponse {
    key: String,
    namespace: String,
    versions: Vec<HistoryEntryResponse>,
}

#[derive(Debug, Serialize)]
struct HistoryEntryResponse {
    value: JsonValue,
    version_id: String,
    timestamp: DateTime<Utc>,
}

/// Request for query endpoint.
#[derive(Debug, Deserialize)]
struct QueryRequest {
    #[serde(default)]
    filter: Option<FilterDef>,
    #[serde(default)]
    sort: Option<SortDef>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct FilterDef {
    field: String,
    op: String,
    value: JsonValue,
}

#[derive(Debug, Deserialize)]
struct SortDef {
    field: String,
    #[serde(default)]
    descending: bool,
}

/// Query response.
#[derive(Debug, Serialize)]
struct QueryResponse {
    results: Vec<QueryRecordResponse>,
    total: usize,
    namespace: String,
}

#[derive(Debug, Serialize)]
struct QueryRecordResponse {
    key: String,
    value: JsonValue,
    version_id: String,
    timestamp: DateTime<Utc>,
}

/// Status response.
#[derive(Debug, Serialize)]
struct StatusResponse {
    key_count: usize,
    total_versions: usize,
    namespace_count: usize,
    namespaces: Vec<String>,
}

/// View creation request.
#[derive(Debug, Deserialize)]
struct CreateViewRequest {
    name: String,
    source: String,
    #[serde(default)]
    filter: Option<FilterDef>,
    #[serde(default)]
    auto_refresh: bool,
}

/// View list response.
#[derive(Debug, Serialize)]
struct ViewsResponse {
    views: Vec<ViewInfoResponse>,
}

#[derive(Debug, Serialize)]
struct ViewInfoResponse {
    name: String,
    source: String,
    auto_refresh: bool,
}

// Handler implementations

async fn handle_get(
    State(db): State<Arc<KoruDelta>>,
    axum::extract::Path((namespace, key)): axum::extract::Path<(String, String)>,
) -> Result<axum::Json<VersionedResponse>, axum::http::StatusCode> {
    match db.get(&namespace, &key).await {
        Ok(versioned) => {
            let response = VersionedResponse {
                value: versioned.value().clone(),
                version_id: versioned.version_id().to_string(),
                timestamp: versioned.timestamp(),
                previous_version: versioned.previous_version().map(|s| s.to_string()),
            };
            Ok(axum::Json(response))
        }
        Err(_) => Err(axum::http::StatusCode::NOT_FOUND),
    }
}

async fn handle_put(
    State(db): State<Arc<KoruDelta>>,
    axum::extract::Path((namespace, key)): axum::extract::Path<(String, String)>,
    axum::Json(request): axum::Json<PutRequest>,
) -> Result<axum::Json<PutResponse>, axum::http::StatusCode> {
    match db.put(&namespace, &key, request.value).await {
        Ok(versioned) => {
            let response = PutResponse {
                version_id: versioned.version_id().to_string(),
                timestamp: versioned.timestamp(),
                previous_version: versioned.previous_version().map(|s| s.to_string()),
            };
            Ok(axum::Json(response))
        }
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn handle_history(
    State(db): State<Arc<KoruDelta>>,
    axum::extract::Path((namespace, key)): axum::extract::Path<(String, String)>,
) -> Result<axum::Json<HistoryResponse>, axum::http::StatusCode> {
    match db.history(&namespace, &key).await {
        Ok(history) => {
            let versions: Vec<_> = history
                .into_iter()
                .map(|entry| HistoryEntryResponse {
                    value: entry.value,
                    version_id: entry.version_id,
                    timestamp: entry.timestamp,
                })
                .collect();

            let response = HistoryResponse {
                key,
                namespace,
                versions,
            };
            Ok(axum::Json(response))
        }
        Err(_) => Err(axum::http::StatusCode::NOT_FOUND),
    }
}

async fn handle_get_at(
    State(db): State<Arc<KoruDelta>>,
    axum::extract::Path((namespace, key, timestamp)): axum::extract::Path<(String, String, String)>,
) -> Result<axum::Json<VersionedResponse>, axum::http::StatusCode> {
    // Parse ISO 8601 timestamp
    let timestamp = match DateTime::parse_from_rfc3339(&timestamp) {
        Ok(ts) => ts.with_timezone(&Utc),
        Err(_) => return Err(axum::http::StatusCode::BAD_REQUEST),
    };

    match db.get_at(&namespace, &key, timestamp).await {
        Ok(versioned) => {
            let response = VersionedResponse {
                value: versioned.value().clone(),
                version_id: versioned.version_id().to_string(),
                timestamp: versioned.timestamp(),
                previous_version: versioned.previous_version().map(|s| s.to_string()),
            };
            Ok(axum::Json(response))
        }
        Err(_) => Err(axum::http::StatusCode::NOT_FOUND),
    }
}

async fn handle_query(
    State(db): State<Arc<KoruDelta>>,
    axum::extract::Path(namespace): axum::extract::Path<String>,
    axum::Json(request): axum::Json<QueryRequest>,
) -> Result<axum::Json<QueryResponse>, axum::http::StatusCode> {
    let mut query = Query::new();

    // Build filter if provided
    if let Some(filter_def) = request.filter {
        let filter = parse_filter(filter_def)?;
        query = query.filter(filter);
    }

    // Add sort if provided
    if let Some(sort_def) = request.sort {
        query = query.sort_by(&sort_def.field, sort_def.descending);
    }

    // Add limit if provided
    if let Some(limit) = request.limit {
        query = query.limit(limit);
    }

    match db.query(&namespace, query).await {
        Ok(results) => {
            let total = results.total_count;
            let records: Vec<_> = results
                .records
                .into_iter()
                .map(|record| QueryRecordResponse {
                    key: record.key,
                    value: record.value,
                    version_id: record.version_id,
                    timestamp: record.timestamp,
                })
                .collect();

            let response = QueryResponse {
                results: records,
                total,
                namespace,
            };
            Ok(axum::Json(response))
        }
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

fn parse_filter(def: FilterDef) -> Result<Filter, axum::http::StatusCode> {
    match def.op.as_str() {
        "eq" => Ok(Filter::eq(&def.field, def.value)),
        "ne" => Ok(Filter::ne(&def.field, def.value)),
        "gt" => Ok(Filter::gt(&def.field, def.value)),
        "gte" => Ok(Filter::gte(&def.field, def.value)),
        "lt" => Ok(Filter::lt(&def.field, def.value)),
        "lte" => Ok(Filter::lte(&def.field, def.value)),
        "contains" => Ok(Filter::contains(&def.field, def.value)),
        "exists" => Ok(Filter::exists(&def.field)),
        _ => Err(axum::http::StatusCode::BAD_REQUEST),
    }
}

async fn handle_list_views(
    State(db): State<Arc<KoruDelta>>,
) -> Result<axum::Json<ViewsResponse>, axum::http::StatusCode> {
    let views = db.list_views().await;
    let view_infos: Vec<_> = views
        .into_iter()
        .map(|v| ViewInfoResponse {
            name: v.name,
            source: v.source_collection,
            auto_refresh: v.auto_refresh,
        })
        .collect();

    Ok(axum::Json(ViewsResponse { views: view_infos }))
}

async fn handle_create_view(
    State(db): State<Arc<KoruDelta>>,
    axum::Json(request): axum::Json<CreateViewRequest>,
) -> Result<axum::Json<serde_json::Value>, axum::http::StatusCode> {
    let mut def = ViewDefinition::new(&request.name, &request.source);

    if let Some(filter_def) = request.filter {
        let filter = parse_filter(filter_def)?;
        let query = Query::new().filter(filter);
        def = def.with_query(query);
    }

    if request.auto_refresh {
        def = def.auto_refresh(true);
    }

    match db.create_view(def).await {
        Ok(_) => Ok(axum::Json(serde_json::json!({ "created": true }))),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn handle_query_view(
    State(db): State<Arc<KoruDelta>>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<axum::Json<QueryResponse>, axum::http::StatusCode> {
    match db.query_view(&name).await {
        Ok(result) => {
            let records: Vec<_> = result
                .records
                .into_iter()
                .map(|record| QueryRecordResponse {
                    key: record.key,
                    value: record.value,
                    version_id: record.version_id,
                    timestamp: record.timestamp,
                })
                .collect();

            let response = QueryResponse {
                results: records,
                total: result.total_count,
                namespace: name, // View name as "namespace" in response
            };
            Ok(axum::Json(response))
        }
        Err(_) => Err(axum::http::StatusCode::NOT_FOUND),
    }
}

async fn handle_refresh_view(
    State(db): State<Arc<KoruDelta>>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<axum::Json<serde_json::Value>, axum::http::StatusCode> {
    match db.refresh_view(&name).await {
        Ok(_) => Ok(axum::Json(serde_json::json!({ "refreshed": true }))),
        Err(_) => Err(axum::http::StatusCode::NOT_FOUND),
    }
}

async fn handle_delete_view(
    State(db): State<Arc<KoruDelta>>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<axum::http::StatusCode, axum::http::StatusCode> {
    match db.delete_view(&name).await {
        Ok(_) => Ok(axum::http::StatusCode::NO_CONTENT),
        Err(_) => Err(axum::http::StatusCode::NOT_FOUND),
    }
}

async fn handle_status(
    State(db): State<Arc<KoruDelta>>,
) -> Result<axum::Json<StatusResponse>, axum::http::StatusCode> {
    let stats = db.stats().await;
    let namespaces = db.list_namespaces().await;

    let response = StatusResponse {
        key_count: stats.key_count,
        total_versions: stats.total_versions,
        namespace_count: stats.namespace_count,
        namespaces,
    };

    Ok(axum::Json(response))
}

async fn handle_list_namespaces(
    State(db): State<Arc<KoruDelta>>,
) -> axum::Json<serde_json::Value> {
    let namespaces = db.list_namespaces().await;
    axum::Json(serde_json::json!({ "namespaces": namespaces }))
}

async fn handle_list_keys(
    State(db): State<Arc<KoruDelta>>,
    axum::extract::Path(namespace): axum::extract::Path<String>,
) -> axum::Json<serde_json::Value> {
    let keys = db.list_keys(&namespace).await;
    axum::Json(serde_json::json!({ "namespace": namespace, "keys": keys }))
}

#[cfg(test)]
mod tests {
    // Note: HTTP tests would require spinning up the server and making requests
    // For now, we rely on integration tests in tests/http_api_tests.rs
}
