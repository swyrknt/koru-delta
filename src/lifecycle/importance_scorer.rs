/// ML-Based Importance Scoring
///
/// Predicts the future value/importance of distinctions based on:
/// - Access patterns (frequency, recency, regularity)
/// - Causal relationships (upstream/downstream importance)
/// - Content similarity to important items
/// - Temporal patterns (time of day, day of week)
///
/// Uses a lightweight "ML" model (really just weighted heuristics + learned weights)
/// that can be updated based on actual access patterns.
use chrono::Utc;
use std::collections::HashMap;

use chrono::Timelike;

use crate::causal_graph::DistinctionId;
use crate::lifecycle::access_tracker::{AccessPattern, AccessTracker};
use crate::lifecycle::ScoreFactor;

/// Importance score for a distinction
#[derive(Debug, Clone)]
pub struct ImportanceScore {
    /// The distinction being scored
    pub distinction_id: DistinctionId,

    /// Overall importance score (0.0 - 1.0)
    /// Higher = more important, should stay in faster tiers
    pub score: f32,

    /// Confidence in this score (0.0 - 1.0)
    /// Based on amount of data available
    pub confidence: f32,

    /// Individual factors contributing to score
    pub factors: Vec<ScoreFactor>,
}

impl ImportanceScore {
    /// Create a new importance score
    pub fn new(distinction_id: DistinctionId, score: f32, confidence: f32) -> Self {
        Self {
            distinction_id,
            score,
            confidence,
            factors: Vec::new(),
        }
    }

    /// Add a factor
    pub fn with_factor(mut self, factor: ScoreFactor) -> Self {
        self.factors.push(factor);
        self
    }

    /// Weighted score considering confidence
    /// Low confidence scores are dampened
    pub fn weighted_score(&self) -> f32 {
        self.score * self.confidence
    }

    /// Check if this distinction should stay in Hot memory
    pub fn should_stay_hot(&self, threshold: f32) -> bool {
        self.score >= threshold
    }

    /// Check if this distinction should be promoted to Hot
    pub fn should_promote_to_hot(&self, threshold: f32) -> bool {
        self.score >= threshold && self.confidence > 0.5
    }

    /// Check if this distinction can be demoted to Warm
    pub fn can_demote_to_warm(&self, threshold: f32) -> bool {
        self.score < threshold
    }

    /// Check if this distinction can be moved to Cold
    pub fn can_move_to_cold(&self, threshold: f32) -> bool {
        self.score < threshold && self.confidence > 0.3
    }
}

/// ML-based importance model
///
/// This is a lightweight "neural network" (really just linear regression with learned weights)
/// that predicts importance based on features extracted from access patterns.
#[derive(Debug)]
pub struct ImportanceModel {
    /// Feature weights (learned)
    weights: ModelWeights,

    /// Learning rate for online updates
    learning_rate: f32,

    /// Number of predictions made
    prediction_count: u64,

    /// Number of predictions that were "correct" (item was accessed again)
    correct_predictions: u64,
}

/// Weights for different features
#[derive(Debug, Clone, Copy)]
struct ModelWeights {
    /// Recency weight
    recency: f32,
    /// Frequency weight
    frequency: f32,
    /// Regularity weight
    regularity: f32,
    /// Time-of-day relevance weight
    time_of_day: f32,
    /// Causal centrality weight
    causal: f32,
    /// Bias term
    bias: f32,
}

impl Default for ModelWeights {
    fn default() -> Self {
        // Initial weights based on intuition
        Self {
            recency: 0.35,
            frequency: 0.25,
            regularity: 0.15,
            time_of_day: 0.10,
            causal: 0.10,
            bias: 0.05,
        }
    }
}

impl ImportanceModel {
    /// Create a new importance model
    pub fn new() -> Self {
        Self {
            weights: ModelWeights::default(),
            learning_rate: 0.01,
            prediction_count: 0,
            correct_predictions: 0,
        }
    }

    /// Predict importance for all tracked distinctions
    pub fn predict_all(&self, tracker: &AccessTracker) -> HashMap<DistinctionId, ImportanceScore> {
        let mut scores = HashMap::new();
        let now = Utc::now();

        for entry in tracker.patterns() {
            let id = entry.key().clone();
            let pattern = entry.value();
            let score = self.predict_single(pattern, now.timestamp() as f64);
            scores.insert(id, score);
        }

        scores
    }

    /// Predict importance for a single pattern
    fn predict_single(&self, pattern: &AccessPattern, now_secs: f64) -> ImportanceScore {
        // Extract features
        let recency = self.calculate_recency_feature(pattern, now_secs);
        let frequency = self.calculate_frequency_feature(pattern);
        let regularity = self.calculate_regularity_feature(pattern);
        let time_of_day = self.calculate_time_of_day_feature(pattern);
        let causal = self.calculate_causal_feature(pattern);

        // Calculate weighted sum
        let raw_score = self.weights.recency * recency
            + self.weights.frequency * frequency
            + self.weights.regularity * regularity
            + self.weights.time_of_day * time_of_day
            + self.weights.causal * causal
            + self.weights.bias;

        // Sigmoid to bound between 0 and 1
        let score = sigmoid(raw_score);

        // Calculate confidence based on data amount
        let confidence = self.calculate_confidence(pattern);

        ImportanceScore {
            distinction_id: pattern.distinction_id.clone(),
            score,
            confidence,
            factors: vec![
                ScoreFactor::Recency(recency),
                ScoreFactor::Frequency(frequency),
                ScoreFactor::TimeOfDay(time_of_day),
                ScoreFactor::SequenceContext(regularity),
                ScoreFactor::PredictedFutureValue(score),
            ],
        }
    }

    /// Calculate recency feature (0.0 - 1.0)
    /// Higher = accessed more recently
    fn calculate_recency_feature(&self, pattern: &AccessPattern, now_secs: f64) -> f32 {
        if let Some(last) = pattern.last_accessed {
            let age_secs = now_secs - last.timestamp() as f64;
            let age_days = age_secs / 86400.0;

            // Exponential decay: 1.0 at 0 days, ~0.37 at 1 day, ~0.14 at 2 days
            (-age_days / 1.0).exp() as f32
        } else {
            0.0
        }
    }

    /// Calculate frequency feature (0.0 - 1.0)
    /// Higher = accessed more frequently
    fn calculate_frequency_feature(&self, pattern: &AccessPattern) -> f32 {
        // Log scale to prevent dominance by extremely frequent items
        let log_count = (pattern.access_count as f32 + 1.0).ln();
        (log_count / 5.0).min(1.0) // Normalize to ~100 accesses = 1.0
    }

    /// Calculate regularity feature (0.0 - 1.0)
    /// Higher = accessed at regular intervals
    fn calculate_regularity_feature(&self, pattern: &AccessPattern) -> f32 {
        pattern.regularity() as f32
    }

    /// Calculate time-of-day feature (0.0 - 1.0)
    /// Higher = currently in peak access time
    fn calculate_time_of_day_feature(&self, pattern: &AccessPattern) -> f32 {
        let now = Utc::now();
        let current_hour = now.hour() as usize;

        // Get peak hour
        let peak_hour = pattern
            .hourly_counts
            .iter()
            .enumerate()
            .max_by_key(|(_, count)| *count)
            .map(|(hour, _)| hour)
            .unwrap_or(0);

        // Calculate distance from peak hour (circular)
        let hour_diff = if current_hour > peak_hour {
            (current_hour - peak_hour).min(24 - (current_hour - peak_hour))
        } else {
            (peak_hour - current_hour).min(24 - (peak_hour - current_hour))
        };

        // Score: 1.0 at peak, decays to 0.5 at ±6 hours
        let score = 1.0 - (hour_diff as f32 / 12.0).min(1.0) * 0.5;

        // Weight by how concentrated accesses are at peak
        let total: u64 = pattern.hourly_counts.iter().sum();
        let peak_count = pattern.hourly_counts[peak_hour];
        let concentration = if total > 0 {
            peak_count as f32 / total as f32
        } else {
            0.0
        };

        score * concentration
    }

    /// Calculate causal centrality feature (0.0 - 1.0)
    /// Higher = more connected in causal graph
    fn calculate_causal_feature(&self, pattern: &AccessPattern) -> f32 {
        // Simple heuristic: more predecessors/successors = more central
        let connections = pattern.predecessors.len() + pattern.successors.len();
        (connections as f32 / 10.0).min(1.0)
    }

    /// Calculate confidence based on data amount
    fn calculate_confidence(&self, pattern: &AccessPattern) -> f32 {
        // More accesses = higher confidence, capped at 1.0
        let base_confidence = (pattern.access_count as f32 / 10.0).min(1.0);

        // Also consider time span (longer = more confidence)
        let time_span_factor =
            if let (Some(first), Some(last)) = (pattern.first_accessed, pattern.last_accessed) {
                let span_days = last.signed_duration_since(first).num_days() as f32;
                (span_days / 7.0).min(1.0) // Week of data = full confidence
            } else {
                0.0
            };

        (base_confidence * 0.7 + time_span_factor * 0.3).min(1.0)
    }

    /// Update model based on actual outcomes (online learning)
    ///
    /// Call this when we verify if a prediction was correct
    /// (e.g., an item we predicted would be important was actually accessed)
    pub fn update_weights(&mut self, _prediction: &ImportanceScore, was_accessed: bool) {
        self.prediction_count += 1;

        if was_accessed {
            self.correct_predictions += 1;
        }

        // Simple online update: adjust weights based on outcome
        // In a real implementation, this would use gradient descent
        let error = if was_accessed { 1.0 } else { 0.0 } - 0.5;

        // Update weights slightly
        self.weights.recency += self.learning_rate * error * 0.1;
        self.weights.frequency += self.learning_rate * error * 0.1;

        // Clamp weights to reasonable range
        self.clamp_weights();
    }

    /// Clamp weights to prevent divergence
    fn clamp_weights(&mut self) {
        let min_weight = 0.0;
        let max_weight = 1.0;

        self.weights.recency = self.weights.recency.clamp(min_weight, max_weight);
        self.weights.frequency = self.weights.frequency.clamp(min_weight, max_weight);
        self.weights.regularity = self.weights.regularity.clamp(min_weight, max_weight);
        self.weights.time_of_day = self.weights.time_of_day.clamp(min_weight, max_weight);
        self.weights.causal = self.weights.causal.clamp(min_weight, max_weight);
        self.weights.bias = self.weights.bias.clamp(-0.5, 0.5);
    }

    /// Get model accuracy
    pub fn accuracy(&self) -> f64 {
        if self.prediction_count == 0 {
            0.0
        } else {
            self.correct_predictions as f64 / self.prediction_count as f64
        }
    }

    /// Get model weights (for inspection/debugging)
    pub fn weights(&self) -> HashMap<String, f32> {
        let mut weights = HashMap::new();
        weights.insert("recency".to_string(), self.weights.recency);
        weights.insert("frequency".to_string(), self.weights.frequency);
        weights.insert("regularity".to_string(), self.weights.regularity);
        weights.insert("time_of_day".to_string(), self.weights.time_of_day);
        weights.insert("causal".to_string(), self.weights.causal);
        weights.insert("bias".to_string(), self.weights.bias);
        weights
    }
}

impl Default for ImportanceModel {
    fn default() -> Self {
        Self::new()
    }
}

/// Sigmoid function to bound values between 0 and 1
fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_pattern(access_count: u64, last_accessed_minutes_ago: i64) -> AccessPattern {
        let last_accessed = if last_accessed_minutes_ago >= 0 {
            Some(Utc::now() - chrono::Duration::minutes(last_accessed_minutes_ago))
        } else {
            None
        };

        AccessPattern {
            distinction_id: "test".to_string(),
            key: crate::types::FullKey::new("test", "key"),
            access_count,
            first_accessed: Some(Utc::now() - chrono::Duration::days(7)),
            last_accessed,
            avg_interval_secs: 3600.0,
            hourly_counts: [
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 10, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ], // Peak at noon
            weekday_counts: [0; 7],
            predecessors: Vec::new(),
            successors: Vec::new(),
            total_duration_ms: 0,
        }
    }

    #[test]
    fn test_importance_model_new() {
        let model = ImportanceModel::new();
        let weights = model.weights();
        assert!(weights.contains_key("recency"));
        assert!(weights.contains_key("frequency"));
        assert_eq!(model.accuracy(), 0.0);
    }

    #[test]
    fn test_recency_feature() {
        let model = ImportanceModel::new();
        let now = Utc::now().timestamp() as f64;

        // Recent access
        let recent_pattern = create_test_pattern(1, 5); // 5 minutes ago
        let recent_score = model.calculate_recency_feature(&recent_pattern, now);
        assert!(
            recent_score > 0.9,
            "Recent access should have high recency score"
        );

        // Old access
        let old_pattern = create_test_pattern(1, 60 * 24 * 2); // 2 days ago
        let old_score = model.calculate_recency_feature(&old_pattern, now);
        assert!(old_score < 0.2, "Old access should have low recency score");
    }

    #[test]
    fn test_frequency_feature() {
        let model = ImportanceModel::new();

        let low_freq = create_test_pattern(1, 0);
        let low_score = model.calculate_frequency_feature(&low_freq);

        let high_freq = create_test_pattern(100, 0);
        let high_score = model.calculate_frequency_feature(&high_freq);

        assert!(
            high_score > low_score,
            "Higher frequency should have higher score"
        );
        assert!(high_score <= 1.0, "Score should be bounded");
    }

    #[test]
    fn test_importance_score_thresholds() {
        let score = ImportanceScore::new("test".to_string(), 0.8, 0.9);

        assert!(score.should_stay_hot(0.7));
        assert!(score.should_promote_to_hot(0.7));
        assert!(!score.can_demote_to_warm(0.7));
    }

    #[test]
    fn test_sigmoid() {
        assert!((sigmoid(0.0) - 0.5).abs() < 0.01);
        assert!(sigmoid(5.0) > 0.99);
        assert!(sigmoid(-5.0) < 0.01);
    }

    #[test]
    fn test_model_update() {
        let mut model = ImportanceModel::new();
        let initial_weights = model.weights;

        let score = ImportanceScore::new("test".to_string(), 0.8, 0.9);
        model.update_weights(&score, true);

        // Weights should have changed slightly
        let updated_weights = model.weights;
        assert_ne!(initial_weights.recency, updated_weights.recency);
    }
}
