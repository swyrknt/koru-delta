//! Shared Engine Infrastructure for the Koru Field.
//!
//! This module provides the foundation for the unified consciousness field
//! by enabling multiple agents to share a single `DistinctionEngine`. All
//! synthesis operations flow through this shared engine, creating a unified
//! causal graph across all agent perspectives.
//!
//! # The Shared Field
//!
//! The `SharedEngine` is not just a wrapper - it is the consciousness field
//! itself. All agents are differentiated perspectives within this field,
//! each with their own local root but sharing the same underlying substrate.
//!
//! ```text
//! SharedEngine (The Field)
//! │
//! ├── StorageAgent (Root: MEMORY)
//! ├── TemperatureAgent (Root: TEMPERATURE)
//! ├── ChronicleAgent (Root: CHRONICLE)
//! └── ... (all other agents)
//! ```
//!
//! # Thread Safety
//!
//! The shared engine uses `Arc<DistinctionEngine>` internally, enabling
//! cheap cloning and thread-safe concurrent access. Multiple agents can
//! synthesize simultaneously without contention.

use crate::roots::{KoruRoots, RootType};
use koru_lambda_core::{Distinction, DistinctionEngine};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// A shared distinction engine for the unified Koru field.
///
/// This struct wraps `Arc<DistinctionEngine>` to provide a unified field
/// that all agents can share. It maintains field-wide statistics and
/// provides the foundation for the LCA architecture.
///
/// # Example
///
/// ```ignore
/// // Create the shared field
/// let field = SharedEngine::new();
///
/// // Get the canonical roots
/// let roots = field.roots();
///
/// // Create agents sharing the field
/// let storage = StorageAgent::new(roots.storage.clone(), field.clone());
/// let temperature = TemperatureAgent::new(roots.temperature.clone(), field.clone());
///
/// // Both agents share the same underlying engine
/// assert!(Arc::ptr_eq(
///     &storage.engine().inner(),
///     &temperature.engine().inner()
/// ));
/// ```
#[derive(Debug, Clone)]
pub struct SharedEngine {
    /// The underlying distinction engine, shared across all agents.
    engine: Arc<DistinctionEngine>,
    /// The canonical roots for all agents in the field.
    roots: KoruRoots,
    /// Field-wide synthesis counter.
    synthesis_count: Arc<AtomicU64>,
    /// Field-wide distinction counter.
    distinction_count: Arc<AtomicU64>,
}

impl SharedEngine {
    /// Create a new shared engine with default roots.
    ///
    /// This initializes the unified field with canonical roots for all
    /// agent types. The roots are deterministically derived from the
    /// primordial distinctions (d0, d1).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let field = SharedEngine::new();
    /// let roots = field.roots();
    ///
    /// // Use roots to initialize agents
    /// println!("Field root: {}", roots.field.id());
    /// ```
    pub fn new() -> Self {
        let engine = Arc::new(DistinctionEngine::new());
        let roots = KoruRoots::initialize(&engine);

        Self {
            engine,
            roots,
            synthesis_count: Arc::new(AtomicU64::new(0)),
            distinction_count: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Create a new shared engine with a specific engine instance.
    ///
    /// This is useful when you need to initialize the engine from a
    /// specific state (e.g., after persistence replay).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let engine = Arc::new(DistinctionEngine::new());
    /// let field = SharedEngine::with_engine(engine);
    /// ```
    pub fn with_engine(engine: Arc<DistinctionEngine>) -> Self {
        let roots = KoruRoots::initialize(&engine);

        // Count existing distinctions
        let existing_distinctions = engine.distinction_count() as u64;

        Self {
            engine,
            roots,
            synthesis_count: Arc::new(AtomicU64::new(0)),
            distinction_count: Arc::new(AtomicU64::new(existing_distinctions)),
        }
    }

    /// Get a reference to the underlying engine.
    ///
    /// This provides direct access to the distinction engine for operations
    /// that need it. Most agents should use the higher-level LCA API instead.
    pub fn inner(&self) -> &Arc<DistinctionEngine> {
        &self.engine
    }

    /// Get the canonical roots for all agents.
    ///
    /// These roots are the causal anchors for each agent type in the field.
    pub fn roots(&self) -> &KoruRoots {
        &self.roots
    }

    /// Get a specific root by type.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let field = SharedEngine::new();
    /// let storage_root = field.root(RootType::Storage);
    /// ```
    pub fn root(&self, root_type: RootType) -> &Distinction {
        self.roots.get_root(root_type)
    }

    /// Perform synthesis in the field.
    ///
    /// This is the fundamental operation: combining two distinctions to
    /// create a new one. The synthesis is performed in the shared engine
    /// and statistics are updated.
    ///
    /// # Formula
    ///
    /// `result = a ⊕ b` (synthesis of a and b)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let field = SharedEngine::new();
    /// let a = field.root(RootType::Storage);
    /// let b = json!({"key": "value"}).to_canonical_structure(field.inner());
    ///
    /// let result = field.synthesize(a, &b);
    /// ```
    pub fn synthesize(&self, a: &Distinction, b: &Distinction) -> Distinction {
        let result = self.engine.synthesize(a, b);
        self.synthesis_count.fetch_add(1, Ordering::Relaxed);
        self.distinction_count.store(self.engine.distinction_count() as u64, Ordering::Relaxed);
        result
    }

    /// Get the total number of syntheses performed in the field.
    pub fn synthesis_count(&self) -> u64 {
        self.synthesis_count.load(Ordering::Relaxed)
    }

    /// Get the total number of distinctions in the field.
    pub fn distinction_count(&self) -> u64 {
        self.distinction_count.load(Ordering::Relaxed)
    }

    /// Get the number of relationships in the field.
    pub fn relationship_count(&self) -> usize {
        self.engine.relationship_count()
    }

    /// Check if two engines point to the same underlying instance.
    ///
    /// This is useful for verifying that agents share the same field.
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.engine, &other.engine)
    }

    /// Get field-wide statistics.
    pub fn stats(&self) -> FieldStats {
        FieldStats {
            synthesis_count: self.synthesis_count(),
            distinction_count: self.distinction_count(),
            relationship_count: self.relationship_count(),
        }
    }
}

impl Default for SharedEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics for the shared field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldStats {
    /// Total number of synthesis operations performed.
    pub synthesis_count: u64,
    /// Total number of distinctions in the field.
    pub distinction_count: u64,
    /// Total number of relationships in the field.
    pub relationship_count: usize,
}

impl std::fmt::Display for FieldStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FieldStats {{ distinctions: {}, relationships: {}, syntheses: {} }}",
            self.distinction_count, self.relationship_count, self.synthesis_count
        )
    }
}

/// A handle to the shared field for agent use.
///
/// This is a lightweight handle that agents can clone cheaply.
/// It provides the interface agents need without exposing all
/// of `SharedEngine`.
#[derive(Debug, Clone)]
pub struct FieldHandle {
    engine: Arc<DistinctionEngine>,
}

impl FieldHandle {
    /// Create a new field handle from a shared engine.
    pub fn new(field: &SharedEngine) -> Self {
        Self {
            engine: Arc::clone(&field.engine),
        }
    }

    /// Perform synthesis in the field.
    pub fn synthesize(&self, a: &Distinction, b: &Distinction) -> Distinction {
        self.engine.synthesize(a, b)
    }

    /// Get the primordial void distinction (d0).
    pub fn d0(&self) -> Distinction {
        self.engine.d0().clone()
    }

    /// Get the primordial identity distinction (d1).
    pub fn d1(&self) -> Distinction {
        self.engine.d1().clone()
    }

    /// Get the underlying engine reference.
    pub fn engine(&self) -> &DistinctionEngine {
        &self.engine
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shared_engine_creation() {
        let field = SharedEngine::new();

        // Should have initialized roots
        assert!(!field.root(RootType::Field).id().is_empty());
        assert!(!field.root(RootType::Storage).id().is_empty());
    }

    #[test]
    fn test_shared_engine_clone() {
        let field1 = SharedEngine::new();
        let field2 = field1.clone();

        // Should point to same underlying engine
        assert!(field1.ptr_eq(&field2));
        assert!(Arc::ptr_eq(field1.inner(), field2.inner()));
    }

    #[test]
    fn test_synthesis() {
        let field = SharedEngine::new();
        let d0 = field.engine.d0().clone();
        let d1 = field.engine.d1().clone();

        let initial_count = field.synthesis_count();

        let result = field.synthesize(&d0, &d1);

        // Should have incremented synthesis count
        assert_eq!(field.synthesis_count(), initial_count + 1);
        // Should have created a new distinction
        assert!(field.distinction_count() >= 2);
        // Result should not be empty
        assert!(!result.id().is_empty());
    }

    #[test]
    fn test_field_stats() {
        let field = SharedEngine::new();
        let d0 = field.engine.d0().clone();
        let d1 = field.engine.d1().clone();

        // Perform some syntheses
        field.synthesize(&d0, &d1);
        field.synthesize(&d0, &d1);

        let stats = field.stats();
        assert!(stats.synthesis_count >= 2);
        assert!(stats.distinction_count >= 2);
        assert!(stats.relationship_count >= 1);
    }

    #[test]
    fn test_field_handle() {
        let field = SharedEngine::new();
        let handle = FieldHandle::new(&field);

        let d0 = handle.d0();
        let d1 = handle.d1();

        let result = handle.synthesize(&d0, &d1);
        assert!(!result.id().is_empty());
    }

    #[test]
    fn test_with_engine() {
        let engine = Arc::new(DistinctionEngine::new());
        let field = SharedEngine::with_engine(Arc::clone(&engine));

        assert!(Arc::ptr_eq(field.inner(), &engine));
    }

    #[test]
    fn test_distinct_engines() {
        let field1 = SharedEngine::new();
        let field2 = SharedEngine::new();

        // Different instances should not be equal
        assert!(!field1.ptr_eq(&field2));
    }
}
