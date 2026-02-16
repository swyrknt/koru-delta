/// Subscriptions for real-time change notifications in KoruDelta.
///
/// This module provides a publish-subscribe system for receiving notifications
/// when data changes. Subscriptions can be:
///
/// - **Collection-level**: Get notified of any change in a collection
/// - **Key-level**: Get notified when a specific key changes
/// - **Filter-based**: Get notified when changes match a filter
///
/// ## LCA Architecture
///
/// SubscriptionAgent implements `LocalCausalAgent`, making all subscription operations
/// causal distinctions. The formula: `ΔNew = ΔLocal_Root ⊕ ΔAction_Data`
///
/// # Example
///
/// ```ignore
/// use koru_delta::subscriptions::{SubscriptionAgent, Subscription};
/// use koru_delta::query::Filter;
///
/// let agent = SubscriptionAgent::new();
///
/// // Subscribe to all changes in "users" collection
/// let sub_id = agent.subscribe(Subscription::collection("users"));
///
/// // Get the receiver for this subscription
/// let mut rx = agent.receiver(sub_id).unwrap();
///
/// // In an async context:
/// while let Some(event) = rx.recv().await {
///     println!("Change: {:?}", event);
/// }
/// ```
use crate::actions::SubscriptionAction;
use crate::engine::SharedEngine;
use crate::error::{DeltaError, DeltaResult};
use crate::query::Filter;
use crate::roots::KoruRoots;
#[cfg(test)]
use crate::types::VectorClock;
use crate::types::VersionedValue;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use koru_lambda_core::{Canonicalizable, Distinction, DistinctionEngine, LocalCausalAgent};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::broadcast;

/// Default channel capacity for subscription broadcasts.
const DEFAULT_CHANNEL_CAPACITY: usize = 256;

/// Unique identifier for a subscription.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SubscriptionId(pub u64);

impl std::fmt::Display for SubscriptionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "sub-{}", self.0)
    }
}

/// Type of change that occurred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    /// A new value was inserted.
    Insert,
    /// An existing value was updated.
    Update,
    /// A value was deleted.
    Delete,
}

/// A change event notification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChangeEvent {
    /// Type of change.
    pub change_type: ChangeType,
    /// The collection/namespace affected.
    pub collection: String,
    /// The key affected.
    pub key: String,
    /// The new value (None for deletes).
    pub value: Option<JsonValue>,
    /// Previous value (None for inserts).
    pub previous_value: Option<JsonValue>,
    /// Timestamp of the change.
    pub timestamp: DateTime<Utc>,
    /// Version ID of the new value.
    pub version_id: Option<String>,
    /// Previous version ID.
    pub previous_version_id: Option<String>,
}

impl ChangeEvent {
    /// Create an insert event.
    pub fn insert(
        collection: impl Into<String>,
        key: impl Into<String>,
        value: &VersionedValue,
    ) -> Self {
        Self {
            change_type: ChangeType::Insert,
            collection: collection.into(),
            key: key.into(),
            value: Some(value.value().clone()),
            previous_value: None,
            timestamp: value.timestamp(),
            version_id: Some(value.version_id().to_string()),
            previous_version_id: None,
        }
    }

    /// Create an update event.
    pub fn update(
        collection: impl Into<String>,
        key: impl Into<String>,
        value: &VersionedValue,
        previous: &VersionedValue,
    ) -> Self {
        Self {
            change_type: ChangeType::Update,
            collection: collection.into(),
            key: key.into(),
            value: Some(value.value().clone()),
            previous_value: Some(previous.value().clone()),
            timestamp: value.timestamp(),
            version_id: Some(value.version_id().to_string()),
            previous_version_id: Some(previous.version_id().to_string()),
        }
    }

    /// Create a delete event.
    pub fn delete(
        collection: impl Into<String>,
        key: impl Into<String>,
        previous: &VersionedValue,
    ) -> Self {
        Self {
            change_type: ChangeType::Delete,
            collection: collection.into(),
            key: key.into(),
            value: None,
            previous_value: Some(previous.value().clone()),
            timestamp: Utc::now(),
            version_id: None,
            previous_version_id: Some(previous.version_id().to_string()),
        }
    }
}

/// A subscription definition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Subscription {
    /// Optional collection filter (None = all collections).
    pub collection: Option<String>,
    /// Optional key filter (None = all keys).
    pub key: Option<String>,
    /// Optional value filter.
    pub filter: Option<Filter>,
    /// Types of changes to receive.
    pub change_types: Vec<ChangeType>,
    /// Human-readable name for this subscription.
    pub name: Option<String>,
}

impl Subscription {
    /// Create a subscription that matches all changes.
    pub fn all() -> Self {
        Self {
            collection: None,
            key: None,
            filter: None,
            change_types: vec![ChangeType::Insert, ChangeType::Update, ChangeType::Delete],
            name: None,
        }
    }

    /// Create a subscription for a specific collection.
    pub fn collection(collection: impl Into<String>) -> Self {
        Self {
            collection: Some(collection.into()),
            key: None,
            filter: None,
            change_types: vec![ChangeType::Insert, ChangeType::Update, ChangeType::Delete],
            name: None,
        }
    }

    /// Create a subscription for a specific key.
    pub fn key(collection: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            collection: Some(collection.into()),
            key: Some(key.into()),
            filter: None,
            change_types: vec![ChangeType::Insert, ChangeType::Update, ChangeType::Delete],
            name: None,
        }
    }

    /// Add a value filter.
    pub fn with_filter(mut self, filter: Filter) -> Self {
        self.filter = Some(filter);
        self
    }

    /// Set specific change types to subscribe to.
    pub fn with_change_types(mut self, types: Vec<ChangeType>) -> Self {
        self.change_types = types;
        self
    }

    /// Only subscribe to inserts.
    pub fn inserts_only(mut self) -> Self {
        self.change_types = vec![ChangeType::Insert];
        self
    }

    /// Only subscribe to updates.
    pub fn updates_only(mut self) -> Self {
        self.change_types = vec![ChangeType::Update];
        self
    }

    /// Only subscribe to deletes.
    pub fn deletes_only(mut self) -> Self {
        self.change_types = vec![ChangeType::Delete];
        self
    }

    /// Set a name for this subscription.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Check if this subscription matches a change event.
    pub fn matches(&self, event: &ChangeEvent) -> bool {
        // Check change type.
        if !self.change_types.contains(&event.change_type) {
            return false;
        }

        // Check collection.
        if let Some(ref collection) = self.collection {
            if &event.collection != collection {
                return false;
            }
        }

        // Check key.
        if let Some(ref key) = self.key {
            if &event.key != key {
                return false;
            }
        }

        // Check filter against new value (or previous for deletes).
        if let Some(ref filter) = self.filter {
            let value_to_check = event.value.as_ref().or(event.previous_value.as_ref());
            if let Some(value) = value_to_check {
                if !filter.matches_value(value) {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }
}

/// Information about an active subscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionInfo {
    /// The subscription ID.
    pub id: SubscriptionId,
    /// The subscription definition.
    pub subscription: Subscription,
    /// When this subscription was created.
    pub created_at: DateTime<Utc>,
    /// Number of events delivered.
    pub events_delivered: u64,
}

/// Internal subscription state.
#[derive(Debug)]
struct SubscriptionState {
    subscription: Subscription,
    sender: broadcast::Sender<ChangeEvent>,
    created_at: DateTime<Utc>,
    events_delivered: AtomicU64,
}

/// Subscription agent implementing LocalCausalAgent trait.
///
/// Follows the LCA formula: `ΔNew = ΔLocal_Root ⊕ ΔAction_Data`
/// All subscription operations are causal distinctions synthesized from the subscription root.
#[derive(Debug)]
pub struct SubscriptionAgent {
    /// LCA: Local root distinction (Root: SUBSCRIPTION)
    local_root: Distinction,

    /// LCA: Handle to the unified field
    _field: SharedEngine,

    /// The underlying distinction engine for content addressing
    engine: Arc<DistinctionEngine>,

    subscriptions: DashMap<SubscriptionId, SubscriptionState>,
    next_id: AtomicU64,
    channel_capacity: usize,
}

impl SubscriptionAgent {
    /// Create a new subscription agent with LCA pattern.
    ///
    /// The agent initializes from the subscription canonical root,
    /// establishing its causal anchor in the field.
    pub fn new(field: &SharedEngine) -> Self {
        Self::with_capacity(field, DEFAULT_CHANNEL_CAPACITY)
    }

    /// Create a new subscription agent with custom channel capacity.
    pub fn with_capacity(field: &SharedEngine, capacity: usize) -> Self {
        let engine = Arc::clone(field.inner());
        let roots = KoruRoots::initialize(&engine);
        let local_root = roots.subscription.clone();

        Self {
            local_root,
            _field: field.clone(),
            engine,
            subscriptions: DashMap::new(),
            next_id: AtomicU64::new(1),
            channel_capacity: capacity,
        }
    }

    /// Get the local root distinction.
    pub fn local_root(&self) -> &Distinction {
        &self.local_root
    }

    /// Apply a subscription action, synthesizing new state.
    ///
    /// This is the primary interface for subscription operations following
    /// the LCA formula: `ΔNew = ΔLocal_Root ⊕ ΔAction_Data`
    pub fn apply_action(&mut self, action: SubscriptionAction) -> Distinction {
        let engine = Arc::clone(&self.engine);
        let new_root = self.synthesize_action(action, &engine);
        self.local_root = new_root.clone();
        new_root
    }

    /// Subscribe to changes.
    ///
    /// Returns the subscription ID and a receiver for events.
    pub fn subscribe(
        &self,
        subscription: Subscription,
    ) -> (SubscriptionId, broadcast::Receiver<ChangeEvent>) {
        let id = SubscriptionId(self.next_id.fetch_add(1, Ordering::SeqCst));
        let (sender, receiver) = broadcast::channel(self.channel_capacity);

        let state = SubscriptionState {
            subscription,
            sender,
            created_at: Utc::now(),
            events_delivered: AtomicU64::new(0),
        };

        self.subscriptions.insert(id, state);

        (id, receiver)
    }

    /// Get a new receiver for an existing subscription.
    ///
    /// This allows multiple consumers to receive the same events.
    pub fn receiver(&self, id: SubscriptionId) -> Option<broadcast::Receiver<ChangeEvent>> {
        self.subscriptions
            .get(&id)
            .map(|state| state.sender.subscribe())
    }

    /// Unsubscribe from changes.
    pub fn unsubscribe(&self, id: SubscriptionId) -> DeltaResult<()> {
        self.subscriptions
            .remove(&id)
            .ok_or_else(|| DeltaError::StorageError(format!("Subscription {} not found", id)))?;
        Ok(())
    }

    /// Get information about a subscription.
    pub fn get_subscription(&self, id: SubscriptionId) -> Option<SubscriptionInfo> {
        self.subscriptions.get(&id).map(|state| SubscriptionInfo {
            id,
            subscription: state.subscription.clone(),
            created_at: state.created_at,
            events_delivered: state.events_delivered.load(Ordering::Relaxed),
        })
    }

    /// List all active subscriptions.
    pub fn list_subscriptions(&self) -> Vec<SubscriptionInfo> {
        self.subscriptions
            .iter()
            .map(|entry| SubscriptionInfo {
                id: *entry.key(),
                subscription: entry.value().subscription.clone(),
                created_at: entry.value().created_at,
                events_delivered: entry.value().events_delivered.load(Ordering::Relaxed),
            })
            .collect()
    }

    /// Get the number of active subscriptions.
    pub fn subscription_count(&self) -> usize {
        self.subscriptions.len()
    }

    /// Notify subscribers of a change.
    ///
    /// This is called by the storage layer when data changes.
    pub fn notify(&self, event: ChangeEvent) {
        for entry in self.subscriptions.iter() {
            let state = entry.value();
            if state.subscription.matches(&event) {
                // Try to send, ignoring errors (receiver may have dropped).
                if state.sender.send(event.clone()).is_ok() {
                    state.events_delivered.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }

    /// Notify subscribers of an insert.
    pub fn notify_insert(
        &self,
        collection: impl Into<String>,
        key: impl Into<String>,
        value: &VersionedValue,
    ) {
        let event = ChangeEvent::insert(collection, key, value);
        self.notify(event);
    }

    /// Notify subscribers of an update.
    pub fn notify_update(
        &self,
        collection: impl Into<String>,
        key: impl Into<String>,
        value: &VersionedValue,
        previous: &VersionedValue,
    ) {
        let event = ChangeEvent::update(collection, key, value, previous);
        self.notify(event);
    }

    /// Notify subscribers of a delete.
    pub fn notify_delete(
        &self,
        collection: impl Into<String>,
        key: impl Into<String>,
        previous: &VersionedValue,
    ) {
        let event = ChangeEvent::delete(collection, key, previous);
        self.notify(event);
    }

    /// Subscribe with synthesis.
    pub fn subscribe_synthesized(
        &mut self,
        subscription: Subscription,
    ) -> (
        SubscriptionId,
        broadcast::Receiver<ChangeEvent>,
        Distinction,
    ) {
        let action = SubscriptionAction::Subscribe {
            subscription: subscription.clone(),
        };
        let new_root = self.apply_action(action);

        let (id, receiver) = self.subscribe(subscription);
        (id, receiver, new_root)
    }

    /// Unsubscribe with synthesis.
    pub fn unsubscribe_synthesized(
        &mut self,
        id: SubscriptionId,
    ) -> (DeltaResult<()>, Distinction) {
        let action = SubscriptionAction::Unsubscribe {
            subscription_id: id,
        };
        let new_root = self.apply_action(action);

        let result = self.unsubscribe(id);
        (result, new_root)
    }

    /// Notify with synthesis.
    pub fn notify_synthesized(&mut self, event: ChangeEvent) -> Distinction {
        let action = SubscriptionAction::Notify {
            event: event.clone(),
        };
        let new_root = self.apply_action(action);

        self.notify(event);
        new_root
    }
}

// LCA Trait Implementation
impl LocalCausalAgent for SubscriptionAgent {
    type ActionData = SubscriptionAction;

    fn get_current_root(&self) -> &Distinction {
        &self.local_root
    }

    fn update_local_root(&mut self, new_root: Distinction) {
        self.local_root = new_root;
    }

    fn synthesize_action(
        &mut self,
        action: SubscriptionAction,
        engine: &Arc<DistinctionEngine>,
    ) -> Distinction {
        // Canonical LCA pattern: ΔNew = ΔLocal_Root ⊕ ΔAction
        let action_distinction = action.to_canonical_structure(engine);
        let new_root = engine.synthesize(&self.local_root, &action_distinction);
        self.local_root = new_root.clone();
        new_root
    }
}

impl Default for SubscriptionAgent {
    fn default() -> Self {
        let field = SharedEngine::new();
        Self::new(&field)
    }
}

/// A wrapper that combines storage and subscription management.
///
/// This provides convenient methods for writing data and automatically
/// notifying subscribers.
pub struct SubscribableStorage {
    storage: Arc<crate::storage::CausalStorage>,
    subscriptions: Arc<SubscriptionAgent>,
}

impl SubscribableStorage {
    /// Create a new subscribable storage wrapper.
    pub fn new(
        storage: Arc<crate::storage::CausalStorage>,
        subscriptions: Arc<SubscriptionAgent>,
    ) -> Self {
        Self {
            storage,
            subscriptions,
        }
    }

    /// Get the underlying storage.
    pub fn storage(&self) -> &Arc<crate::storage::CausalStorage> {
        &self.storage
    }

    /// Get the subscription manager.
    pub fn subscriptions(&self) -> &Arc<SubscriptionAgent> {
        &self.subscriptions
    }

    /// Put a value and notify subscribers.
    pub fn put(
        &self,
        namespace: impl Into<String> + Clone,
        key: impl Into<String> + Clone,
        value: JsonValue,
    ) -> DeltaResult<VersionedValue> {
        let ns = namespace.clone().into();
        let k = key.clone().into();

        // Check if this is an update.
        let previous = self.storage.get(&ns, &k).ok();

        // Perform the write.
        let result = self.storage.put(namespace, key, value)?;

        // Notify subscribers.
        if let Some(prev) = previous {
            self.subscriptions.notify_update(&ns, &k, &result, &prev);
        } else {
            self.subscriptions.notify_insert(&ns, &k, &result);
        }

        Ok(result)
    }

    /// Get a value.
    pub fn get(
        &self,
        namespace: impl Into<String>,
        key: impl Into<String>,
    ) -> DeltaResult<VersionedValue> {
        self.storage.get(namespace, key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::Filter;
    use serde_json::json;
    use std::time::Duration;

    fn create_test_value(value: JsonValue) -> VersionedValue {
        VersionedValue::from_json(
            value,
            Utc::now(),
            "test-version".to_string(), // write_id
            "test-version".to_string(), // distinction_id
            None,
            VectorClock::new(),
        )
    }

    #[test]
    fn test_subscription_all() {
        let sub = Subscription::all();

        let event = ChangeEvent::insert(
            "users",
            "alice",
            &create_test_value(json!({"name": "Alice"})),
        );
        assert!(sub.matches(&event));

        let event2 = ChangeEvent::insert(
            "products",
            "widget",
            &create_test_value(json!({"price": 10})),
        );
        assert!(sub.matches(&event2));
    }

    #[test]
    fn test_subscription_collection() {
        let sub = Subscription::collection("users");

        let event = ChangeEvent::insert(
            "users",
            "alice",
            &create_test_value(json!({"name": "Alice"})),
        );
        assert!(sub.matches(&event));

        let event2 = ChangeEvent::insert(
            "products",
            "widget",
            &create_test_value(json!({"price": 10})),
        );
        assert!(!sub.matches(&event2));
    }

    #[test]
    fn test_subscription_key() {
        let sub = Subscription::key("users", "alice");

        let event = ChangeEvent::insert(
            "users",
            "alice",
            &create_test_value(json!({"name": "Alice"})),
        );
        assert!(sub.matches(&event));

        let event2 =
            ChangeEvent::insert("users", "bob", &create_test_value(json!({"name": "Bob"})));
        assert!(!sub.matches(&event2));
    }

    #[test]
    fn test_subscription_with_filter() {
        let sub = Subscription::collection("users").with_filter(Filter::gt("age", json!(18)));

        let event = ChangeEvent::insert(
            "users",
            "alice",
            &create_test_value(json!({"name": "Alice", "age": 25})),
        );
        assert!(sub.matches(&event));

        let event2 = ChangeEvent::insert(
            "users",
            "bob",
            &create_test_value(json!({"name": "Bob", "age": 15})),
        );
        assert!(!sub.matches(&event2));
    }

    #[test]
    fn test_subscription_change_types() {
        let sub = Subscription::collection("users").inserts_only();

        let value = create_test_value(json!({"name": "Alice"}));

        let insert = ChangeEvent::insert("users", "alice", &value);
        assert!(sub.matches(&insert));

        let update = ChangeEvent::update("users", "alice", &value, &value);
        assert!(!sub.matches(&update));
    }

    #[tokio::test]
    async fn test_subscription_manager_basic() {
        use crate::engine::SharedEngine;
        let field = SharedEngine::new();
        let manager = SubscriptionAgent::new(&field);

        let (id, mut rx) = manager.subscribe(Subscription::collection("users"));

        // Notify of a change.
        let value = create_test_value(json!({"name": "Alice"}));
        manager.notify_insert("users", "alice", &value);

        // Should receive the event.
        tokio::select! {
            event = rx.recv() => {
                let event = event.unwrap();
                assert_eq!(event.collection, "users");
                assert_eq!(event.key, "alice");
                assert_eq!(event.change_type, ChangeType::Insert);
            }
            _ = tokio::time::sleep(Duration::from_millis(100)) => {
                panic!("Should have received event");
            }
        }

        // Unsubscribe.
        manager.unsubscribe(id).unwrap();
        assert_eq!(manager.subscription_count(), 0);
    }

    #[tokio::test]
    async fn test_subscription_filtering() {
        use crate::engine::SharedEngine;
        let field = SharedEngine::new();
        let manager = SubscriptionAgent::new(&field);

        // Subscribe only to users collection.
        let (_id, mut rx) = manager.subscribe(Subscription::collection("users"));

        // Notify of a change to products (should not receive).
        let value = create_test_value(json!({"price": 10}));
        manager.notify_insert("products", "widget", &value);

        // Notify of a change to users (should receive).
        let user_value = create_test_value(json!({"name": "Alice"}));
        manager.notify_insert("users", "alice", &user_value);

        // Should receive only the users event.
        tokio::select! {
            event = rx.recv() => {
                let event = event.unwrap();
                assert_eq!(event.collection, "users");
            }
            _ = tokio::time::sleep(Duration::from_millis(100)) => {
                panic!("Should have received event");
            }
        }
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        use crate::engine::SharedEngine;
        let field = SharedEngine::new();
        let manager = SubscriptionAgent::new(&field);

        let (_id1, mut rx1) = manager.subscribe(Subscription::all());
        let (_id2, mut rx2) = manager.subscribe(Subscription::all());

        let value = create_test_value(json!({"data": "test"}));
        manager.notify_insert("test", "key", &value);

        // Both should receive.
        let e1 = rx1.try_recv().unwrap();
        let e2 = rx2.try_recv().unwrap();

        assert_eq!(e1.key, "key");
        assert_eq!(e2.key, "key");
    }

    #[test]
    fn test_subscription_info() {
        use crate::engine::SharedEngine;
        let field = SharedEngine::new();
        let manager = SubscriptionAgent::new(&field);

        let (id, _rx) =
            manager.subscribe(Subscription::collection("users").with_name("user_watcher"));

        let info = manager.get_subscription(id).unwrap();
        assert_eq!(info.id, id);
        assert_eq!(info.subscription.name, Some("user_watcher".to_string()));
        assert_eq!(info.events_delivered, 0);
    }

    #[test]
    fn test_subscription_list() {
        use crate::engine::SharedEngine;
        let field = SharedEngine::new();
        let manager = SubscriptionAgent::new(&field);

        let (_id1, _rx1) = manager.subscribe(Subscription::collection("users"));
        let (_id2, _rx2) = manager.subscribe(Subscription::collection("products"));
        let (_id3, _rx3) = manager.subscribe(Subscription::all());

        let subs = manager.list_subscriptions();
        assert_eq!(subs.len(), 3);
    }

    #[test]
    fn test_events_delivered_counter() {
        use crate::engine::SharedEngine;
        let field = SharedEngine::new();
        let manager = SubscriptionAgent::new(&field);

        let (id, _rx) = manager.subscribe(Subscription::all());

        // Send some events.
        for i in 0..5 {
            let value = create_test_value(json!({"count": i}));
            manager.notify_insert("test", format!("key{}", i), &value);
        }

        let info = manager.get_subscription(id).unwrap();
        assert_eq!(info.events_delivered, 5);
    }

    #[tokio::test]
    async fn test_subscribable_storage() {
        use crate::engine::SharedEngine;
        use crate::storage::CausalStorage;
        use koru_lambda_core::DistinctionEngine;

        let engine = Arc::new(DistinctionEngine::new());
        let storage = Arc::new(CausalStorage::new(engine));
        let field = SharedEngine::new();
        let subs = Arc::new(SubscriptionAgent::new(&field));

        let subscribable = SubscribableStorage::new(storage, subs.clone());

        let (_id, mut rx) = subs.subscribe(Subscription::collection("users"));

        // Write through subscribable storage.
        subscribable
            .put("users", "alice", json!({"name": "Alice"}))
            .unwrap();

        // Should receive notification.
        let event = rx.try_recv().unwrap();
        assert_eq!(event.collection, "users");
        assert_eq!(event.key, "alice");
        assert_eq!(event.change_type, ChangeType::Insert);

        // Update the same key.
        subscribable
            .put("users", "alice", json!({"name": "Alice", "age": 30}))
            .unwrap();

        let event = rx.try_recv().unwrap();
        assert_eq!(event.change_type, ChangeType::Update);
        assert!(event.previous_value.is_some());
    }

    // LCA Tests
    mod lca_tests {
        use super::*;
        use koru_lambda_core::LocalCausalAgent;

        fn setup_agent() -> SubscriptionAgent {
            let field = SharedEngine::new();
            SubscriptionAgent::new(&field)
        }

        #[test]
        fn test_subscription_agent_implements_lca_trait() {
            let agent = setup_agent();

            // Verify trait is implemented
            let _root = agent.get_current_root();
        }

        #[test]
        fn test_subscription_agent_has_unique_local_root() {
            let field = SharedEngine::new();
            let agent1 = SubscriptionAgent::new(&field);
            let agent2 = SubscriptionAgent::new(&field);

            // Each agent should have the same subscription root from canonical roots
            assert_eq!(
                agent1.local_root().id(),
                agent2.local_root().id(),
                "Subscription agents share the same canonical root"
            );
        }

        #[test]
        fn test_subscribe_synthesizes() {
            let mut agent = setup_agent();
            let root_before = agent.local_root().id().to_string();

            let (_id, _receiver, new_root) =
                agent.subscribe_synthesized(Subscription::collection("users"));

            let root_after = agent.local_root().id().to_string();
            assert_ne!(
                root_before, root_after,
                "Local root should change after synthesis"
            );
            assert_eq!(new_root.id(), root_after);
        }

        #[test]
        fn test_unsubscribe_synthesizes() {
            let mut agent = setup_agent();

            // First subscribe
            let (id, _receiver, _) = agent.subscribe_synthesized(Subscription::collection("users"));

            let root_before = agent.local_root().id().to_string();

            let (result, new_root) = agent.unsubscribe_synthesized(id);
            assert!(result.is_ok());

            let root_after = agent.local_root().id().to_string();
            assert_ne!(
                root_before, root_after,
                "Local root should change after unsubscribe synthesis"
            );
            assert_eq!(new_root.id(), root_after);
        }

        #[test]
        fn test_notify_synthesizes() {
            let mut agent = setup_agent();

            // Subscribe first
            agent.subscribe(Subscription::collection("users"));

            let root_before = agent.local_root().id().to_string();

            let event = ChangeEvent::insert(
                "users",
                "alice",
                &create_test_value(json!({"name": "Alice"})),
            );
            let new_root = agent.notify_synthesized(event);

            let root_after = agent.local_root().id().to_string();
            assert_ne!(
                root_before, root_after,
                "Local root should change after notify synthesis"
            );
            assert_eq!(new_root.id(), root_after);
        }

        #[test]
        fn test_apply_action_changes_root() {
            let mut agent = setup_agent();
            let root_before = agent.local_root().id().to_string();

            let action = SubscriptionAction::ListSubscriptions;
            let new_root = agent.apply_action(action);

            let root_after = agent.local_root().id().to_string();
            assert_ne!(
                root_before, root_after,
                "Local root should change after apply_action"
            );
            assert_eq!(new_root.id(), root_after);
        }
    }
}
