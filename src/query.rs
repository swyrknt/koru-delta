/// Query engine for KoruDelta.
///
/// This module provides a powerful query language for filtering, projecting,
/// and aggregating data stored in KoruDelta. It supports:
///
/// - **Filtering**: Select records matching specific criteria
/// - **Projection**: Select specific fields from documents
/// - **Aggregation**: Compute statistics (count, sum, avg, min, max)
/// - **Sorting**: Order results by field values
/// - **Limiting**: Restrict the number of results
/// - **History queries**: Query across all versions of a key
///
/// # Example
///
/// ```ignore
/// use koru_delta::query::{Query, Filter, Aggregation};
///
/// // Find all users over 30
/// let query = Query::new()
///     .filter(Filter::gt("age", 30))
///     .project(&["name", "email"])
///     .sort_by("name", true)
///     .limit(10);
///
/// let results = db.query("users", query).await?;
/// ```
use crate::error::DeltaResult;
use crate::types::HistoryEntry;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as JsonValue};
use std::cmp::Ordering;

/// A filter condition for querying data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Filter {
    /// Field equals value.
    Eq { field: String, value: JsonValue },
    /// Field not equals value.
    Ne { field: String, value: JsonValue },
    /// Field greater than value.
    Gt { field: String, value: JsonValue },
    /// Field greater than or equal to value.
    Gte { field: String, value: JsonValue },
    /// Field less than value.
    Lt { field: String, value: JsonValue },
    /// Field less than or equal to value.
    Lte { field: String, value: JsonValue },
    /// Field contains substring (for strings) or element (for arrays).
    Contains { field: String, value: JsonValue },
    /// Field exists (is not null/missing).
    Exists { field: String },
    /// Field matches regex pattern (for strings).
    Matches { field: String, pattern: String },
    /// Logical AND of multiple filters.
    And(Vec<Filter>),
    /// Logical OR of multiple filters.
    Or(Vec<Filter>),
    /// Logical NOT of a filter.
    Not(Box<Filter>),
}

impl Filter {
    /// Create an equality filter.
    pub fn eq(field: impl Into<String>, value: impl Into<JsonValue>) -> Self {
        Self::Eq {
            field: field.into(),
            value: value.into(),
        }
    }

    /// Create a not-equals filter.
    pub fn ne(field: impl Into<String>, value: impl Into<JsonValue>) -> Self {
        Self::Ne {
            field: field.into(),
            value: value.into(),
        }
    }

    /// Create a greater-than filter.
    pub fn gt(field: impl Into<String>, value: impl Into<JsonValue>) -> Self {
        Self::Gt {
            field: field.into(),
            value: value.into(),
        }
    }

    /// Create a greater-than-or-equal filter.
    pub fn gte(field: impl Into<String>, value: impl Into<JsonValue>) -> Self {
        Self::Gte {
            field: field.into(),
            value: value.into(),
        }
    }

    /// Create a less-than filter.
    pub fn lt(field: impl Into<String>, value: impl Into<JsonValue>) -> Self {
        Self::Lt {
            field: field.into(),
            value: value.into(),
        }
    }

    /// Create a less-than-or-equal filter.
    pub fn lte(field: impl Into<String>, value: impl Into<JsonValue>) -> Self {
        Self::Lte {
            field: field.into(),
            value: value.into(),
        }
    }

    /// Create a contains filter.
    pub fn contains(field: impl Into<String>, value: impl Into<JsonValue>) -> Self {
        Self::Contains {
            field: field.into(),
            value: value.into(),
        }
    }

    /// Create an exists filter.
    pub fn exists(field: impl Into<String>) -> Self {
        Self::Exists {
            field: field.into(),
        }
    }

    /// Create a regex match filter.
    pub fn matches(field: impl Into<String>, pattern: impl Into<String>) -> Self {
        Self::Matches {
            field: field.into(),
            pattern: pattern.into(),
        }
    }

    /// Combine filters with AND.
    pub fn and(filters: Vec<Filter>) -> Self {
        Self::And(filters)
    }

    /// Combine filters with OR.
    pub fn or(filters: Vec<Filter>) -> Self {
        Self::Or(filters)
    }

    /// Negate a filter.
    #[allow(clippy::should_implement_trait)]
    pub fn not(filter: Filter) -> Self {
        Self::Not(Box::new(filter))
    }

    /// Evaluate this filter against a JSON value.
    pub fn matches_value(&self, value: &JsonValue) -> bool {
        match self {
            Filter::Eq {
                field,
                value: expected,
            } => get_field(value, field).is_some_and(|v| &v == expected),
            Filter::Ne {
                field,
                value: expected,
            } => get_field(value, field).is_none_or(|v| &v != expected),
            Filter::Gt {
                field,
                value: expected,
            } => get_field(value, field)
                .is_some_and(|v| compare_json(&v, expected) == Some(Ordering::Greater)),
            Filter::Gte {
                field,
                value: expected,
            } => get_field(value, field).is_some_and(|v| {
                matches!(
                    compare_json(&v, expected),
                    Some(Ordering::Greater | Ordering::Equal)
                )
            }),
            Filter::Lt {
                field,
                value: expected,
            } => get_field(value, field)
                .is_some_and(|v| compare_json(&v, expected) == Some(Ordering::Less)),
            Filter::Lte {
                field,
                value: expected,
            } => get_field(value, field).is_some_and(|v| {
                matches!(
                    compare_json(&v, expected),
                    Some(Ordering::Less | Ordering::Equal)
                )
            }),
            Filter::Contains {
                field,
                value: expected,
            } => get_field(value, field).is_some_and(|v| json_contains(&v, expected)),
            Filter::Exists { field } => get_field(value, field).is_some_and(|v| !v.is_null()),
            Filter::Matches { field, pattern } => get_field(value, field).is_some_and(|v| {
                if let Some(s) = v.as_str() {
                    regex::Regex::new(pattern).is_ok_and(|re| re.is_match(s))
                } else {
                    false
                }
            }),
            Filter::And(filters) => filters.iter().all(|f| f.matches_value(value)),
            Filter::Or(filters) => filters.iter().any(|f| f.matches_value(value)),
            Filter::Not(filter) => !filter.matches_value(value),
        }
    }
}

/// Aggregation operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Aggregation {
    /// Count of matching records.
    Count,
    /// Sum of a numeric field.
    Sum { field: String },
    /// Average of a numeric field.
    Avg { field: String },
    /// Minimum value of a field.
    Min { field: String },
    /// Maximum value of a field.
    Max { field: String },
    /// Collect unique values of a field.
    Distinct { field: String },
    /// Group by a field and apply sub-aggregations.
    GroupBy {
        field: String,
        aggregations: Vec<(String, Aggregation)>,
    },
}

impl Aggregation {
    /// Create a count aggregation.
    pub fn count() -> Self {
        Self::Count
    }

    /// Create a sum aggregation.
    pub fn sum(field: impl Into<String>) -> Self {
        Self::Sum {
            field: field.into(),
        }
    }

    /// Create an average aggregation.
    pub fn avg(field: impl Into<String>) -> Self {
        Self::Avg {
            field: field.into(),
        }
    }

    /// Create a minimum aggregation.
    pub fn min(field: impl Into<String>) -> Self {
        Self::Min {
            field: field.into(),
        }
    }

    /// Create a maximum aggregation.
    pub fn max(field: impl Into<String>) -> Self {
        Self::Max {
            field: field.into(),
        }
    }

    /// Create a distinct values aggregation.
    pub fn distinct(field: impl Into<String>) -> Self {
        Self::Distinct {
            field: field.into(),
        }
    }
}

/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortOrder {
    /// Ascending order (smallest first).
    Asc,
    /// Descending order (largest first).
    Desc,
}

/// Sort specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortBy {
    /// Field to sort by.
    pub field: String,
    /// Sort order.
    pub order: SortOrder,
}

impl SortBy {
    /// Create a new sort specification.
    pub fn new(field: impl Into<String>, order: SortOrder) -> Self {
        Self {
            field: field.into(),
            order,
        }
    }

    /// Sort ascending.
    pub fn asc(field: impl Into<String>) -> Self {
        Self::new(field, SortOrder::Asc)
    }

    /// Sort descending.
    pub fn desc(field: impl Into<String>) -> Self {
        Self::new(field, SortOrder::Desc)
    }
}

/// A query against KoruDelta data.
///
/// Queries can filter, project, sort, and limit results.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Query {
    /// Filter conditions.
    pub filters: Vec<Filter>,
    /// Fields to project (empty = all fields).
    pub projection: Vec<String>,
    /// Sort specifications.
    pub sort: Vec<SortBy>,
    /// Maximum number of results.
    pub limit: Option<usize>,
    /// Number of results to skip.
    pub offset: Option<usize>,
    /// Aggregation to perform.
    pub aggregation: Option<Aggregation>,
}

impl Query {
    /// Create a new empty query.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a filter condition.
    pub fn filter(mut self, filter: Filter) -> Self {
        self.filters.push(filter);
        self
    }

    /// Add multiple filter conditions (AND).
    pub fn filters(mut self, filters: Vec<Filter>) -> Self {
        self.filters.extend(filters);
        self
    }

    /// Set fields to project.
    pub fn project(mut self, fields: &[&str]) -> Self {
        self.projection = fields.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Add a sort specification.
    pub fn sort_by(mut self, field: impl Into<String>, ascending: bool) -> Self {
        self.sort.push(SortBy::new(
            field,
            if ascending {
                SortOrder::Asc
            } else {
                SortOrder::Desc
            },
        ));
        self
    }

    /// Set the maximum number of results.
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set the number of results to skip.
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Set an aggregation to perform.
    pub fn aggregate(mut self, aggregation: Aggregation) -> Self {
        self.aggregation = Some(aggregation);
        self
    }

    /// Check if a value matches all filters.
    pub fn matches(&self, value: &JsonValue) -> bool {
        self.filters.iter().all(|f| f.matches_value(value))
    }

    /// Apply projection to a value.
    pub fn apply_projection(&self, value: &JsonValue) -> JsonValue {
        if self.projection.is_empty() {
            return value.clone();
        }

        let mut result = Map::new();
        for field in &self.projection {
            if let Some(v) = get_field(value, field) {
                result.insert(field.clone(), v);
            }
        }
        JsonValue::Object(result)
    }
}

/// Result of a query execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// Matching records.
    pub records: Vec<QueryRecord>,
    /// Total count before limit/offset.
    pub total_count: usize,
    /// Aggregation result (if aggregation was requested).
    pub aggregation: Option<JsonValue>,
}

/// A single record in query results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRecord {
    /// The key of the record.
    pub key: String,
    /// The value (possibly projected).
    pub value: JsonValue,
    /// Timestamp of this version.
    pub timestamp: DateTime<Utc>,
    /// Version ID.
    pub version_id: String,
}

/// A history query for querying across versions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HistoryQuery {
    /// Base query for filtering versions.
    pub query: Query,
    /// Filter by time range (start).
    pub from_time: Option<DateTime<Utc>>,
    /// Filter by time range (end).
    pub to_time: Option<DateTime<Utc>>,
    /// Include only the latest N versions.
    pub latest: Option<usize>,
}

impl HistoryQuery {
    /// Create a new history query.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the base query.
    pub fn with_query(mut self, query: Query) -> Self {
        self.query = query;
        self
    }

    /// Filter versions from a specific time.
    pub fn from(mut self, time: DateTime<Utc>) -> Self {
        self.from_time = Some(time);
        self
    }

    /// Filter versions until a specific time.
    pub fn to(mut self, time: DateTime<Utc>) -> Self {
        self.to_time = Some(time);
        self
    }

    /// Include only the latest N versions.
    pub fn latest(mut self, n: usize) -> Self {
        self.latest = Some(n);
        self
    }

    /// Check if a history entry matches this query.
    pub fn matches_entry(&self, entry: &HistoryEntry) -> bool {
        // Check time bounds.
        if let Some(from) = self.from_time {
            if entry.timestamp < from {
                return false;
            }
        }
        if let Some(to) = self.to_time {
            if entry.timestamp > to {
                return false;
            }
        }
        // Check value filter.
        self.query.matches(&entry.value)
    }
}

/// Query executor for running queries against storage.
pub struct QueryExecutor;

impl QueryExecutor {
    /// Execute a query against a collection of values.
    pub fn execute<I>(query: &Query, items: I) -> DeltaResult<QueryResult>
    where
        I: Iterator<Item = (String, JsonValue, DateTime<Utc>, String)>,
    {
        let mut records: Vec<QueryRecord> = items
            .filter(|(_, value, _, _)| query.matches(value))
            .map(|(key, value, timestamp, version_id)| QueryRecord {
                key,
                value: query.apply_projection(&value),
                timestamp,
                version_id,
            })
            .collect();

        let total_count = records.len();

        // Apply sorting.
        if !query.sort.is_empty() {
            records.sort_by(|a, b| {
                for sort_spec in &query.sort {
                    let a_val = get_field(&a.value, &sort_spec.field);
                    let b_val = get_field(&b.value, &sort_spec.field);

                    let cmp = match (a_val, b_val) {
                        (Some(av), Some(bv)) => compare_json(&av, &bv).unwrap_or(Ordering::Equal),
                        (Some(_), None) => Ordering::Less,
                        (None, Some(_)) => Ordering::Greater,
                        (None, None) => Ordering::Equal,
                    };

                    let cmp = match sort_spec.order {
                        SortOrder::Asc => cmp,
                        SortOrder::Desc => cmp.reverse(),
                    };

                    if cmp != Ordering::Equal {
                        return cmp;
                    }
                }
                Ordering::Equal
            });
        }

        // Apply offset.
        if let Some(offset) = query.offset {
            records = records.into_iter().skip(offset).collect();
        }

        // Apply limit.
        if let Some(limit) = query.limit {
            records.truncate(limit);
        }

        // Compute aggregation.
        let aggregation = query
            .aggregation
            .as_ref()
            .map(|agg| compute_aggregation(agg, &records));

        Ok(QueryResult {
            records,
            total_count,
            aggregation,
        })
    }

    /// Execute a history query.
    pub fn execute_history(
        query: &HistoryQuery,
        history: Vec<HistoryEntry>,
    ) -> DeltaResult<Vec<HistoryEntry>> {
        let mut results: Vec<HistoryEntry> = history
            .into_iter()
            .filter(|entry| query.matches_entry(entry))
            .collect();

        // Apply sorting from base query.
        if !query.query.sort.is_empty() {
            results.sort_by(|a, b| {
                for sort_spec in &query.query.sort {
                    let a_val = get_field(&a.value, &sort_spec.field);
                    let b_val = get_field(&b.value, &sort_spec.field);

                    let cmp = match (a_val, b_val) {
                        (Some(av), Some(bv)) => compare_json(&av, &bv).unwrap_or(Ordering::Equal),
                        (Some(_), None) => Ordering::Less,
                        (None, Some(_)) => Ordering::Greater,
                        (None, None) => Ordering::Equal,
                    };

                    let cmp = match sort_spec.order {
                        SortOrder::Asc => cmp,
                        SortOrder::Desc => cmp.reverse(),
                    };

                    if cmp != Ordering::Equal {
                        return cmp;
                    }
                }
                Ordering::Equal
            });
        }

        // Apply latest filter.
        if let Some(latest) = query.latest {
            let len = results.len();
            if len > latest {
                results = results.into_iter().skip(len - latest).collect();
            }
        }

        // Apply limit from base query.
        if let Some(limit) = query.query.limit {
            results.truncate(limit);
        }

        Ok(results)
    }
}

/// Get a field from a JSON value using dot notation.
fn get_field(value: &JsonValue, field: &str) -> Option<JsonValue> {
    let mut current = value;
    for part in field.split('.') {
        match current {
            JsonValue::Object(map) => {
                current = map.get(part)?;
            }
            JsonValue::Array(arr) => {
                let index: usize = part.parse().ok()?;
                current = arr.get(index)?;
            }
            _ => return None,
        }
    }
    Some(current.clone())
}

/// Compare two JSON values.
/// Returns ordering with nulls sorting before all other values.
fn compare_json(a: &JsonValue, b: &JsonValue) -> Option<Ordering> {
    match (a, b) {
        // Null sorts before everything
        (JsonValue::Null, JsonValue::Null) => Some(Ordering::Equal),
        (JsonValue::Null, _) => Some(Ordering::Less),
        (_, JsonValue::Null) => Some(Ordering::Greater),
        
        // Same type comparisons
        (JsonValue::Number(a), JsonValue::Number(b)) => {
            let a_f = a.as_f64()?;
            let b_f = b.as_f64()?;
            a_f.partial_cmp(&b_f)
        }
        (JsonValue::String(a), JsonValue::String(b)) => Some(a.cmp(b)),
        (JsonValue::Bool(a), JsonValue::Bool(b)) => Some(a.cmp(b)),
        
        // Different types - sort by type name for deterministic order
        _ => None,
    }
}

/// Check if a JSON value contains another value.
fn json_contains(container: &JsonValue, item: &JsonValue) -> bool {
    match container {
        JsonValue::String(s) => {
            if let Some(substr) = item.as_str() {
                s.contains(substr)
            } else {
                false
            }
        }
        JsonValue::Array(arr) => arr.contains(item),
        JsonValue::Object(map) => {
            if let Some(key) = item.as_str() {
                map.contains_key(key)
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Compute an aggregation over query results.
fn compute_aggregation(agg: &Aggregation, records: &[QueryRecord]) -> JsonValue {
    match agg {
        Aggregation::Count => JsonValue::Number(records.len().into()),
        Aggregation::Sum { field } => {
            let sum: f64 = records
                .iter()
                .filter_map(|r| get_field(&r.value, field))
                .filter_map(|v| v.as_f64())
                .sum();
            serde_json::json!(sum)
        }
        Aggregation::Avg { field } => {
            let values: Vec<f64> = records
                .iter()
                .filter_map(|r| get_field(&r.value, field))
                .filter_map(|v| v.as_f64())
                .collect();
            if values.is_empty() {
                JsonValue::Null
            } else {
                let avg = values.iter().sum::<f64>() / values.len() as f64;
                serde_json::json!(avg)
            }
        }
        Aggregation::Min { field } => records
            .iter()
            .filter_map(|r| get_field(&r.value, field))
            .min_by(|a, b| compare_json(a, b).unwrap_or(Ordering::Equal))
            .unwrap_or(JsonValue::Null),
        Aggregation::Max { field } => records
            .iter()
            .filter_map(|r| get_field(&r.value, field))
            .max_by(|a, b| compare_json(a, b).unwrap_or(Ordering::Equal))
            .unwrap_or(JsonValue::Null),
        Aggregation::Distinct { field } => {
            let mut values: Vec<JsonValue> = records
                .iter()
                .filter_map(|r| get_field(&r.value, field))
                .collect();
            values.dedup();
            JsonValue::Array(values)
        }
        Aggregation::GroupBy {
            field,
            aggregations,
        } => {
            let mut groups: std::collections::HashMap<String, Vec<&QueryRecord>> =
                std::collections::HashMap::new();

            for record in records {
                let key = get_field(&record.value, field)
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "null".to_string());
                groups.entry(key).or_default().push(record);
            }

            let result: Map<String, JsonValue> = groups
                .into_iter()
                .map(|(key, group_records)| {
                    let group_records: Vec<QueryRecord> =
                        group_records.into_iter().cloned().collect();
                    let mut group_result = Map::new();
                    for (name, sub_agg) in aggregations {
                        group_result
                            .insert(name.clone(), compute_aggregation(sub_agg, &group_records));
                    }
                    (key, JsonValue::Object(group_result))
                })
                .collect();

            JsonValue::Object(result)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_filter_eq() {
        let filter = Filter::eq("name", json!("Alice"));
        assert!(filter.matches_value(&json!({"name": "Alice"})));
        assert!(!filter.matches_value(&json!({"name": "Bob"})));
    }

    #[test]
    fn test_filter_gt() {
        let filter = Filter::gt("age", json!(30));
        assert!(filter.matches_value(&json!({"age": 35})));
        assert!(!filter.matches_value(&json!({"age": 30})));
        assert!(!filter.matches_value(&json!({"age": 25})));
    }

    #[test]
    fn test_filter_contains_string() {
        let filter = Filter::contains("name", json!("ice"));
        assert!(filter.matches_value(&json!({"name": "Alice"})));
        assert!(!filter.matches_value(&json!({"name": "Bob"})));
    }

    #[test]
    fn test_filter_contains_array() {
        let filter = Filter::contains("tags", json!("admin"));
        assert!(filter.matches_value(&json!({"tags": ["admin", "user"]})));
        assert!(!filter.matches_value(&json!({"tags": ["user"]})));
    }

    #[test]
    fn test_filter_and() {
        let filter = Filter::and(vec![
            Filter::gt("age", json!(18)),
            Filter::lt("age", json!(65)),
        ]);
        assert!(filter.matches_value(&json!({"age": 30})));
        assert!(!filter.matches_value(&json!({"age": 10})));
        assert!(!filter.matches_value(&json!({"age": 70})));
    }

    #[test]
    fn test_filter_or() {
        let filter = Filter::or(vec![
            Filter::eq("status", json!("active")),
            Filter::eq("status", json!("pending")),
        ]);
        assert!(filter.matches_value(&json!({"status": "active"})));
        assert!(filter.matches_value(&json!({"status": "pending"})));
        assert!(!filter.matches_value(&json!({"status": "inactive"})));
    }

    #[test]
    fn test_filter_nested_field() {
        let filter = Filter::eq("user.name", json!("Alice"));
        assert!(filter.matches_value(&json!({"user": {"name": "Alice"}})));
        assert!(!filter.matches_value(&json!({"user": {"name": "Bob"}})));
    }

    #[test]
    fn test_query_projection() {
        let query = Query::new().project(&["name", "email"]);
        let value = json!({"name": "Alice", "email": "alice@example.com", "age": 30});
        let projected = query.apply_projection(&value);
        assert_eq!(
            projected,
            json!({"name": "Alice", "email": "alice@example.com"})
        );
    }

    #[test]
    fn test_query_execution() {
        let query = Query::new()
            .filter(Filter::gt("age", json!(25)))
            .sort_by("age", true)
            .limit(2);

        let items = vec![
            (
                "alice".to_string(),
                json!({"name": "Alice", "age": 30}),
                Utc::now(),
                "v1".to_string(),
            ),
            (
                "bob".to_string(),
                json!({"name": "Bob", "age": 20}),
                Utc::now(),
                "v2".to_string(),
            ),
            (
                "charlie".to_string(),
                json!({"name": "Charlie", "age": 35}),
                Utc::now(),
                "v3".to_string(),
            ),
            (
                "dave".to_string(),
                json!({"name": "Dave", "age": 28}),
                Utc::now(),
                "v4".to_string(),
            ),
        ];

        let result = QueryExecutor::execute(&query, items.into_iter()).unwrap();

        assert_eq!(result.total_count, 3); // Alice, Charlie, Dave
        assert_eq!(result.records.len(), 2); // Limited to 2
        assert_eq!(result.records[0].key, "dave"); // Age 28 (sorted asc)
        assert_eq!(result.records[1].key, "alice"); // Age 30
    }

    #[test]
    fn test_aggregation_count() {
        let query = Query::new().aggregate(Aggregation::count());

        let items = vec![
            (
                "a".to_string(),
                json!({"x": 1}),
                Utc::now(),
                "v1".to_string(),
            ),
            (
                "b".to_string(),
                json!({"x": 2}),
                Utc::now(),
                "v2".to_string(),
            ),
            (
                "c".to_string(),
                json!({"x": 3}),
                Utc::now(),
                "v3".to_string(),
            ),
        ];

        let result = QueryExecutor::execute(&query, items.into_iter()).unwrap();
        assert_eq!(result.aggregation, Some(json!(3)));
    }

    #[test]
    fn test_aggregation_sum() {
        let query = Query::new().aggregate(Aggregation::sum("value"));

        let items = vec![
            (
                "a".to_string(),
                json!({"value": 10}),
                Utc::now(),
                "v1".to_string(),
            ),
            (
                "b".to_string(),
                json!({"value": 20}),
                Utc::now(),
                "v2".to_string(),
            ),
            (
                "c".to_string(),
                json!({"value": 30}),
                Utc::now(),
                "v3".to_string(),
            ),
        ];

        let result = QueryExecutor::execute(&query, items.into_iter()).unwrap();
        assert_eq!(result.aggregation, Some(json!(60.0)));
    }

    #[test]
    fn test_aggregation_avg() {
        let query = Query::new().aggregate(Aggregation::avg("score"));

        let items = vec![
            (
                "a".to_string(),
                json!({"score": 80}),
                Utc::now(),
                "v1".to_string(),
            ),
            (
                "b".to_string(),
                json!({"score": 90}),
                Utc::now(),
                "v2".to_string(),
            ),
            (
                "c".to_string(),
                json!({"score": 100}),
                Utc::now(),
                "v3".to_string(),
            ),
        ];

        let result = QueryExecutor::execute(&query, items.into_iter()).unwrap();
        assert_eq!(result.aggregation, Some(json!(90.0)));
    }

    #[test]
    fn test_history_query() {
        let now = Utc::now();
        let hour_ago = now - chrono::Duration::hours(1);
        let two_hours_ago = now - chrono::Duration::hours(2);

        let query = HistoryQuery::new()
            .from(hour_ago)
            .with_query(Query::new().filter(Filter::gt("count", json!(5))));

        let history = vec![
            HistoryEntry::new(json!({"count": 3}), two_hours_ago, "v1".to_string()),
            HistoryEntry::new(json!({"count": 7}), hour_ago, "v2".to_string()),
            HistoryEntry::new(json!({"count": 10}), now, "v3".to_string()),
        ];

        let results = QueryExecutor::execute_history(&query, history).unwrap();

        // Should include v2 and v3 (from hour_ago, count > 5)
        assert_eq!(results.len(), 2);
    }
}
