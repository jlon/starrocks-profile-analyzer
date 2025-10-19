//! # SpecializedMetricsStrategy - 专用指标解析策略接口
//! 
//! 定义所有专用指标解析策略必须实现的 trait。

use crate::models::OperatorSpecializedMetrics;

/// 专用指标解析策略 trait
/// 
/// 所有 Operator 的专用指标解析器都必须实现此 trait。
pub trait SpecializedMetricsStrategy: Send + Sync {
    /// 解析专用指标
    /// 
    /// # Arguments
    /// * `text` - Operator 的 UniqueMetrics 文本块（或完整的 Operator 块）
    /// 
    /// # Returns
    /// * `OperatorSpecializedMetrics` - 解析后的专用指标
    fn parse(&self, text: &str) -> OperatorSpecializedMetrics;
}

