//! Agent Orchestrator - Central coordination for all LCA agents.
//!
//! The orchestrator manages the lifecycle of all agents in the KoruDelta system,
//! coordinates their access to the shared distinction engine, and manages
//! rhythmic coordination cycles (pulses) for external integration.
//!
//! # LCA Architecture
//!
//! The orchestrator itself follows the LCA pattern:
//! - It has a local root (RootType::Orchestrator)
//! - All coordination actions are synthesized distinctions
//! - Agent registration/unregistration produces distinctions
//!
//! # Agent Registry
//!
//! All agents register with the orchestrator to:
//! - Receive shared engine access
//! - Participate in coordination cycles
//! - Announce their capabilities
//!
//! # Pulse Coordination
//!
//! The orchestrator can coordinate rhythmic cycles for external systems.
//! These cycles (called "pulses") allow external agents to synchronize
//! their operations with KoruDelta's internal state.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

use koru_lambda_core::{Canonicalizable, Distinction};

use crate::actions::{KoruAction, PulseAction};
use crate::engine::{FieldHandle, SharedEngine};
use crate::roots::RootType;

/// The central orchestrator for all LCA agents.
///
/// The orchestrator maintains:
/// - The shared distinction engine (unified field)
/// - The canonical roots (all agent starting points)
/// - The agent registry (all registered agents)
/// - The pulse coordinator (rhythmic external coordination)
///
/// # Example
///
/// ```rust,ignore
/// let orchestrator = KoruOrchestrator::new();
///
/// // Register an agent
/// orchestrator.register_agent::<MyAgent>("my_agent");
///
/// // Trigger a coordination pulse
/// orchestrator.pulse(CoordinationPhase::Consolidation);
/// ```
pub struct KoruOrchestrator {
    /// The shared distinction engine (unified field)
    engine: SharedEngine,

    /// Handle to the field for synthesis operations
    field: FieldHandle,

    /// LCA: Local root distinction (Root: ORCHESTRATOR)
    local_root: RwLock<Distinction>,

    /// Registry of all agents
    agents: RwLock<AgentRegistry>,

    /// Pulse coordinator for external coordination
    pulse: PulseCoordinator,

    /// Statistics
    agents_registered: AtomicU64,
    pulses_triggered: AtomicU64,
}

/// Information about a registered agent.
#[derive(Debug, Clone)]
pub struct AgentInfo {
    /// Unique identifier for the agent
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// The agent's local root distinction
    pub root: Distinction,

    /// Agent type identifier
    pub agent_type: String,

    /// Capabilities this agent provides
    pub capabilities: Vec<AgentCapability>,
}

/// Capabilities an agent can provide.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AgentCapability {
    /// Storage operations
    Storage,
    /// Memory tier management
    MemoryTiering,
    /// Authentication/identity
    Identity,
    /// Network/distributed operations
    Network,
    /// Query processing
    Query,
    /// View maintenance
    Views,
    /// Vector search
    VectorSearch,
    /// Background processes
    Processes,
    /// Lifecycle management
    Lifecycle,
    /// Custom capability
    Custom(String),
}

/// Registry of all agents in the system.
#[derive(Debug, Default)]
pub struct AgentRegistry {
    /// Map of agent ID to agent info
    agents: HashMap<String, AgentInfo>,

    /// Map of capability to list of agent IDs
    capabilities: HashMap<AgentCapability, Vec<String>>,
}

/// Coordinator for rhythmic external coordination cycles.
///
/// Pulses allow external systems to synchronize with KoruDelta's
/// internal operations. Each phase of a pulse corresponds to a
/// different type of coordination.
#[derive(Debug)]
pub struct PulseCoordinator {
    /// Current phase
    current_phase: RwLock<CoordinationPhase>,

    /// Phase sequence
    phase_sequence: Vec<CoordinationPhase>,

    /// Current position in sequence
    sequence_position: RwLock<usize>,
}

/// Phases of a coordination cycle.
///
/// These phases provide hooks for external systems to coordinate
/// their operations with KoruDelta's internal state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CoordinationPhase {
    /// Input phase - external systems provide input
    Input,
    /// Processing phase - internal computation
    Processing,
    /// Output phase - results available to external systems
    Output,
    /// Consolidation phase - memory optimization
    Consolidation,
    /// Exploration phase - internal pattern discovery
    Exploration,
    /// Idle phase - waiting for next cycle
    #[default]
    Idle,
}

impl KoruOrchestrator {
    /// Create a new orchestrator with default configuration.
    pub fn new() -> Self {
        let engine = SharedEngine::new();
        Self::with_engine(engine)
    }

    /// Create a new orchestrator with a specific engine.
    ///
    /// # LCA Pattern
    ///
    /// The orchestrator initializes with:
    /// - `local_root` = RootType::Orchestrator (from shared field roots)
    /// - `field` = Handle to the unified distinction engine
    pub fn with_engine(engine: SharedEngine) -> Self {
        let local_root = engine.root(RootType::Orchestrator).clone();
        let field = FieldHandle::new(&engine);

        let pulse = PulseCoordinator::new();

        Self {
            engine,
            field,
            local_root: RwLock::new(local_root),
            agents: RwLock::new(AgentRegistry::default()),
            pulse,
            agents_registered: AtomicU64::new(0),
            pulses_triggered: AtomicU64::new(0),
        }
    }

    /// Get the shared engine.
    pub fn engine(&self) -> &SharedEngine {
        &self.engine
    }

    /// Get the field handle.
    pub fn field(&self) -> &FieldHandle {
        &self.field
    }

    /// Get the local root distinction.
    pub fn local_root(&self) -> Distinction {
        self.local_root.read().unwrap().clone()
    }

    // ========================================================================
    // Agent Registration
    // ========================================================================

    /// Register an agent with the orchestrator.
    ///
    /// # LCA Pattern
    ///
    /// Registration synthesizes: `ΔNew = ΔLocal_Root ⊕ ΔRegisterAgent_Action`
    pub fn register_agent(&self, info: AgentInfo) {
        // Synthesize registration action
        let action = PulseAction::RegisterAgent {
            agent_id: info.id.clone(),
            agent_type: info.agent_type.clone(),
        };
        let _ = self.synthesize_action_internal(action);

        // Add to registry
        let mut agents = self.agents.write().unwrap();

        // Index capabilities
        for cap in &info.capabilities {
            agents
                .capabilities
                .entry(cap.clone())
                .or_default()
                .push(info.id.clone());
        }

        // Store agent info
        agents.agents.insert(info.id.clone(), info);

        self.agents_registered.fetch_add(1, Ordering::SeqCst);
    }

    /// Unregister an agent.
    ///
    /// # LCA Pattern
    ///
    /// Unregistration synthesizes: `ΔNew = ΔLocal_Root ⊕ ΔUnregisterAgent_Action`
    pub fn unregister_agent(&self, agent_id: &str) {
        // Synthesize unregistration action
        let action = PulseAction::UnregisterAgent {
            agent_id: agent_id.to_string(),
        };
        let _ = self.synthesize_action_internal(action);

        // Remove from registry
        let mut agents = self.agents.write().unwrap();

        if let Some(info) = agents.agents.remove(agent_id) {
            // Remove from capability indices
            for cap in &info.capabilities {
                if let Some(ids) = agents.capabilities.get_mut(cap) {
                    ids.retain(|id| id != agent_id);
                }
            }
        }
    }

    /// Get information about a registered agent.
    pub fn get_agent(&self, agent_id: &str) -> Option<AgentInfo> {
        let agents = self.agents.read().unwrap();
        agents.agents.get(agent_id).cloned()
    }

    /// Get all registered agents.
    pub fn list_agents(&self) -> Vec<AgentInfo> {
        let agents = self.agents.read().unwrap();
        agents.agents.values().cloned().collect()
    }

    /// Find agents with a specific capability.
    pub fn find_agents_by_capability(&self, capability: AgentCapability) -> Vec<AgentInfo> {
        let agents = self.agents.read().unwrap();
        agents
            .capabilities
            .get(&capability)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| agents.agents.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    // ========================================================================
    // Pulse Coordination
    // ========================================================================

    /// Trigger a coordination pulse with a specific phase.
    ///
    /// # LCA Pattern
    ///
    /// Pulse triggers synthesize: `ΔNew = ΔLocal_Root ⊕ ΔPulse_Action`
    pub fn pulse(&self, phase: CoordinationPhase) {
        // Update pulse coordinator
        *self.pulse.current_phase.write().unwrap() = phase;

        // Synthesize pulse action
        let phase_str = format!("{:?}", phase);
        let action = PulseAction::TriggerPulse { phase: phase_str };
        let _ = self.synthesize_action_internal(action);

        self.pulses_triggered.fetch_add(1, Ordering::SeqCst);
    }

    /// Advance to the next phase in the sequence.
    pub fn advance_phase(&self) {
        let next = self.pulse.next_phase();
        self.pulse(next);
    }

    /// Get the current coordination phase.
    pub fn current_phase(&self) -> CoordinationPhase {
        *self.pulse.current_phase.read().unwrap()
    }

    /// Get the pulse coordinator.
    pub fn pulse_coordinator(&self) -> &PulseCoordinator {
        &self.pulse
    }

    // ========================================================================
    // Synthesis
    // ========================================================================

    /// Synthesize a KoruAction with the orchestrator's local root.
    ///
    /// # LCA Pattern
    ///
    /// `ΔNew = ΔLocal_Root ⊕ ΔAction`
    pub fn synthesize_action(&self, action: KoruAction) -> Distinction {
        let engine = self.field.engine_arc();
        let action_distinction = action.to_canonical_structure(engine);
        let local_root = self.local_root.read().unwrap().clone();
        let new_root = engine.synthesize(&local_root, &action_distinction);
        *self.local_root.write().unwrap() = new_root.clone();
        new_root
    }

    /// Internal synthesis helper for orchestrator-specific actions.
    fn synthesize_action_internal(&self, action: PulseAction) -> Distinction {
        let engine = self.field.engine_arc();
        let action_distinction = action.to_canonical_structure(engine);
        let local_root = self.local_root.read().unwrap().clone();
        let new_root = engine.synthesize(&local_root, &action_distinction);
        *self.local_root.write().unwrap() = new_root.clone();
        new_root
    }

    // ========================================================================
    // Cross-Agent Synthesis
    // ========================================================================

    /// Get the local root of a registered agent.
    ///
    /// This enables cross-agent synthesis by allowing one agent to reference
    /// another's causal anchor.
    pub fn get_agent_root(&self, agent_id: &str) -> Option<Distinction> {
        self.get_agent(agent_id).map(|info| info.root)
    }

    /// Synthesize a distinction from multiple agent roots.
    ///
    /// This is the foundation of cross-agent causality - creating distinctions
    /// that span multiple agent perspectives.
    ///
    /// # Formula
    /// `ΔCombined = ΔAgent1_Root ⊕ ΔAgent2_Root ⊕ ... ⊕ ΔAction`
    pub fn synthesize_cross_agent(
        &self,
        agent_ids: &[&str],
        action: KoruAction,
    ) -> Option<Distinction> {
        let engine = self.field.engine_arc();

        // Collect all agent roots
        let roots: Vec<Distinction> = agent_ids
            .iter()
            .filter_map(|id| self.get_agent_root(id))
            .collect();

        if roots.is_empty() {
            return None;
        }

        // Synthesize all roots together
        let combined_root = roots.iter().skip(1).fold(roots[0].clone(), |acc, root| {
            engine.synthesize(&acc, root)
        });

        // Synthesize with action
        let action_distinction = action.to_canonical_structure(engine);
        Some(engine.synthesize(&combined_root, &action_distinction))
    }

    /// Get all registered agent IDs.
    pub fn list_agent_ids(&self) -> Vec<String> {
        let agents = self.agents.read().unwrap();
        agents.agents.keys().cloned().collect()
    }

    /// Get count of registered agents.
    pub fn agent_count(&self) -> usize {
        let agents = self.agents.read().unwrap();
        agents.agents.len()
    }

    // ========================================================================
    // Statistics
    // ========================================================================

    /// Get orchestrator statistics.
    pub fn stats(&self) -> OrchestratorStats {
        let agents = self.agents.read().unwrap();
        OrchestratorStats {
            agents_registered: self.agents_registered.load(Ordering::SeqCst),
            pulses_triggered: self.pulses_triggered.load(Ordering::SeqCst),
            active_agents: agents.agents.len() as u64,
            current_phase: self.current_phase(),
        }
    }
}

impl Default for KoruOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// LCA Pattern Verification
// ============================================================================

// Note: KoruOrchestrator follows the LCA pattern internally:
// - Has local_root (RootType::Orchestrator)
// - All coordination operations synthesize: ΔNew = ΔLocal_Root ⊕ ΔAction
// - Uses interior mutability for ergonomic &self API
//
// The LocalCausalAgent trait is not implemented because the trait requires
// &mut self for synthesize_action, which would force an ergonomic regression
// on the public API. The architecture is followed; the trait is omitted.

impl AgentRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an agent.
    pub fn register(&mut self, info: AgentInfo) {
        for cap in &info.capabilities {
            self.capabilities
                .entry(cap.clone())
                .or_default()
                .push(info.id.clone());
        }
        self.agents.insert(info.id.clone(), info);
    }

    /// Unregister an agent.
    pub fn unregister(&mut self, agent_id: &str) {
        if let Some(info) = self.agents.remove(agent_id) {
            for cap in &info.capabilities {
                if let Some(ids) = self.capabilities.get_mut(cap) {
                    ids.retain(|id| id != agent_id);
                }
            }
        }
    }

    /// Get agent by ID.
    pub fn get(&self, agent_id: &str) -> Option<&AgentInfo> {
        self.agents.get(agent_id)
    }

    /// List all agents.
    pub fn list(&self) -> Vec<&AgentInfo> {
        self.agents.values().collect()
    }

    /// Find agents by capability.
    pub fn find_by_capability(&self, capability: AgentCapability) -> Vec<&AgentInfo> {
        self.capabilities
            .get(&capability)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.agents.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl PulseCoordinator {
    /// Create a new pulse coordinator with default phase sequence.
    pub fn new() -> Self {
        Self::with_sequence(vec![
            CoordinationPhase::Input,
            CoordinationPhase::Processing,
            CoordinationPhase::Output,
            CoordinationPhase::Consolidation,
        ])
    }

    /// Create a pulse coordinator with a custom phase sequence.
    pub fn with_sequence(phase_sequence: Vec<CoordinationPhase>) -> Self {
        Self {
            current_phase: RwLock::new(CoordinationPhase::Idle),
            phase_sequence,
            sequence_position: RwLock::new(0),
        }
    }

    /// Get the current phase.
    pub fn current_phase(&self) -> CoordinationPhase {
        *self.current_phase.read().unwrap()
    }

    /// Get the next phase in the sequence.
    pub fn next_phase(&self) -> CoordinationPhase {
        let mut position = self.sequence_position.write().unwrap();
        *position = (*position + 1) % self.phase_sequence.len();
        self.phase_sequence[*position]
    }

    /// Get the phase sequence.
    pub fn sequence(&self) -> &[CoordinationPhase] {
        &self.phase_sequence
    }

    /// Set the phase sequence.
    pub fn set_sequence(&mut self, sequence: Vec<CoordinationPhase>) {
        self.phase_sequence = sequence;
        *self.sequence_position.write().unwrap() = 0;
    }
}

impl Default for PulseCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

/// Orchestrator statistics.
#[derive(Debug, Clone)]
pub struct OrchestratorStats {
    pub agents_registered: u64,
    pub pulses_triggered: u64,
    pub active_agents: u64,
    pub current_phase: CoordinationPhase,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orchestrator_creation() {
        let orch = KoruOrchestrator::new();
        assert_eq!(orch.stats().agents_registered, 0);
        assert_eq!(orch.stats().pulses_triggered, 0);
    }

    #[test]
    fn test_agent_registration() {
        let orch = KoruOrchestrator::new();

        let agent = AgentInfo {
            id: "test_agent".to_string(),
            name: "Test Agent".to_string(),
            root: orch.engine().inner().d0().clone(),
            agent_type: "test".to_string(),
            capabilities: vec![AgentCapability::Storage],
        };

        orch.register_agent(agent);

        assert_eq!(orch.stats().agents_registered, 1);
        assert!(orch.get_agent("test_agent").is_some());
    }

    #[test]
    fn test_agent_unregistration() {
        let orch = KoruOrchestrator::new();

        let agent = AgentInfo {
            id: "test_agent".to_string(),
            name: "Test Agent".to_string(),
            root: orch.engine().inner().d0().clone(),
            agent_type: "test".to_string(),
            capabilities: vec![AgentCapability::Storage],
        };

        orch.register_agent(agent);
        orch.unregister_agent("test_agent");

        assert!(orch.get_agent("test_agent").is_none());
    }

    #[test]
    fn test_find_by_capability() {
        let orch = KoruOrchestrator::new();

        let agent1 = AgentInfo {
            id: "storage_agent".to_string(),
            name: "Storage Agent".to_string(),
            root: orch.engine().inner().d0().clone(),
            agent_type: "storage".to_string(),
            capabilities: vec![AgentCapability::Storage],
        };

        let agent2 = AgentInfo {
            id: "query_agent".to_string(),
            name: "Query Agent".to_string(),
            root: orch.engine().inner().d0().clone(),
            agent_type: "query".to_string(),
            capabilities: vec![AgentCapability::Query],
        };

        orch.register_agent(agent1);
        orch.register_agent(agent2);

        let storage_agents = orch.find_agents_by_capability(AgentCapability::Storage);
        assert_eq!(storage_agents.len(), 1);
        assert_eq!(storage_agents[0].id, "storage_agent");
    }

    #[test]
    fn test_pulse_coordination() {
        let orch = KoruOrchestrator::new();

        assert_eq!(orch.current_phase(), CoordinationPhase::Idle);

        orch.pulse(CoordinationPhase::Input);
        assert_eq!(orch.current_phase(), CoordinationPhase::Input);
        assert_eq!(orch.stats().pulses_triggered, 1);

        orch.advance_phase();
        assert_eq!(orch.current_phase(), CoordinationPhase::Processing);
    }

    #[test]
    fn test_pulse_sequence_wraps() {
        let orch = KoruOrchestrator::new();

        // Advance through all phases
        for _ in 0..4 {
            orch.advance_phase();
        }

        // Should wrap back to first phase
        assert_eq!(orch.current_phase(), CoordinationPhase::Input);
    }

    #[test]
    fn test_synthesize_action() {
        let orch = KoruOrchestrator::new();
        let root_before_id = orch.local_root().id().to_string();

        let action = KoruAction::Storage(crate::actions::StorageAction::Query {
            pattern_json: serde_json::json!({}),
        });

        let new_root = orch.synthesize_action(action);

        assert_ne!(new_root.id(), root_before_id);
        assert_eq!(orch.local_root().id(), new_root.id());
    }

    #[test]
    fn test_agent_registry_direct() {
        let orch = KoruOrchestrator::new();
        let mut registry = AgentRegistry::new();

        let agent = AgentInfo {
            id: "test".to_string(),
            name: "Test".to_string(),
            root: orch.engine().inner().d0().clone(),
            agent_type: "test".to_string(),
            capabilities: vec![AgentCapability::Custom("test_cap".to_string())],
        };

        registry.register(agent);
        assert_eq!(registry.list().len(), 1);

        registry.unregister("test");
        assert!(registry.list().is_empty());
    }

    #[test]
    fn test_custom_capability() {
        let orch = KoruOrchestrator::new();

        let agent = AgentInfo {
            id: "custom_agent".to_string(),
            name: "Custom Agent".to_string(),
            root: orch.engine().inner().d0().clone(),
            agent_type: "custom".to_string(),
            capabilities: vec![AgentCapability::Custom("my_feature".to_string())],
        };

        orch.register_agent(agent);

        let found = orch.find_agents_by_capability(AgentCapability::Custom("my_feature".to_string()));
        assert_eq!(found.len(), 1);
    }
}
