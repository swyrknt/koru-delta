/// Memory tiering subsystem.
///
/// This module implements the layered memory architecture inspired by
/// the human brain's memory systems:
///
/// - **Hot Memory**: Working memory (fast, limited, LRU cache)
/// - **Warm Memory**: Recent chronicle (full detail, disk-backed)
/// - **Cold Memory**: Consolidated epochs (compressed patterns)
/// - **Deep Memory**: Genomic storage (minimal, portable)
///
/// ## The Flow
///
/// Data flows through the layers based on access patterns:
///
/// ```text
/// Put: Data → Hot (immediate access)
///          ↓
///     Access stops → Warm (chronicle)
///          ↓
///     Time passes → Cold (consolidated)
///          ↓
///     Epoch ends → Deep (genomic)
/// ```
///
/// ## User Benefit
///
/// - Bounded RAM regardless of database size
/// - Frequently accessed data stays fast
/// - Old data compressed but available
/// - Runs on devices from Raspberry Pi to datacenter
pub mod hot;
pub mod warm;
pub mod cold;
pub mod deep;

pub use hot::{Evicted, HotConfig, HotMemory, HotStats};
pub use warm::{WarmConfig, WarmMemory, WarmStats};
pub use cold::{ColdConfig, ColdMemory, ColdStats, ConsolidationResult, Pattern};
pub use deep::{CausalTopology, DeepConfig, DeepMemory, DeepStats, EpochSummary, ExpressionResult, Genome, ReferencePattern};
