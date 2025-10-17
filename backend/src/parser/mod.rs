// StarRocks Profile Parser
//
// Architecture:
// - core/          : Core parsing components (value, topology, operator, etc.)
// - specialized/   : Specialized metrics parsers (strategy pattern)
// - analysis/      : Performance analysis components (hotspot detection)
//
// Main entry point: ProfileComposer

// Core modules
pub mod error;
pub mod core;
pub mod specialized;
pub mod analysis;
pub mod composer;

// Public API - Core exports
pub use error::{ParseError, ParseResult};
pub use composer::ProfileComposer;
pub use core::{ValueParser, TopologyParser, OperatorParser, TreeBuilder, MetricsParser};
pub use analysis::{Bottleneck, HotspotConfig, HotspotDetector};
pub use specialized::SpecializedMetricsParser;
