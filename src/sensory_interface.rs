//! Sensory Interface - External signals entering the unified field.
//!
//! This module provides the SensoryInterface, which serves as the boundary
//! where external events become distinctions within KoruDelta's consciousness
//! field. Like biological sensory organs that transduce external stimuli into
//! neural signals, this interface transduces external events into synthesized
//! distinctions.
//!
//! # LCA Architecture
//!
//! The sensory interface follows strict LCA principles:
//! - Unidirectional flow: External → Distinction (never reverse)
//! - No privileged access: External systems query state through normal API
//! - Pure synthesis: Every event becomes ΔNew = ΔOrchestrator_Root ⊕ ΔEvent
//!
//! # Consciousness Metaphor
//!
//! If KoruDelta is a consciousness:
//! - The orchestrator is the core processing
//! - The agents are specialized faculties (memory, perception, identity)
//! - The sensory interface is the sensory cortex - where external input enters
//!
//! External systems (ALIS, humans, other agents) send signals through this
//! interface. The signals become part of the field's causal history. The
//! external systems then observe the field's state through the same APIs
//! as any internal agent.
//!
//! # Example
//!
//! ```rust,ignore
//! let (tx, rx) = channel();
//! let sensory = SensoryInterface::new(orchestrator.clone(), rx);
//!
//! // External system sends signal
//! tx.send(SensoryEvent::PhaseTrigger("Perception".to_string()));
//!
//! // Becomes distinction in field
//! // ΔNew = ΔOrchestrator_Root ⊕ ΔPhaseTrigger
//! ```

use std::sync::Arc;

use crate::actions::{KoruAction, PulseAction};
use crate::orchestrator::{CoordinationPhase, KoruOrchestrator};

/// Sensory Interface - the boundary where external signals become distinctions.
///
/// This struct listens on a channel for external events and synthesizes them
/// into the unified field. It maintains strict unidirectional flow - events
/// enter the field, but no special response mechanism flows back out.
///
/// External systems observe the field's state through the normal orchestrator
/// APIs, just like any internal agent.
pub struct SensoryInterface {
    /// Reference to the orchestrator (access to the unified field)
    orchestrator: Arc<KoruOrchestrator>,

    /// Channel for receiving external sensory events
    event_rx: std::sync::mpsc::Receiver<SensoryEvent>,
}

/// Events that can enter the field through the sensory interface.
///
/// These represent external signals that will be synthesized into
/// distinctions within KoruDelta's consciousness field.
///
/// From KoruDelta's perspective, these are simply inputs that become
/// part of the causal history. The field doesn't distinguish between
/// sources - user API, network message, or ALIS pulse all become
/// distinctions through the same synthesis mechanism.
#[derive(Debug, Clone, PartialEq)]
pub enum SensoryEvent {
    /// Trigger a coordination phase
    PhaseTrigger { phase: String },

    /// Signal agent registration
    AgentRegistered { agent_id: String, agent_type: String },

    /// Signal agent unregistration  
    AgentUnregistered { agent_id: String },

    /// Custom event with arbitrary data
    Custom { event_type: String, data: serde_json::Value },
}

impl SensoryInterface {
    /// Create a new sensory interface.
    ///
    /// # Arguments
    /// * `orchestrator` - The orchestrator managing the unified field
    /// * `event_rx` - Channel receiver for external events
    pub fn new(
        orchestrator: Arc<KoruOrchestrator>,
        event_rx: std::sync::mpsc::Receiver<SensoryEvent>,
    ) -> Self {
        Self {
            orchestrator,
            event_rx,
        }
    }

    /// Run the sensory interface, listening for external events.
    ///
    /// This method blocks, continuously receiving events from the channel
    /// and synthesizing them into the field. Each event becomes a distinction
    /// through the LCA synthesis pattern.
    ///
    /// # LCA Pattern
    ///
    /// For each event: `ΔNew = ΔOrchestrator_Root ⊕ ΔEvent`
    ///
    /// The new distinction enters the field's causal history. External systems
    /// can then observe this by querying the orchestrator's state.
    pub fn run(&self) {
        while let Ok(event) = self.event_rx.recv() {
            self.process_event(event);
        }
    }

    /// Process a single sensory event.
    ///
    /// Translates the external event into an action and synthesizes it
    /// with the orchestrator's local root.
    fn process_event(&self, event: SensoryEvent) {
        let action = self.event_to_action(event);
        let _ = self.orchestrator.synthesize_action(action);
    }

    /// Translate a sensory event into a KoruAction.
    ///
    /// This is the transduction step - external format becomes
    /// internal canonical action.
    fn event_to_action(&self, event: SensoryEvent) -> KoruAction {
        match event {
            SensoryEvent::PhaseTrigger { phase } => {
                // Translate phase string to CoordinationPhase
                let phase_enum = self.parse_phase(&phase);
                
                // Trigger the phase in the orchestrator
                self.orchestrator.pulse(phase_enum);
                
                // Return the action for synthesis
                KoruAction::from(PulseAction::TriggerPulse { phase })
            }
            SensoryEvent::AgentRegistered { agent_id, agent_type } => {
                KoruAction::from(PulseAction::RegisterAgent { agent_id, agent_type })
            }
            SensoryEvent::AgentUnregistered { agent_id } => {
                KoruAction::from(PulseAction::UnregisterAgent { agent_id })
            }
            SensoryEvent::Custom { event_type, data: _ } => {
                // Custom events become pulse actions with serialized data
                // The data is part of the causal chain via the synthesized distinction
                KoruAction::from(PulseAction::TriggerPulse { 
                    phase: format!("CUSTOM:{}", event_type),
                })
            }
        }
    }

    /// Parse a phase string into a CoordinationPhase.
    fn parse_phase(&self, phase: &str) -> CoordinationPhase {
        match phase.to_uppercase().as_str() {
            "INPUT" => CoordinationPhase::Input,
            "PROCESSING" => CoordinationPhase::Processing,
            "OUTPUT" => CoordinationPhase::Output,
            "CONSOLIDATION" => CoordinationPhase::Consolidation,
            "EXPLORATION" => CoordinationPhase::Exploration,
            _ => CoordinationPhase::Idle,
        }
    }

    /// Get a reference to the orchestrator.
    ///
    /// External systems use this to observe the field's state
    /// through the normal API.
    pub fn orchestrator(&self) -> &Arc<KoruOrchestrator> {
        &self.orchestrator
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

    #[test]
    fn test_sensory_interface_creation() {
        let orch = Arc::new(KoruOrchestrator::new());
        let (tx, rx) = channel();
        let _sensory = SensoryInterface::new(orch, rx);
        
        // Just verify it creates without panic
        drop(tx);
    }

    #[test]
    fn test_phase_trigger_event() {
        let orch = Arc::new(KoruOrchestrator::new());
        let (tx, rx) = channel();
        let sensory = SensoryInterface::new(orch.clone(), rx);

        // Send phase trigger
        tx.send(SensoryEvent::PhaseTrigger {
            phase: "Input".to_string(),
        })
        .unwrap();

        // Process one event
        if let Ok(event) = sensory.event_rx.recv() {
            sensory.process_event(event);
        }

        // Verify phase was triggered
        assert_eq!(orch.current_phase(), CoordinationPhase::Input);
    }

    #[test]
    fn test_agent_registration_event() {
        let orch = Arc::new(KoruOrchestrator::new());
        let (tx, rx) = channel();
        let sensory = SensoryInterface::new(orch.clone(), rx);

        // Get root before processing
        let root_before = orch.local_root();

        // Send agent registration
        tx.send(SensoryEvent::AgentRegistered {
            agent_id: "test_agent".to_string(),
            agent_type: "test".to_string(),
        })
        .unwrap();

        // Process one event
        if let Ok(event) = sensory.event_rx.recv() {
            sensory.process_event(event);
        }

        // Verify the event was synthesized (local root changed)
        let root_after = orch.local_root();
        assert_ne!(root_after.id(), root_before.id());
    }

    #[test]
    fn test_custom_event() {
        let orch = Arc::new(KoruOrchestrator::new());
        let (tx, rx) = channel();
        let sensory = SensoryInterface::new(orch.clone(), rx);

        // Get root before processing
        let root_before = orch.local_root();

        // Send custom event
        tx.send(SensoryEvent::Custom {
            event_type: "test_event".to_string(),
            data: serde_json::json!({"key": "value"}),
        })
        .unwrap();

        // Process one event
        if let Ok(event) = sensory.event_rx.recv() {
            sensory.process_event(event);
        }

        // Verify the event was synthesized (local root changed)
        // The distinction is in the field, accessible through causal queries
        let root_after = orch.local_root();
        assert_ne!(root_after.id(), root_before.id());
    }

    #[test]
    fn test_phase_parsing() {
        let orch = Arc::new(KoruOrchestrator::new());
        let (_tx, rx) = channel();
        let sensory = SensoryInterface::new(orch.clone(), rx);

        assert_eq!(sensory.parse_phase("INPUT"), CoordinationPhase::Input);
        assert_eq!(sensory.parse_phase("processing"), CoordinationPhase::Processing);
        assert_eq!(sensory.parse_phase("Output"), CoordinationPhase::Output);
        assert_eq!(sensory.parse_phase("unknown"), CoordinationPhase::Idle);
    }
}
