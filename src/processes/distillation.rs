/// Distillation Process: Natural selection of distinctions.
///
/// This process "distills" history by keeping only the "fit" distinctions
/// and archiving/discarding the "unfit" ones. Like evolution selecting
/// for beneficial traits.
///
/// ## Fitness Criteria
///
/// - High reference count = fit (many things point to it)
/// - Many causal descendants = fit (important in history)
/// - Recent = fit (still relevant)
/// - Low fitness = archive to Deep or discard
///
/// ## The Algorithm
///
/// 1. Score each distinction's fitness
/// 2. Classify as fit or unfit
/// 3. Keep fit distinctions in working memory
/// 4. Archive unfit to Deep (or discard if truly unimportant)
use crate::causal_graph::CausalGraph;
use crate::memory::{ColdMemory, DeepMemory};
use crate::reference_graph::ReferenceGraph;
use chrono::{DateTime, Duration, Utc};
use std::sync::atomic::{AtomicU64, Ordering};

/// Distillation configuration.
#[derive(Debug, Clone)]
pub struct DistillationConfig {
    /// How often to run distillation (seconds)
    pub interval_secs: u64,

    /// Fitness threshold (references >= this are "fit")
    pub fitness_threshold: usize,

    /// Age threshold (older than this loses fitness)
    pub age_threshold: Duration,

    /// Descendant bonus (each descendant adds this much fitness)
    pub descendant_bonus: usize,
}

impl Default for DistillationConfig {
    fn default() -> Self {
        Self {
            interval_secs: 3600,              // Hourly
            fitness_threshold: 2,             // 2+ references = fit
            age_threshold: Duration::days(7), // Week old = less fit
            descendant_bonus: 1,              // Each descendant adds 1
        }
    }
}

/// Distillation Process - natural selection of distinctions.
pub struct DistillationProcess {
    config: DistillationConfig,
    distinctions_evaluated: AtomicU64,
    distinctions_preserved: AtomicU64,
    distinctions_archived: AtomicU64,
}

/// Fitness score for a distinction.
#[derive(Debug, Clone)]
pub struct Fitness {
    pub distinction_id: String,
    pub reference_count: usize,
    pub descendant_count: usize,
    pub age_days: i64,
    pub total_score: i64,
}

impl DistillationProcess {
    /// Create new distillation process.
    pub fn new() -> Self {
        Self::with_config(DistillationConfig::default())
    }

    /// Create with custom config.
    pub fn with_config(config: DistillationConfig) -> Self {
        Self {
            config,
            distinctions_evaluated: AtomicU64::new(0),
            distinctions_preserved: AtomicU64::new(0),
            distinctions_archived: AtomicU64::new(0),
        }
    }

    /// Calculate fitness for a distinction.
    ///
    /// Fitness = references + descendants - age_penalty
    pub fn calculate_fitness(
        &self,
        distinction_id: &str,
        reference_graph: &ReferenceGraph,
        causal_graph: &CausalGraph,
        timestamp: DateTime<Utc>,
    ) -> Fitness {
        let reference_count = reference_graph.reference_count(&distinction_id.to_string());
        let descendant_count = causal_graph.descendants(distinction_id).len();
        let age = Utc::now().signed_duration_since(timestamp);
        let age_days = age.num_days();

        // Calculate score
        let mut score = reference_count as i64;
        score += (descendant_count * self.config.descendant_bonus) as i64;

        // Age penalty: lose 1 point per day after threshold
        if age > self.config.age_threshold {
            let excess_days = (age - self.config.age_threshold).num_days();
            score -= excess_days;
        }

        Fitness {
            distinction_id: distinction_id.to_string(),
            reference_count,
            descendant_count,
            age_days,
            total_score: score,
        }
    }

    /// Classify distinctions as fit or unfit.
    pub fn classify_distinctions(
        &self,
        distinctions: &[(String, DateTime<Utc>)], // (id, timestamp)
        reference_graph: &ReferenceGraph,
        causal_graph: &CausalGraph,
    ) -> Classification {
        let mut fit = Vec::new();
        let mut unfit = Vec::new();

        for (id, timestamp) in distinctions {
            let fitness = self.calculate_fitness(id, reference_graph, causal_graph, *timestamp);
            self.distinctions_evaluated.fetch_add(1, Ordering::Relaxed);

            if fitness.total_score >= self.config.fitness_threshold as i64 {
                fit.push(fitness);
            } else {
                unfit.push(fitness);
            }
        }

        Classification { fit, unfit }
    }

    /// Distill a cold epoch - keep fit, archive unfit.
    pub fn distill_epoch(
        &self,
        _cold: &ColdMemory,
        deep: &DeepMemory,
        epoch_num: usize,
        _reference_graph: &ReferenceGraph,
        _causal_graph: &CausalGraph,
    ) -> DistillationResult {
        // TODO: In real implementation, would iterate over epoch's distinctions
        // For now, placeholder

        let preserved = 0;
        let archived = 0;

        // Archive the epoch
        deep.archive_epoch(
            format!("epoch_{}", epoch_num),
            0, // Would be actual count
            0, // Would be actual size
        );

        self.distinctions_preserved
            .fetch_add(preserved, Ordering::Relaxed);
        self.distinctions_archived
            .fetch_add(archived, Ordering::Relaxed);

        DistillationResult {
            distinctions_preserved: preserved,
            distinctions_archived: archived,
        }
    }

    /// Get the fitness threshold.
    pub fn fitness_threshold(&self) -> usize {
        self.config.fitness_threshold
    }

    /// Get interval.
    pub fn interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.config.interval_secs)
    }

    /// Get statistics.
    pub fn stats(&self) -> DistillationStats {
        DistillationStats {
            distinctions_evaluated: self.distinctions_evaluated.load(Ordering::Relaxed),
            distinctions_preserved: self.distinctions_preserved.load(Ordering::Relaxed),
            distinctions_archived: self.distinctions_archived.load(Ordering::Relaxed),
            preservation_rate: self.preservation_rate(),
        }
    }

    /// Calculate preservation rate (0.0 to 1.0).
    fn preservation_rate(&self) -> f64 {
        let evaluated = self.distinctions_evaluated.load(Ordering::Relaxed);
        let preserved = self.distinctions_preserved.load(Ordering::Relaxed);

        if evaluated == 0 {
            0.0
        } else {
            preserved as f64 / evaluated as f64
        }
    }
}

impl Default for DistillationProcess {
    fn default() -> Self {
        Self::new()
    }
}

/// Classification result.
#[derive(Debug, Clone)]
pub struct Classification {
    pub fit: Vec<Fitness>,
    pub unfit: Vec<Fitness>,
}

/// Distillation result.
#[derive(Debug, Clone)]
pub struct DistillationResult {
    pub distinctions_preserved: u64,
    pub distinctions_archived: u64,
}

/// Distillation statistics.
#[derive(Debug, Clone)]
pub struct DistillationStats {
    pub distinctions_evaluated: u64,
    pub distinctions_preserved: u64,
    pub distinctions_archived: u64,
    pub preservation_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::causal_graph::CausalGraph;
    use crate::reference_graph::ReferenceGraph;

    #[test]
    fn test_calculate_fitness() {
        let distillation = DistillationProcess::new();
        let ref_graph = ReferenceGraph::new();
        let causal_graph = CausalGraph::new();

        // Add node with no references
        ref_graph.add_node("test".to_string());
        causal_graph.add_node("test".to_string());

        let fitness = distillation.calculate_fitness(
            "test",
            &ref_graph,
            &causal_graph,
            Utc::now(), // Now, so no age penalty
        );

        assert_eq!(fitness.reference_count, 0);
        assert_eq!(fitness.total_score, 0);
    }

    #[test]
    fn test_fitness_with_references() {
        let distillation = DistillationProcess::new();
        let ref_graph = ReferenceGraph::new();
        let causal_graph = CausalGraph::new();

        // Setup: test2 references test1
        ref_graph.add_node("test1".to_string());
        ref_graph.add_node("test2".to_string());
        ref_graph.add_reference("test2".to_string(), "test1".to_string());

        causal_graph.add_node("test1".to_string());

        let fitness =
            distillation.calculate_fitness("test1", &ref_graph, &causal_graph, Utc::now());

        assert_eq!(fitness.reference_count, 1);
        assert_eq!(fitness.total_score, 1);
    }

    #[test]
    fn test_classify_distinctions() {
        let distillation = DistillationProcess::with_config(DistillationConfig {
            fitness_threshold: 2,
            ..Default::default()
        });
        let ref_graph = ReferenceGraph::new();
        let causal_graph = CausalGraph::new();

        // Setup distinctions
        let distinctions = vec![
            ("high_fit".to_string(), Utc::now()),
            ("low_fit".to_string(), Utc::now()),
        ];

        // Add references: high_fit has 3, low_fit has 0
        ref_graph.add_node("high_fit".to_string());
        ref_graph.add_node("low_fit".to_string());
        ref_graph.add_node("ref1".to_string());
        ref_graph.add_node("ref2".to_string());
        ref_graph.add_node("ref3".to_string());
        ref_graph.add_reference("ref1".to_string(), "high_fit".to_string());
        ref_graph.add_reference("ref2".to_string(), "high_fit".to_string());
        ref_graph.add_reference("ref3".to_string(), "high_fit".to_string());

        causal_graph.add_node("high_fit".to_string());
        causal_graph.add_node("low_fit".to_string());

        let classification =
            distillation.classify_distinctions(&distinctions, &ref_graph, &causal_graph);

        assert_eq!(classification.fit.len(), 1);
        assert_eq!(classification.unfit.len(), 1);
        assert_eq!(classification.fit[0].distinction_id, "high_fit");
    }

    #[test]
    fn test_age_penalty() {
        let distillation = DistillationProcess::new();
        let ref_graph = ReferenceGraph::new();
        let causal_graph = CausalGraph::new();

        ref_graph.add_node("old".to_string());
        causal_graph.add_node("old".to_string());

        // Created 30 days ago
        let old_timestamp = Utc::now() - Duration::days(30);

        let fitness =
            distillation.calculate_fitness("old", &ref_graph, &causal_graph, old_timestamp);

        // Should have negative score due to age (30 - 7 = 23 days over threshold)
        assert!(fitness.total_score < 0);
    }

    #[test]
    fn test_stats() {
        let distillation = DistillationProcess::new();

        // Simulate some evaluations
        let ref_graph = ReferenceGraph::new();
        let causal_graph = CausalGraph::new();

        for i in 0..10 {
            let id = format!("test{}", i);
            ref_graph.add_node(id.clone());
            causal_graph.add_node(id);
        }

        let distinctions: Vec<_> = (0..10)
            .map(|i| (format!("test{}", i), Utc::now()))
            .collect();

        let _ = distillation.classify_distinctions(&distinctions, &ref_graph, &causal_graph);

        let stats = distillation.stats();
        assert_eq!(stats.distinctions_evaluated, 10);
    }
}
