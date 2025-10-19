// StarRocks Profile Parser
//
// Architecture:
// - core/          : Core parsing components (value, topology, operator, etc.)
// - specialized/   : Specialized metrics parsers (strategy pattern)
//
// Main entry point: ProfileComposer

// Core modules
pub mod error;
pub mod core;
pub mod specialized;
pub mod composer;

// Public API - Core exports
pub use error::{ParseError, ParseResult};
pub use composer::ProfileComposer;
pub use core::{ValueParser, TopologyParser, OperatorParser, TreeBuilder, MetricsParser};
pub use specialized::SpecializedMetricsParser;
