//! # HotspotDetector - 热点检测器
//! 
//! 负责分析执行树，识别性能热点和瓶颈。

use crate::models::{ExecutionTree, ExecutionTreeNode, HotSeverity};

#[derive(Debug, Clone)]
pub struct HotspotConfig {
    pub moderate_threshold: f64,   // 20% - 轻度热点
    pub severe_threshold: f64,     // 50% - 中度热点
    pub critical_threshold: f64,   // 80% - 严重热点
}

impl Default for HotspotConfig {
    fn default() -> Self {
        Self {
            moderate_threshold: 20.0,
            severe_threshold: 50.0,
            critical_threshold: 80.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Bottleneck {
    pub node_id: String,
    pub operator_name: String,
    pub severity: HotSeverity,
    pub time_percentage: f64,
    pub reason: String,
}

pub struct HotspotDetector;

impl HotspotDetector {
    /// 检测执行树中的热点
    /// 
    /// 标记每个节点的 is_hotspot 和 hotspot_severity 字段。
    /// 
    /// # Arguments
    /// * `nodes` - 执行树的所有节点
    /// * `config` - 热点检测配置
    pub fn detect(nodes: &mut [ExecutionTreeNode], config: HotspotConfig) {
        // 1. 找到最大执行时间
        let max_time = Self::find_max_operator_time(nodes);
        
        if max_time == 0.0 {
            return; // 没有时间信息，无法检测
        }
        
        // 2. 标记每个节点的热点状态
        for node in nodes.iter_mut() {
            if let Some(time_ms) = node.metrics.operator_total_time {
                let time = time_ms as f64; // already in milliseconds
                let percentage = (time / max_time) * 100.0;
                
                let severity = Self::calculate_severity(percentage, &config);
                node.is_hotspot = percentage > config.moderate_threshold;
                node.hotspot_severity = severity;
            }
        }
    }
    
    /// 找到性能瓶颈
    /// 
    /// 返回所有热点节点的详细信息。
    pub fn find_bottlenecks(tree: &ExecutionTree, config: &HotspotConfig) -> Vec<Bottleneck> {
        let mut bottlenecks = Vec::new();
        let max_time = Self::find_max_operator_time(&tree.nodes);
        
        if max_time == 0.0 {
            return bottlenecks;
        }
        
        for node in &tree.nodes {
            if let Some(time_ms) = node.metrics.operator_total_time {
                let time = time_ms as f64; // already in milliseconds
                let percentage = (time / max_time) * 100.0;
                
                if percentage > config.moderate_threshold {
                    let severity = Self::calculate_severity(percentage, config);
                    let reason = Self::diagnose_bottleneck_reason(node, percentage);
                    
                    bottlenecks.push(Bottleneck {
                        node_id: node.id.clone(),
                        operator_name: node.operator_name.clone(),
                        severity,
                        time_percentage: percentage,
                        reason,
                    });
                }
            }
        }
        
        // 按严重程度排序
        bottlenecks.sort_by(|a, b| {
            b.time_percentage.partial_cmp(&a.time_percentage).unwrap()
        });
        
        bottlenecks
    }
    
    /// 计算严重程度
    fn calculate_severity(percentage: f64, config: &HotspotConfig) -> HotSeverity {
        if percentage > config.critical_threshold {
            HotSeverity::Critical
        } else if percentage > config.severe_threshold {
            HotSeverity::Severe
        } else if percentage > config.moderate_threshold {
            HotSeverity::Moderate
        } else {
            HotSeverity::Normal
        }
    }
    
    /// 诊断瓶颈原因
    fn diagnose_bottleneck_reason(node: &ExecutionTreeNode, percentage: f64) -> String {
        let mut reasons = Vec::new();
        
        // 1. 时间占比过高
        reasons.push(format!("执行时间占总时间的 {:.1}%", percentage));
        
        // 2. 根据 Operator 类型分析
        match node.operator_name.as_str() {
            "CONNECTOR_SCAN" | "OLAP_SCAN" => {
                reasons.push("扫描操作耗时，可能是数据量大或 I/O 慢".to_string());
            }
            "HASH_JOIN" => {
                reasons.push("Join 操作耗时，可能是数据倾斜或构建端数据量大".to_string());
            }
            "AGGREGATE" => {
                reasons.push("聚合操作耗时，可能是分组键基数高".to_string());
            }
            "EXCHANGE_SINK" | "EXCHANGE_SOURCE" => {
                reasons.push("数据交换耗时，可能是网络慢或数据量大".to_string());
            }
            _ => {}
        }
        
        // 3. 检查内存使用
        if let Some(mem) = node.metrics.memory_usage {
            if mem > 1024 * 1024 * 1024 {  // > 1GB
                reasons.push(format!("内存使用较高: {:.2} GB", mem as f64 / 1024.0 / 1024.0 / 1024.0));
            }
        }
        
        reasons.join("; ")
    }
    
    /// 找到最大 Operator 执行时间（毫秒）
    fn find_max_operator_time(nodes: &[ExecutionTreeNode]) -> f64 {
        nodes.iter()
            .filter_map(|n| n.metrics.operator_total_time)
            .map(|t| t as f64) // already in milliseconds
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::HotSeverity;
    
    #[test]
    fn test_calculate_severity() {
        let config = HotspotConfig::default();
        
        assert_eq!(
            HotspotDetector::calculate_severity(85.0, &config),
            HotSeverity::Critical
        );
        assert_eq!(
            HotspotDetector::calculate_severity(60.0, &config),
            HotSeverity::Severe
        );
        assert_eq!(
            HotspotDetector::calculate_severity(30.0, &config),
            HotSeverity::Moderate
        );
        assert_eq!(
            HotspotDetector::calculate_severity(10.0, &config),
            HotSeverity::Normal
        );
    }
}

