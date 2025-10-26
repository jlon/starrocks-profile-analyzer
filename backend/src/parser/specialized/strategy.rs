//! 

use crate::models::OperatorSpecializedMetrics;

/// 
pub trait SpecializedMetricsStrategy: Send + Sync {

    /// 
    /// 
    fn parse(&self, text: &str) -> OperatorSpecializedMetrics;
}

