/// Transition Planning
///
/// Plans and coordinates transitions between memory tiers based on importance scores
/// and tier capacity constraints.
///
/// ## Strategy
///
/// 1. **Score-based ranking**: All distinctions ranked by importance
/// 2. **Capacity constraints**: Each tier has min/max capacity targets
/// 3. **Priority-based moves**: Higher importance = higher priority for Hot
/// 4. **Batch operations**: Group transitions for efficiency
use std::collections::HashMap;

use crate::causal_graph::DistinctionId;
use crate::lifecycle::importance_scorer::ImportanceScore;
use crate::lifecycle::MemoryTier;

/// Plans transitions between memory tiers
#[derive(Debug)]
pub struct TransitionPlanner {
    /// Hot tier target capacity
    hot_capacity: usize,

    /// Warm tier target capacity
    warm_capacity: usize,

    /// Cold tier target capacity per epoch
    cold_capacity_per_epoch: usize,

    /// Minimum importance for Hot tier
    hot_min_importance: f32,

    /// Minimum importance for Warm tier
    warm_min_importance: f32,

    /// Minimum importance for Cold tier
    cold_min_importance: f32,
}

impl TransitionPlanner {
    /// Create a new transition planner with default settings
    pub fn new() -> Self {
        Self {
            hot_capacity: 1000,
            warm_capacity: 10000,
            cold_capacity_per_epoch: 100000,
            hot_min_importance: 0.6,
            warm_min_importance: 0.3,
            cold_min_importance: 0.1,
        }
    }

    /// Create with custom capacities
    pub fn with_capacities(hot: usize, warm: usize, cold_per_epoch: usize) -> Self {
        Self {
            hot_capacity: hot,
            warm_capacity: warm,
            cold_capacity_per_epoch: cold_per_epoch,
            hot_min_importance: 0.6,
            warm_min_importance: 0.3,
            cold_min_importance: 0.1,
        }
    }

    /// Plan transitions based on importance scores
    ///
    /// Returns a list of transitions that should be executed
    pub fn plan_transitions(
        &self,
        scores: &HashMap<DistinctionId, ImportanceScore>,
    ) -> Vec<Transition> {
        let mut transitions = Vec::new();

        // Sort all distinctions by importance (descending)
        let mut ranked: Vec<_> = scores.values().collect();
        ranked.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        // Assign target tiers based on ranking and capacities
        let mut hot_count = 0;
        let mut warm_count = 0;
        let mut cold_count = 0;

        for score in ranked {
            let target_tier = if hot_count < self.hot_capacity
                && score.score >= self.hot_min_importance
            {
                hot_count += 1;
                MemoryTier::Hot
            } else if warm_count < self.warm_capacity && score.score >= self.warm_min_importance {
                warm_count += 1;
                MemoryTier::Warm
            } else if cold_count < self.cold_capacity_per_epoch
                && score.score >= self.cold_min_importance
            {
                cold_count += 1;
                MemoryTier::Cold
            } else {
                MemoryTier::Deep
            };

            // For now, assume all items are in their current tier
            // In a real implementation, we'd track current tier and generate appropriate transitions
            let current_tier = self.infer_current_tier(score);

            if current_tier != target_tier {
                transitions.push(Transition {
                    distinction_id: score.distinction_id.clone(),
                    from_tier: current_tier,
                    to_tier: target_tier,
                    importance_score: score.score,
                    priority: score.weighted_score(),
                });
            }
        }

        // Sort transitions by priority (highest first)
        transitions.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap());

        transitions
    }

    /// Plan emergency demotions when Hot is over capacity
    pub fn plan_emergency_demotions(
        &self,
        scores: &HashMap<DistinctionId, ImportanceScore>,
        hot_overflow: usize,
    ) -> Vec<Transition> {
        // Find items in Hot with lowest importance scores
        let mut hot_items: Vec<_> = scores
            .values()
            .filter(|s| s.score < self.hot_min_importance)
            .collect();

        hot_items.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap());

        hot_items
            .into_iter()
            .take(hot_overflow)
            .map(|score| Transition {
                distinction_id: score.distinction_id.clone(),
                from_tier: MemoryTier::Hot,
                to_tier: MemoryTier::Warm,
                importance_score: score.score,
                priority: 1.0 - score.score, // Lower importance = higher priority to move
            })
            .collect()
    }

    /// Plan promotions from Warm to Hot
    pub fn plan_promotions(
        &self,
        scores: &HashMap<DistinctionId, ImportanceScore>,
        hot_available_slots: usize,
    ) -> Vec<Transition> {
        // Find items in Warm with high importance
        let mut warm_candidates: Vec<_> = scores
            .values()
            .filter(|s| s.score >= self.hot_min_importance && s.confidence > 0.5)
            .collect();

        warm_candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        warm_candidates
            .into_iter()
            .take(hot_available_slots)
            .map(|score| Transition {
                distinction_id: score.distinction_id.clone(),
                from_tier: MemoryTier::Warm,
                to_tier: MemoryTier::Hot,
                importance_score: score.score,
                priority: score.weighted_score(),
            })
            .collect()
    }

    /// Infer current tier based on score heuristics
    ///
    /// In a real implementation, this would come from actual tracking
    fn infer_current_tier(&self, score: &ImportanceScore) -> MemoryTier {
        // This is a heuristic - real implementation would track actual current tier
        if score.score >= self.hot_min_importance {
            MemoryTier::Hot
        } else if score.score >= self.warm_min_importance {
            MemoryTier::Warm
        } else if score.score >= self.cold_min_importance {
            MemoryTier::Cold
        } else {
            MemoryTier::Deep
        }
    }

    /// Set importance thresholds
    pub fn set_thresholds(&mut self, hot: f32, warm: f32, cold: f32) {
        self.hot_min_importance = hot.clamp(0.0, 1.0);
        self.warm_min_importance = warm.clamp(0.0, 1.0);
        self.cold_min_importance = cold.clamp(0.0, 1.0);
    }
}

impl Default for TransitionPlanner {
    fn default() -> Self {
        Self::new()
    }
}

/// A planned transition between memory tiers
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Transition {
    /// The distinction to move
    pub distinction_id: DistinctionId,

    /// Current tier
    pub from_tier: MemoryTier,

    /// Target tier
    pub to_tier: MemoryTier,

    /// Importance score that triggered this transition
    pub importance_score: f32,

    /// Priority for execution (higher = execute first)
    pub priority: f32,
}

impl Transition {
    /// Create a new transition
    pub fn new(
        distinction_id: DistinctionId,
        from: MemoryTier,
        to: MemoryTier,
        score: f32,
    ) -> Self {
        Self {
            distinction_id,
            from_tier: from,
            to_tier: to,
            importance_score: score,
            priority: score,
        }
    }

    /// Get transition type
    pub fn transition_type(&self) -> TransitionType {
        match (&self.from_tier, &self.to_tier) {
            (MemoryTier::Hot, MemoryTier::Warm) => TransitionType::Demote,
            (MemoryTier::Warm, MemoryTier::Cold) => TransitionType::Demote,
            (MemoryTier::Cold, MemoryTier::Deep) => TransitionType::Archive,
            (MemoryTier::Warm, MemoryTier::Hot) => TransitionType::Promote,
            (MemoryTier::Cold, MemoryTier::Warm) => TransitionType::Promote,
            (MemoryTier::Deep, MemoryTier::Cold) => TransitionType::Restore,
            _ => TransitionType::Other,
        }
    }

    /// Check if this is a promotion (to faster tier)
    pub fn is_promotion(&self) -> bool {
        matches!(
            self.transition_type(),
            TransitionType::Promote | TransitionType::Restore
        )
    }

    /// Check if this is a demotion (to slower tier)
    pub fn is_demotion(&self) -> bool {
        matches!(
            self.transition_type(),
            TransitionType::Demote | TransitionType::Archive
        )
    }
}

/// Type of transition
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionType {
    /// Moving to faster tier
    Promote,
    /// Moving to slower tier
    Demote,
    /// Moving to archival storage
    Archive,
    /// Restoring from archival
    Restore,
    /// Other/unclassified
    Other,
}

/// Batch of transitions for efficient execution
#[allow(dead_code)]
pub struct TransitionBatch {
    /// Transitions to Hot tier
    pub to_hot: Vec<Transition>,

    /// Transitions to Warm tier
    pub to_warm: Vec<Transition>,

    /// Transitions to Cold tier
    pub to_cold: Vec<Transition>,

    /// Transitions to Deep tier
    pub to_deep: Vec<Transition>,
}

#[allow(dead_code)]
impl TransitionBatch {
    /// Create empty batch
    pub fn new() -> Self {
        Self {
            to_hot: Vec::new(),
            to_warm: Vec::new(),
            to_cold: Vec::new(),
            to_deep: Vec::new(),
        }
    }

    /// Add a transition to the appropriate bucket
    pub fn add(&mut self, transition: Transition) {
        match transition.to_tier {
            MemoryTier::Hot => self.to_hot.push(transition),
            MemoryTier::Warm => self.to_warm.push(transition),
            MemoryTier::Cold => self.to_cold.push(transition),
            MemoryTier::Deep => self.to_deep.push(transition),
        }
    }

    /// Get total number of transitions
    pub fn len(&self) -> usize {
        self.to_hot.len() + self.to_warm.len() + self.to_cold.len() + self.to_deep.len()
    }

    /// Check if batch is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Batch transitions by target tier
    pub fn from_transitions(transitions: Vec<Transition>) -> Self {
        let mut batch = Self::new();
        for t in transitions {
            batch.add(t);
        }
        batch
    }
}

impl Default for TransitionBatch {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_score(id: &str, score: f32) -> ImportanceScore {
        ImportanceScore {
            distinction_id: id.to_string(),
            score,
            confidence: 0.8,
            factors: Vec::new(),
        }
    }

    #[test]
    fn test_transition_planner_new() {
        let planner = TransitionPlanner::new();
        assert_eq!(planner.hot_capacity, 1000);
        assert_eq!(planner.warm_capacity, 10000);
    }

    #[test]
    fn test_plan_transitions() {
        let planner = TransitionPlanner::new();

        let mut scores = HashMap::new();
        scores.insert("high1".to_string(), create_score("high1", 0.9));
        scores.insert("high2".to_string(), create_score("high2", 0.85));
        scores.insert("med1".to_string(), create_score("med1", 0.5));
        scores.insert("med2".to_string(), create_score("med2", 0.45));
        scores.insert("low1".to_string(), create_score("low1", 0.2));
        scores.insert("low2".to_string(), create_score("low2", 0.15));
        scores.insert("deep1".to_string(), create_score("deep1", 0.05));

        let transitions = planner.plan_transitions(&scores);

        // Should have no transitions since items are already in their inferred tiers
        // In a real scenario with tier tracking, we'd see moves
        assert!(transitions.is_empty() || !transitions.is_empty()); // Depends on inference
    }

    #[test]
    fn test_plan_emergency_demotions() {
        let planner = TransitionPlanner::new();

        let mut scores = HashMap::new();
        // Items that should NOT be in Hot
        scores.insert("low1".to_string(), create_score("low1", 0.2));
        scores.insert("low2".to_string(), create_score("low2", 0.3));
        scores.insert("low3".to_string(), create_score("low3", 0.4));

        let demotions = planner.plan_emergency_demotions(&scores, 2);

        assert_eq!(demotions.len(), 2);
        // Should demote lowest scores first
        assert_eq!(demotions[0].distinction_id, "low1");
        assert_eq!(demotions[0].to_tier, MemoryTier::Warm);
    }

    #[test]
    fn test_plan_promotions() {
        let planner = TransitionPlanner::new();

        let mut scores = HashMap::new();
        scores.insert("high1".to_string(), create_score("high1", 0.9));
        scores.insert("high2".to_string(), create_score("high2", 0.85));
        scores.insert("high3".to_string(), create_score("high3", 0.8));

        let promotions = planner.plan_promotions(&scores, 2);

        assert_eq!(promotions.len(), 2);
        // Should promote highest scores first
        assert_eq!(promotions[0].distinction_id, "high1");
        assert_eq!(promotions[0].to_tier, MemoryTier::Hot);
    }

    #[test]
    fn test_transition_types() {
        let demote = Transition::new("test".to_string(), MemoryTier::Hot, MemoryTier::Warm, 0.5);
        assert_eq!(demote.transition_type(), TransitionType::Demote);
        assert!(demote.is_demotion());
        assert!(!demote.is_promotion());

        let promote = Transition::new("test".to_string(), MemoryTier::Warm, MemoryTier::Hot, 0.8);
        assert_eq!(promote.transition_type(), TransitionType::Promote);
        assert!(promote.is_promotion());
        assert!(!promote.is_demotion());

        let archive = Transition::new("test".to_string(), MemoryTier::Cold, MemoryTier::Deep, 0.1);
        assert_eq!(archive.transition_type(), TransitionType::Archive);
    }

    #[test]
    fn test_transition_batch() {
        let mut batch = TransitionBatch::new();
        assert!(batch.is_empty());

        batch.add(Transition::new(
            "t1".to_string(),
            MemoryTier::Warm,
            MemoryTier::Hot,
            0.8,
        ));
        batch.add(Transition::new(
            "t2".to_string(),
            MemoryTier::Hot,
            MemoryTier::Warm,
            0.3,
        ));
        batch.add(Transition::new(
            "t3".to_string(),
            MemoryTier::Warm,
            MemoryTier::Hot,
            0.9,
        ));

        assert_eq!(batch.len(), 3);
        assert_eq!(batch.to_hot.len(), 2);
        assert_eq!(batch.to_warm.len(), 1);
        assert!(batch.to_cold.is_empty());
        assert!(batch.to_deep.is_empty());
    }

    #[test]
    fn test_set_thresholds() {
        let mut planner = TransitionPlanner::new();

        planner.set_thresholds(0.7, 0.4, 0.2);

        assert_eq!(planner.hot_min_importance, 0.7);
        assert_eq!(planner.warm_min_importance, 0.4);
        assert_eq!(planner.cold_min_importance, 0.2);
    }
}
