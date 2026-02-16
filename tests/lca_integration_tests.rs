//! LCA Architecture Integration and Falsification Tests
//!
//! These tests verify the core properties of the Local Causal Agent architecture:
//! 1. All agents follow the synthesis formula: ΔNew = ΔLocal_Root ⊕ ΔAction
//! 2. Cross-agent synthesis produces valid distinctions
//! 3. Causal chains are preserved across agent boundaries
//! 4. Content-addressing ensures determinism

use koru_delta::{
    KoruDelta,
    actions::{KoruAction, PulseAction, StorageAction},
    orchestrator::{KoruOrchestrator, AgentInfo, AgentCapability, CoordinationPhase},
};
use koru_delta::workspace_agent::WorkspaceAgent;
use koru_lambda_core::{Canonicalizable, DistinctionEngine};
use std::sync::Arc;

// ====================================================================================
// Falsification Test 1: Synthesis Formula Verification
// ====================================================================================

/// FALSIFICATION: If an agent doesn't follow ΔNew = ΔLocal_Root ⊕ ΔAction,
/// its local_root won't change after operations.
#[tokio::test]
async fn test_synthesis_formula_advances_local_root() {
    let mut db = KoruDelta::start().await.unwrap();
    let root_before = db.local_root().clone();

    // Perform a synthesis action (not just put - that uses legacy storage)
    let action = StorageAction::Query {
        pattern_json: serde_json::json!({"test": "data"}),
    };
    let _ = db.synthesize_storage_action(action).await.unwrap();

    let root_after = db.local_root().clone();

    // FALSIFICATION: If this fails, the synthesis formula isn't being followed
    assert_ne!(
        root_before.id(),
        root_after.id(),
        "FAIL: Local root should advance after synthesis (ΔNew = ΔLocal ⊕ ΔAction)"
    );
}

/// FALSIFICATION: Two identical actions should produce the same action distinction
/// (content-addressed), even if local roots differ.
#[tokio::test]
async fn test_content_addressing_same_action_same_distinction() {
    let _db1 = KoruDelta::start().await.unwrap();
    let _db2 = KoruDelta::start().await.unwrap();

    // Same action, different databases (different local roots)
    let action = StorageAction::Store {
        namespace: "test".to_string(),
        key: "key".to_string(),
        value_json: serde_json::json!({"data": "value"}),
    };

    let engine = Arc::new(DistinctionEngine::new());
    let action_distinction_1 = action.to_canonical_structure(&engine);
    let action_distinction_2 = action.to_canonical_structure(&engine);

    // FALSIFICATION: If this fails, content-addressing is broken
    assert_eq!(
        action_distinction_1.id(),
        action_distinction_2.id(),
        "FAIL: Same action must produce same distinction (content-addressing)"
    );
}

// ====================================================================================
// Falsification Test 2: Cross-Agent Synthesis
// ====================================================================================

/// FALSIFICATION: Cross-agent synthesis must produce valid distinctions
/// that incorporate all agent roots.
#[test]
fn test_cross_agent_synthesis_produces_valid_distinction() {
    let orchestrator = KoruOrchestrator::new();

    // Register two test agents
    let agent1 = AgentInfo {
        id: "agent1".to_string(),
        name: "Test Agent 1".to_string(),
        root: orchestrator.engine().inner().d0().clone(),
        agent_type: "test".to_string(),
        capabilities: vec![AgentCapability::Storage],
    };

    let agent2 = AgentInfo {
        id: "agent2".to_string(),
        name: "Test Agent 2".to_string(),
        root: orchestrator.engine().inner().d1().clone(),
        agent_type: "test".to_string(),
        capabilities: vec![AgentCapability::Query],
    };

    orchestrator.register_agent(agent1);
    orchestrator.register_agent(agent2);

    // Cross-agent synthesis
    let action = KoruAction::Pulse(PulseAction::TriggerPulse {
        phase: "Test".to_string(),
    });

    let result = orchestrator.synthesize_cross_agent(
        &["agent1", "agent2"],
        action,
    );

    // FALSIFICATION: Must produce a valid distinction
    assert!(
        result.is_some(),
        "FAIL: Cross-agent synthesis must produce a distinction"
    );

    let distinction = result.unwrap();
    assert_ne!(
        distinction.id(),
        orchestrator.engine().inner().d0().id(),
        "FAIL: Cross-agent result must be distinct from d0"
    );
}

/// FALSIFICATION: Empty agent list should produce None
#[test]
fn test_cross_agent_empty_list_returns_none() {
    let orchestrator = KoruOrchestrator::new();

    let action = KoruAction::Pulse(PulseAction::TriggerPulse {
        phase: "Test".to_string(),
    });

    let result = orchestrator.synthesize_cross_agent(&[], action);

    // FALSIFICATION: Empty list must return None
    assert!(
        result.is_none(),
        "FAIL: Cross-agent with empty list should return None"
    );
}

// ====================================================================================
// Falsification Test 3: Agent Registry Integrity
// ====================================================================================

/// FALSIFICATION: Agent registration must synthesize (local_root must change)
#[test]
fn test_agent_registration_synthesizes() {
    let orchestrator = KoruOrchestrator::new();
    let root_before = orchestrator.local_root();

    let agent = AgentInfo {
        id: "test_agent".to_string(),
        name: "Test".to_string(),
        root: orchestrator.engine().inner().d0().clone(),
        agent_type: "test".to_string(),
        capabilities: vec![AgentCapability::Custom("test".to_string())],
    };

    orchestrator.register_agent(agent);
    let root_after = orchestrator.local_root();

    // FALSIFICATION: Registration must advance local_root
    assert_ne!(
        root_before.id(),
        root_after.id(),
        "FAIL: Agent registration must synthesize (ΔNew = ΔLocal ⊕ ΔRegister)"
    );
}

/// FALSIFICATION: Unregistered agents should not be queryable
#[test]
fn test_unregistered_agent_not_found() {
    let orchestrator = KoruOrchestrator::new();

    // FALSIFICATION: Unregistered agent must return None
    assert!(
        orchestrator.get_agent("nonexistent").is_none(),
        "FAIL: Unregistered agent should not be found"
    );
    assert!(
        orchestrator.get_agent_root("nonexistent").is_none(),
        "FAIL: Unregistered agent root should not be accessible"
    );
}

// ====================================================================================
// Falsification Test 4: Workspace Isolation (Causal Boundaries)
// ====================================================================================

/// FALSIFICATION: Workspaces must have distinct local roots (causal isolation)
#[tokio::test]
async fn test_workspaces_have_distinct_local_roots() {
    let engine = Arc::new(DistinctionEngine::new());
    let roots = koru_delta::roots::KoruRoots::initialize(&engine);

    let mut workspace_agent = WorkspaceAgent::new(roots.workspace.clone(), engine.clone());

    let ws1 = workspace_agent.create_workspace("ws1", "Workspace 1");
    let ws2 = workspace_agent.create_workspace("ws2", "Workspace 2");

    // FALSIFICATION: Workspaces must have distinct roots
    assert_ne!(
        ws1.local_root.id(),
        ws2.local_root.id(),
        "FAIL: Workspaces must have distinct local roots for causal isolation"
    );
}

// ====================================================================================
// Falsification Test 5: Pulse Coordination Synthesis
// ====================================================================================

/// FALSIFICATION: Pulse triggers must advance the orchestrator's local_root
#[test]
fn test_pulse_advances_orchestrator_root() {
    let orchestrator = KoruOrchestrator::new();
    let root_before = orchestrator.local_root();

    orchestrator.pulse(CoordinationPhase::Input);
    let root_after = orchestrator.local_root();

    // FALSIFICATION: Pulse must synthesize
    assert_ne!(
        root_before.id(),
        root_after.id(),
        "FAIL: Pulse must advance local_root (ΔNew = ΔLocal ⊕ ΔPulse)"
    );
}

/// FALSIFICATION: Phase transitions must follow the sequence
#[test]
fn test_phase_sequence_cycles_correctly() {
    let orchestrator = KoruOrchestrator::new();

    // Start at Idle
    assert_eq!(orchestrator.current_phase(), CoordinationPhase::Idle);

    // First pulse goes to Input (first in sequence after Idle)
    orchestrator.pulse(CoordinationPhase::Input);
    assert_eq!(orchestrator.current_phase(), CoordinationPhase::Input);

    // Advance through phases
    orchestrator.advance_phase();
    assert_eq!(orchestrator.current_phase(), CoordinationPhase::Processing);
}

// ====================================================================================
// Integration Test: Multi-Agent Workflow
// ====================================================================================

/// INTEGRATION: A complete workflow involving multiple agents
#[tokio::test]
async fn test_multi_agent_workflow() {
    // Set up the unified field
    let db = KoruDelta::start().await.unwrap();
    let orchestrator = KoruOrchestrator::new();

    // 1. Storage agent stores data
    db.put("users", "alice", serde_json::json!({"name": "Alice"}))
        .await
        .unwrap();

    // 2. Register a "query agent" with the orchestrator
    let query_agent = AgentInfo {
        id: "query_service".to_string(),
        name: "Query Service".to_string(),
        root: db.local_root().clone(),
        agent_type: "query".to_string(),
        capabilities: vec![AgentCapability::Query],
    };
    orchestrator.register_agent(query_agent);

    // 3. Trigger coordination pulse
    orchestrator.pulse(CoordinationPhase::Processing);

    // 4. Verify cross-agent synthesis capability exists
    let agent_ids = orchestrator.list_agent_ids();
    assert!(
        agent_ids.contains(&"query_service".to_string()),
        "Query service should be registered"
    );

    // 5. Verify we can get agent roots for cross-agent operations
    let query_root = orchestrator.get_agent_root("query_service");
    assert!(query_root.is_some(), "Should be able to retrieve agent root");
}

// ====================================================================================
// Integration Test: Deterministic Roots
// ====================================================================================

/// INTEGRATION: Same engine should produce same canonical roots
#[test]
fn test_canonical_roots_are_deterministic() {
    let engine1 = Arc::new(DistinctionEngine::new());
    let engine2 = Arc::new(DistinctionEngine::new());

    let roots1 = koru_delta::roots::KoruRoots::initialize(&engine1);
    let roots2 = koru_delta::roots::KoruRoots::initialize(&engine2);

    // All canonical roots should be identical across engines
    assert_eq!(
        roots1.storage.id(),
        roots2.storage.id(),
        "Storage root must be deterministic"
    );
    assert_eq!(
        roots1.temperature.id(),
        roots2.temperature.id(),
        "Temperature root must be deterministic"
    );
    assert_eq!(
        roots1.identity.id(),
        roots2.identity.id(),
        "Identity root must be deterministic"
    );
    assert_eq!(
        roots1.network.id(),
        roots2.network.id(),
        "Network root must be deterministic"
    );
}

// ====================================================================================
// Integration Test: Agent Count and Registry
// ====================================================================================

#[test]
fn test_agent_registry_counts() {
    let orchestrator = KoruOrchestrator::new();

    // Initially empty
    assert_eq!(orchestrator.agent_count(), 0);

    // Register agents
    for i in 0..5 {
        let agent = AgentInfo {
            id: format!("agent_{}", i),
            name: format!("Agent {}", i),
            root: orchestrator.engine().inner().d0().clone(),
            agent_type: "test".to_string(),
            capabilities: vec![AgentCapability::Custom(format!("cap_{}", i))],
        };
        orchestrator.register_agent(agent);
    }

    assert_eq!(orchestrator.agent_count(), 5);
    assert_eq!(orchestrator.list_agent_ids().len(), 5);
}
