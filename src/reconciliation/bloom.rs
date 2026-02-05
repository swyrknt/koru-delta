/// Bloom Filter for Probabilistic Set Membership.
///
/// Bloom filters provide space-efficient probabilistic set membership testing.
/// They can tell you:
/// - "Definitely not in set" (no false negatives)
/// - "Probably in set" (some false positives)
///
/// This is perfect for sync protocols—if a filter says "not in set", we know
/// for sure we need to sync that distinction. False positives just mean we
/// might sync something already present (harmless).
///
/// ## Example
///
/// ```rust
/// use koru_delta::reconciliation::BloomFilter;
///
/// let mut filter = BloomFilter::new(1000, 0.01); // 1% false positive rate
/// filter.insert("distinction_123");
///
/// assert!(filter.might_contain("distinction_123")); // Probably true (was inserted)
/// assert!(filter.definitely_not_contain("distinction_456")); // Definitely not in set
/// ```
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Bloom filter for distinction set membership.
#[derive(Debug, Clone)]
pub struct BloomFilter {
    /// Bit array.
    bits: Vec<bool>,
    /// Number of hash functions.
    k: usize,
    /// Size of bit array.
    m: usize,
    /// Number of items inserted.
    n: usize,
}

impl BloomFilter {
    /// Create a new Bloom filter.
    ///
    /// # Arguments
    ///
    /// * `expected_items` - Expected number of items to insert
    /// * `false_positive_rate` - Desired false positive probability (0.0-1.0)
    ///
    /// # Example
    ///
    /// ```rust
    /// use koru_delta::reconciliation::BloomFilter;
    ///
    /// let filter = BloomFilter::new(10000, 0.01); // 1% FP rate for 10K items
    /// ```
    pub fn new(expected_items: usize, false_positive_rate: f64) -> Self {
        // Calculate optimal bit array size: m = -n * ln(p) / (ln(2)^2)
        let m = ((-1.0 * expected_items as f64 * false_positive_rate.ln())
            / (2.0_f64.ln().powi(2)))
            .ceil() as usize;

        // Calculate optimal number of hash functions: k = m/n * ln(2)
        let k = ((m as f64 / expected_items as f64) * 2.0_f64.ln()).ceil() as usize;

        Self {
            bits: vec![false; m.max(1)],
            k: k.max(1),
            m: m.max(1),
            n: 0,
        }
    }

    /// Create a Bloom filter with specific dimensions.
    ///
    /// For advanced use when you want precise control over size.
    pub fn with_dimensions(m: usize, k: usize) -> Self {
        Self {
            bits: vec![false; m],
            k,
            m,
            n: 0,
        }
    }

    /// Insert an item into the filter.
    pub fn insert(&mut self, item: &str) {
        for i in 0..self.k {
            let idx = self.hash(item, i);
            self.bits[idx] = true;
        }
        self.n += 1;
    }

    /// Check if an item might be in the set.
    ///
    /// Returns `true` if the item might be in the set (could be false positive).
    /// Returns `false` if definitely not in the set (no false negatives).
    pub fn might_contain(&self, item: &str) -> bool {
        for i in 0..self.k {
            let idx = self.hash(item, i);
            if !self.bits[idx] {
                return false;
            }
        }
        true
    }

    /// Check if an item is definitely NOT in the set.
    ///
    /// This is the more useful operation for sync—if true, we definitely
    /// need to send this distinction to the other node.
    pub fn definitely_not_contain(&self, item: &str) -> bool {
        !self.might_contain(item)
    }

    /// Get the number of items inserted.
    pub fn len(&self) -> usize {
        self.n
    }

    /// Check if filter is empty.
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// Get the size of the bit array in bytes.
    pub fn size_in_bytes(&self) -> usize {
        (self.m + 7) / 8 // Round up to nearest byte
    }

    /// Estimate current false positive rate.
    ///
    /// Formula: (1 - e^(-kn/m))^k
    pub fn current_false_positive_rate(&self) -> f64 {
        let exponent = -1.0 * self.k as f64 * self.n as f64 / self.m as f64;
        (1.0 - exponent.exp()).powi(self.k as i32)
    }

    /// Clear all items from the filter.
    pub fn clear(&mut self) {
        self.bits.fill(false);
        self.n = 0;
    }

    /// Hash an item with a seed.
    fn hash(&self, item: &str, seed: usize) -> usize {
        let mut hasher = DefaultHasher::new();
        item.hash(&mut hasher);
        seed.hash(&mut hasher);
        (hasher.finish() as usize) % self.m
    }
}

impl Default for BloomFilter {
    fn default() -> Self {
        Self::new(1000, 0.01)
    }
}

/// Bloom filter exchange protocol.
///
/// Efficient two-way set membership check.
#[derive(Debug, Clone)]
pub struct BloomExchange {
    /// Local filter (what we have).
    local: BloomFilter,
    /// Remote filter (what they have).
    remote: Option<BloomFilter>,
}

impl BloomExchange {
    /// Create a new exchange with our distinctions.
    pub fn new(distinctions: &[String], expected_count: usize, fp_rate: f64) -> Self {
        let mut local = BloomFilter::new(expected_count, fp_rate);
        for d in distinctions {
            local.insert(d);
        }

        Self {
            local,
            remote: None,
        }
    }

    /// Receive the remote filter.
    pub fn receive_remote(&mut self, remote: BloomFilter) {
        self.remote = Some(remote);
    }

    /// Find distinctions we have that they probably don't.
    pub fn find_missing_remote(&self) -> Vec<String> {
        // This would need to track actual distinctions
        // For now, return empty (needs integration with distinction storage)
        vec![]
    }

    /// Get our filter to send to remote.
    pub fn get_local_filter(&self) -> &BloomFilter {
        &self.local
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_filter() {
        let filter = BloomFilter::new(100, 0.01);
        assert!(filter.is_empty());
        assert_eq!(filter.len(), 0);
        assert!(!filter.might_contain("anything"));
    }

    #[test]
    fn test_insert_and_query() {
        let mut filter = BloomFilter::new(100, 0.01);
        filter.insert("distinction_1");
        filter.insert("distinction_2");

        assert!(filter.might_contain("distinction_1"));
        assert!(filter.might_contain("distinction_2"));
        assert!(!filter.might_contain("distinction_3"));
    }

    #[test]
    fn test_definitely_not_contain() {
        let mut filter = BloomFilter::new(100, 0.01);
        filter.insert("present");

        assert!(filter.definitely_not_contain("absent"));
        assert!(!filter.definitely_not_contain("present")); // Might contain
    }

    #[test]
    fn test_no_false_negatives() {
        let mut filter = BloomFilter::new(1000, 0.01);
        
        // Insert many items
        for i in 0..100 {
            filter.insert(&format!("item_{}", i));
        }

        // All inserted items should return true
        for i in 0..100 {
            assert!(
                filter.might_contain(&format!("item_{}", i)),
                "False negative for item_{}",
                i
            );
        }
    }

    #[test]
    fn test_false_positive_rate() {
        // With 1% target FP rate, most items not inserted should return false
        let mut filter = BloomFilter::new(10000, 0.01);
        
        for i in 0..1000 {
            filter.insert(&format!("item_{}", i));
        }

        // Check FP rate on items not inserted
        let mut false_positives = 0;
        for i in 1000..2000 {
            if filter.might_contain(&format!("item_{}", i)) {
                false_positives += 1;
            }
        }

        let fp_rate = false_positives as f64 / 1000.0;
        // Should be around 1%, allow some variance
        assert!(fp_rate < 0.05, "False positive rate too high: {}", fp_rate);
    }

    #[test]
    fn test_clear() {
        let mut filter = BloomFilter::new(100, 0.01);
        filter.insert("item");
        assert!(filter.might_contain("item"));

        filter.clear();
        assert!(filter.is_empty());
        assert!(!filter.might_contain("item"));
    }

    #[test]
    fn test_size_calculation() {
        // For 1000 items at 1% FP, should need ~9585 bits (~1.2KB)
        let filter = BloomFilter::new(1000, 0.01);
        let size = filter.size_in_bytes();
        assert!(size > 1000, "Filter should be ~1KB, got {} bytes", size);
        assert!(size < 2000, "Filter should be <2KB, got {} bytes", size);
    }

    #[test]
    fn test_fp_rate_estimation() {
        let mut filter = BloomFilter::new(1000, 0.01);
        
        // Add more items than expected
        for i in 0..2000 {
            filter.insert(&format!("item_{}", i));
        }

        // FP rate should have increased
        let estimated = filter.current_false_positive_rate();
        assert!(estimated > 0.01, "FP rate should increase with more items");
    }
}
