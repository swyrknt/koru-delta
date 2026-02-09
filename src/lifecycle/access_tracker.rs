/// Access Pattern Tracking
///
/// Records detailed access patterns for each distinction including:
/// - Frequency (how often accessed)
/// - Recency (when last accessed)
/// - Time of day patterns (circadian access patterns)
/// - Access sequences (what comes before/after)
/// - Access duration/context
use chrono::{DateTime, Datelike, Duration, NaiveTime, Timelike, Utc};
use dashmap::DashMap;
use std::collections::VecDeque;

use crate::causal_graph::DistinctionId;
use crate::types::FullKey;

/// Tracks access patterns for all distinctions
pub struct AccessTracker {
    /// Access patterns by distinction ID
    patterns: DashMap<DistinctionId, AccessPattern>,

    /// Recent access sequence (for sequence analysis)
    recent_sequence: std::sync::Mutex<VecDeque<(DistinctionId, DateTime<Utc>)>>,

    /// Maximum sequence length to track
    max_sequence_length: usize,

    /// Hourly access distribution (0-23 hours)
    hourly_distribution: DashMap<u8, u64>,

    /// Day of week distribution (0-6, where 0 = Monday)
    weekday_distribution: DashMap<u8, u64>,
}

/// Pattern of access for a single distinction
#[derive(Debug, Clone)]
pub struct AccessPattern {
    /// The distinction ID
    pub distinction_id: DistinctionId,

    /// Full key for this distinction
    pub key: FullKey,

    /// Number of times accessed
    pub access_count: u64,

    /// When first accessed
    pub first_accessed: Option<DateTime<Utc>>,

    /// When last accessed
    pub last_accessed: Option<DateTime<Utc>>,

    /// Average time between accesses (for regularity analysis)
    pub avg_interval_secs: f64,

    /// Hourly access counts (0-23)
    pub hourly_counts: [u64; 24],

    /// Day of week counts (0-6)
    pub weekday_counts: [u64; 7],

    /// What typically comes before this distinction
    pub predecessors: Vec<DistinctionId>,

    /// What typically comes after this distinction
    pub successors: Vec<DistinctionId>,

    /// Total time spent (if duration tracking enabled)
    pub total_duration_ms: u64,
}

impl AccessTracker {
    /// Create a new access tracker
    pub fn new() -> Self {
        Self::with_capacity(10000)
    }

    /// Create with specified initial capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            patterns: DashMap::with_capacity(capacity),
            recent_sequence: std::sync::Mutex::new(VecDeque::with_capacity(100)),
            max_sequence_length: 100,
            hourly_distribution: DashMap::new(),
            weekday_distribution: DashMap::new(),
        }
    }

    /// Record an access
    pub fn record_access(&self, key: FullKey, distinction_id: DistinctionId) {
        let now = Utc::now();
        let hour = now.hour() as u8;
        let weekday = now.weekday().num_days_from_monday() as u8;

        // Update hourly distribution
        *self.hourly_distribution.entry(hour).or_insert(0) += 1;

        // Update weekday distribution
        *self.weekday_distribution.entry(weekday).or_insert(0) += 1;

        // Update pattern for this distinction
        let mut pattern = self
            .patterns
            .entry(distinction_id.clone())
            .or_insert_with(|| AccessPattern::new(distinction_id.clone(), key.clone()));

        // Calculate interval from last access
        if let Some(last) = pattern.last_accessed {
            let interval = now.signed_duration_since(last);
            let interval_secs = interval.num_seconds() as f64;

            // Update rolling average
            let n = pattern.access_count as f64;
            pattern.avg_interval_secs = (pattern.avg_interval_secs * n + interval_secs) / (n + 1.0);
        }

        // Update basic stats
        pattern.access_count += 1;
        pattern.last_accessed = Some(now);
        if pattern.first_accessed.is_none() {
            pattern.first_accessed = Some(now);
        }

        // Update hourly counts
        pattern.hourly_counts[hour as usize] += 1;

        // Update weekday counts
        pattern.weekday_counts[weekday as usize] += 1;

        // Update sequence
        self.update_sequence(distinction_id.clone(), now);

        // Drop the write lock before calling update_related
        drop(pattern);

        // Update predecessor/successor relationships
        self.update_related(distinction_id);
    }

    /// Record access with duration
    pub fn record_access_with_duration(
        &self,
        key: FullKey,
        distinction_id: DistinctionId,
        duration_ms: u64,
    ) {
        self.record_access(key, distinction_id.clone());

        if let Some(mut pattern) = self.patterns.get_mut(&distinction_id) {
            pattern.total_duration_ms += duration_ms;
        }
    }

    /// Get pattern for a distinction
    pub fn get_pattern(&self, distinction_id: &DistinctionId) -> Option<AccessPattern> {
        self.patterns.get(distinction_id).map(|p| p.clone())
    }

    /// Get all patterns
    pub fn patterns(&self) -> dashmap::iter::Iter<'_, DistinctionId, AccessPattern> {
        self.patterns.iter()
    }

    /// Get the most frequently accessed distinctions
    pub fn most_frequent(&self, limit: usize) -> Vec<(DistinctionId, u64)> {
        let mut items: Vec<_> = self
            .patterns
            .iter()
            .map(|e| (e.key().clone(), e.access_count))
            .collect();

        items.sort_by(|a, b| b.1.cmp(&a.1));
        items.into_iter().take(limit).collect()
    }

    /// Get the most recently accessed distinctions
    pub fn most_recent(&self, limit: usize) -> Vec<(DistinctionId, DateTime<Utc>)> {
        let mut items: Vec<_> = self
            .patterns
            .iter()
            .filter_map(|e| e.last_accessed.map(|t| (e.key().clone(), t)))
            .collect();

        items.sort_by(|a, b| b.1.cmp(&a.1));
        items.into_iter().take(limit).collect()
    }

    /// Get distinctions accessed at a specific hour (for time-based queries)
    pub fn accessed_at_hour(&self, hour: u8) -> Vec<DistinctionId> {
        self.patterns
            .iter()
            .filter(|e| e.hourly_counts[hour as usize % 24] > 0)
            .map(|e| e.key().clone())
            .collect()
    }

    /// Get the "peak hour" for a distinction (when it's most accessed)
    pub fn peak_hour(&self, distinction_id: &DistinctionId) -> Option<u8> {
        self.patterns.get(distinction_id).map(|p| {
            p.hourly_counts
                .iter()
                .enumerate()
                .max_by_key(|(_, count)| *count)
                .map(|(hour, _)| hour as u8)
                .unwrap_or(0)
        })
    }

    /// Predict next access time based on pattern
    pub fn predict_next_access(&self, distinction_id: &DistinctionId) -> Option<DateTime<Utc>> {
        let pattern = self.patterns.get(distinction_id)?;

        if pattern.avg_interval_secs <= 0.0 || pattern.last_accessed.is_none() {
            return None;
        }

        let last = pattern.last_accessed.unwrap();
        let interval = Duration::seconds(pattern.avg_interval_secs as i64);

        Some(last + interval)
    }

    /// Get total tracked distinctions
    pub fn len(&self) -> usize {
        self.patterns.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }

    /// Get hourly distribution across all distinctions
    pub fn global_hourly_distribution(&self) -> [u64; 24] {
        let mut dist = [0u64; 24];
        for (hour, item) in dist.iter_mut().enumerate() {
            *item = self
                .hourly_distribution
                .get(&(hour as u8))
                .map(|v| *v)
                .unwrap_or(0);
        }
        dist
    }

    /// Get statistics
    pub fn stats(&self) -> AccessTrackerStats {
        let total_accesses: u64 = self.patterns.iter().map(|p| p.access_count).sum();

        let unique_distinctions = self.patterns.len() as u64;

        AccessTrackerStats {
            unique_distinctions,
            total_accesses,
            avg_accesses_per_distinction: if unique_distinctions > 0 {
                total_accesses as f64 / unique_distinctions as f64
            } else {
                0.0
            },
        }
    }

    /// Update the recent access sequence
    fn update_sequence(&self, distinction_id: DistinctionId, timestamp: DateTime<Utc>) {
        if let Ok(mut seq) = self.recent_sequence.lock() {
            seq.push_back((distinction_id, timestamp));

            while seq.len() > self.max_sequence_length {
                seq.pop_front();
            }
        }
    }

    /// Update predecessor/successor relationships
    fn update_related(&self, distinction_id: DistinctionId) {
        let recent = {
            if let Ok(seq) = self.recent_sequence.lock() {
                seq.iter().cloned().collect::<Vec<_>>()
            } else {
                Vec::new()
            }
        };

        // Find this distinction in the sequence
        if let Some(pos) = recent.iter().position(|(id, _)| id == &distinction_id) {
            // Get predecessor (if not first)
            if pos > 0 {
                let predecessor = recent[pos - 1].0.clone();
                if let Some(mut pattern) = self.patterns.get_mut(&distinction_id) {
                    if !pattern.predecessors.contains(&predecessor) {
                        pattern.predecessors.push(predecessor);
                        // Keep only most recent 5
                        if pattern.predecessors.len() > 5 {
                            pattern.predecessors.remove(0);
                        }
                    }
                }
            }

            // Get successor (if not last)
            if pos < recent.len() - 1 {
                let successor = recent[pos + 1].0.clone();
                if let Some(mut pattern) = self.patterns.get_mut(&distinction_id) {
                    if !pattern.successors.contains(&successor) {
                        pattern.successors.push(successor);
                        // Keep only most recent 5
                        if pattern.successors.len() > 5 {
                            pattern.successors.remove(0);
                        }
                    }
                }
            }
        }
    }
}

impl Default for AccessTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl AccessPattern {
    /// Create a new access pattern
    fn new(distinction_id: DistinctionId, key: FullKey) -> Self {
        Self {
            distinction_id,
            key,
            access_count: 0,
            first_accessed: None,
            last_accessed: None,
            avg_interval_secs: 0.0,
            hourly_counts: [0; 24],
            weekday_counts: [0; 7],
            predecessors: Vec::new(),
            successors: Vec::new(),
            total_duration_ms: 0,
        }
    }

    /// Calculate access regularity (how consistent are intervals)
    /// Returns 0.0 (irregular) to 1.0 (very regular)
    pub fn regularity(&self) -> f64 {
        if self.access_count < 3 {
            return 0.0;
        }

        // Simple heuristic: high count + consistent hour = regular
        let peak_hour_count = self.hourly_counts.iter().max().copied().unwrap_or(0);
        let hour_concentration = peak_hour_count as f64 / self.access_count as f64;

        // Also consider interval consistency (would need variance calculation)
        // For now, use a simple formula
        let count_factor = (self.access_count as f64 / 10.0).min(1.0);

        hour_concentration * count_factor
    }

    /// Get average access time of day
    pub fn average_access_time(&self) -> Option<NaiveTime> {
        if self.access_count == 0 {
            return None;
        }

        // Weighted average of hours
        let total_weight: u64 = self.hourly_counts.iter().sum();
        if total_weight == 0 {
            return None;
        }

        let weighted_hour: f64 = self
            .hourly_counts
            .iter()
            .enumerate()
            .map(|(hour, count)| hour as f64 * *count as f64)
            .sum::<f64>()
            / total_weight as f64;

        Some(
            NaiveTime::from_hms_opt(weighted_hour as u32, 0, 0)
                .unwrap_or_else(|| NaiveTime::from_hms_opt(0, 0, 0).expect("valid time")),
        )
    }

    /// Get average duration per access (ms)
    pub fn avg_duration_ms(&self) -> u64 {
        if self.access_count == 0 {
            0
        } else {
            self.total_duration_ms / self.access_count
        }
    }
}

/// Access tracker statistics
#[derive(Debug, Clone)]
pub struct AccessTrackerStats {
    pub unique_distinctions: u64,
    pub total_accesses: u64,
    pub avg_accesses_per_distinction: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_access_tracker_new() {
        let tracker = AccessTracker::new();
        assert!(tracker.is_empty());
        assert_eq!(tracker.len(), 0);
    }

    #[test]
    fn test_record_access() {
        let tracker = AccessTracker::new();
        let key = FullKey::new("test", "key1");
        let id = "dist1".to_string();

        tracker.record_access(key, id.clone());

        assert_eq!(tracker.len(), 1);

        let pattern = tracker.get_pattern(&id).unwrap();
        assert_eq!(pattern.access_count, 1);
        assert!(pattern.last_accessed.is_some());
        assert!(pattern.first_accessed.is_some());
    }

    #[test]
    fn test_multiple_accesses() {
        let tracker = AccessTracker::new();
        let key = FullKey::new("test", "key1");
        let id = "dist1".to_string();

        for _ in 0..5 {
            tracker.record_access(key.clone(), id.clone());
        }

        let pattern = tracker.get_pattern(&id).unwrap();
        assert_eq!(pattern.access_count, 5);
    }

    #[test]
    fn test_most_frequent() {
        let tracker = AccessTracker::new();

        // dist1: 5 accesses
        for i in 0..5 {
            tracker.record_access(
                FullKey::new("test", format!("key{}", i)),
                "dist1".to_string(),
            );
        }

        // dist2: 3 accesses
        for i in 0..3 {
            tracker.record_access(
                FullKey::new("test", format!("key{}", i + 10)),
                "dist2".to_string(),
            );
        }

        // dist3: 1 access
        tracker.record_access(FullKey::new("test", "key20"), "dist3".to_string());

        let most_frequent = tracker.most_frequent(2);
        assert_eq!(most_frequent.len(), 2);
        assert_eq!(most_frequent[0].0, "dist1");
        assert_eq!(most_frequent[0].1, 5);
    }

    #[test]
    fn test_peak_hour() {
        let tracker = AccessTracker::new();
        let key = FullKey::new("test", "key1");
        let id = "dist1".to_string();

        // Record access
        tracker.record_access(key, id.clone());

        let peak = tracker.peak_hour(&id);
        assert!(peak.is_some());
    }

    #[test]
    fn test_access_pattern_regularity() {
        let pattern = AccessPattern {
            distinction_id: "test".to_string(),
            key: FullKey::new("test", "key"),
            access_count: 10,
            first_accessed: Some(Utc::now()),
            last_accessed: Some(Utc::now()),
            avg_interval_secs: 3600.0,
            hourly_counts: [
                10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
            weekday_counts: [0; 7],
            predecessors: Vec::new(),
            successors: Vec::new(),
            total_duration_ms: 0,
        };

        // All accesses at hour 0, should be regular
        let regularity = pattern.regularity();
        assert!(regularity > 0.5);
    }

    #[test]
    fn test_stats() {
        let tracker = AccessTracker::new();

        // Add some accesses
        for i in 0..3 {
            tracker.record_access(
                FullKey::new("test", format!("key{}", i)),
                format!("dist{}", i),
            );
        }

        // Add multiple to first (3 more to make 4 total for dist0)
        for _ in 0..3 {
            tracker.record_access(FullKey::new("test", "key0"), "dist0".to_string());
        }

        let stats = tracker.stats();
        assert_eq!(stats.unique_distinctions, 3);
        assert_eq!(stats.total_accesses, 6);
        assert_eq!(stats.avg_accesses_per_distinction, 2.0);
    }
}
