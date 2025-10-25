//! # NodeInfo - 节点信息聚合器
//! 
//! 对应StarRocks的ExplainAnalyzer.NodeInfo类
//! 
//! 负责：
//! 1. 从fragments中按plan_node_id聚合operators
//! 2. 计算各种metrics（totalTime, cpuTime, networkTime, scanTime等）
//! 3. 计算时间百分比

use crate::models::{Fragment, Operator};
use crate::parser::core::{ProfileNodeParser, ValueParser};
use crate::parser::core::topology_parser::{TopologyNode, NodeClass};
use std::collections::HashMap;

/// OperatorProfile: operator的简化表示
#[derive(Debug, Clone)]
pub struct OperatorProfile {
    pub name: String,
    pub common_metrics: HashMap<String, String>,
    pub unique_metrics: HashMap<String, String>,
}

impl From<Operator> for OperatorProfile {
    fn from(op: Operator) -> Self {
        Self {
            name: op.name,
            common_metrics: op.common_metrics,
            unique_metrics: op.unique_metrics,
        }
    }
}

/// Counter: 对应StarRocks的Counter类
#[derive(Debug, Clone)]
pub struct Counter {
    pub value: u64,
    pub unit: CounterUnit,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CounterUnit {
    TimeNs,
    Bytes,
    Rows,
    None,
}

/// SearchMode: 对应StarRocks的ExplainAnalyzer.SearchMode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SearchMode {
    NativeOnly,
    SubordinateOnly,
    Both,
}

/// NodeInfo: 对应StarRocks的ExplainAnalyzer.NodeInfo
/// 
/// 聚合一个plan node的所有operators和metrics
#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub plan_node_id: i32,
    pub node_class: NodeClass,
    
    // Operator profiles（由ProfileNodeParser提取）
    pub operator_profiles: Vec<OperatorProfile>,
    pub subordinate_profiles: Vec<OperatorProfile>,
    
    // Computed metrics（对应ExplainAnalyzer.NodeInfo）
    pub total_time: Option<Counter>,
    pub cpu_time: Option<Counter>,
    pub network_time: Option<Counter>,
    pub scan_time: Option<Counter>,
    pub output_row_nums: Option<Counter>,
    pub peek_memory: Option<Counter>,
    pub allocated_memory: Option<Counter>,
    pub total_time_percentage: f64,
}

impl NodeInfo {
    /// 从fragments和topology构建所有NodeInfo
    /// 
    /// 对应ExplainAnalyzer.parseProfile() (line 239-304)
    pub fn build_from_fragments_and_topology(
        topology_nodes: &[TopologyNode],
        fragments: &[Fragment]
    ) -> HashMap<i32, NodeInfo> {
        let mut all_node_infos = HashMap::new();
        
        // 1. 使用ProfileNodeParser从每个fragment提取operators
        let mut operators_by_plan_id: HashMap<i32, (Vec<Operator>, Vec<Operator>)> = HashMap::new();
        
        for fragment in fragments {
            let parser = ProfileNodeParser::new(fragment.clone());
            let node_map = parser.parse();
            
            // 合并到全局map（处理EXCHANGE可能在多个fragments中）
            for (plan_id, (native_ops, sub_ops)) in node_map {
                let entry = operators_by_plan_id.entry(plan_id).or_insert((Vec::new(), Vec::new()));
                entry.0.extend(native_ops);
                entry.1.extend(sub_ops);
            }
        }
        
        // 2. 为每个topology node创建NodeInfo，绑定operators
        for topo_node in topology_nodes {
            let (native_ops, sub_ops) = operators_by_plan_id
                .remove(&topo_node.id)
                .unwrap_or((Vec::new(), Vec::new()));
            
            let node_info = NodeInfo {
                plan_node_id: topo_node.id,
                node_class: topo_node.node_class.clone(),
                operator_profiles: native_ops.into_iter().map(|op| op.into()).collect(),
                subordinate_profiles: sub_ops.into_iter().map(|op| op.into()).collect(),
                total_time: None,
                cpu_time: None,
                network_time: None,
                scan_time: None,
                output_row_nums: None,
                peek_memory: None,
                allocated_memory: None,
                total_time_percentage: 0.0,
            };
            
            all_node_infos.insert(topo_node.id, node_info);
        }
        
        // 3. 处理没有在topology中但有operators的plan_node_id（如果有）
        for (plan_id, (native_ops, sub_ops)) in operators_by_plan_id {
            let node_info = NodeInfo {
                plan_node_id: plan_id,
                node_class: NodeClass::Unknown,
                operator_profiles: native_ops.into_iter().map(|op| op.into()).collect(),
                subordinate_profiles: sub_ops.into_iter().map(|op| op.into()).collect(),
                total_time: None,
                cpu_time: None,
                network_time: None,
                scan_time: None,
                output_row_nums: None,
                peek_memory: None,
                allocated_memory: None,
                total_time_percentage: 0.0,
            };
            
            all_node_infos.insert(plan_id, node_info);
        }
        
        all_node_infos
    }
    
    /// 计算时间使用（对应ExplainAnalyzer.NodeInfo.computeTimeUsage, line 1529-1552）
    pub fn compute_time_usage(&mut self, cumulative_time: u64) {
        // 1. 聚合cpuTime（sumUpMetric with SearchMode.BOTH, useMaxValue=true）
        self.cpu_time = self.sum_up_metric(
            SearchMode::Both,
            true,
            &["CommonMetrics", "OperatorTotalTime"]
        );
        
        // 2. totalTime = cpuTime
        self.total_time = self.cpu_time.clone();
        
        // 3. 根据node_class添加额外时间
        match self.node_class {
            NodeClass::ExchangeNode => {
                // 添加NetworkTime（searchMetric with SearchMode.NATIVE_ONLY, useMaxValue=true）
                self.network_time = self.search_metric(
                    SearchMode::NativeOnly,
                    None,
                    true,
                    &["UniqueMetrics", "NetworkTime"]
                );
                
                if let (Some(total), Some(network)) = (&mut self.total_time, &self.network_time) {
                    total.value += network.value;
                }
            }
            NodeClass::ScanNode => {
                // 添加ScanTime
                self.scan_time = self.search_metric(
                    SearchMode::NativeOnly,
                    None,
                    true,
                    &["UniqueMetrics", "ScanTime"]
                );
                
                if let (Some(total), Some(scan)) = (&mut self.total_time, &self.scan_time) {
                    total.value += scan.value;
                }
            }
            _ => {}
        }
        
        // 4. 计算百分比
        if let Some(total) = &self.total_time {
            if cumulative_time > 0 {
                self.total_time_percentage = (total.value as f64 * 100.0) / cumulative_time as f64;
            }
        }
    }
    
    /// 计算内存使用（对应ExplainAnalyzer.NodeInfo.computeMemoryUsage, line 1554-1559）
    pub fn compute_memory_usage(&mut self) {
        self.peek_memory = self.sum_up_metric(
            SearchMode::NativeOnly,
            true,
            &["CommonMetrics", "OperatorPeakMemoryUsage"]
        );
        
        self.allocated_memory = self.sum_up_metric(
            SearchMode::NativeOnly,
            false,
            &["CommonMetrics", "OperatorAllocatedMemoryUsage"]
        );
    }
    
    /// 判断指标是否为时间消耗型（占总时间>30%）
    /// 对应ExplainAnalyzer.NodeInfo.isTimeConsumingMetric (line 1507-1522)
    pub fn is_time_consuming_metric(&self, metric_name: &str) -> bool {
        use crate::constants::time_thresholds;
        
        // 如果没有总时间或总时间为0，返回false
        if self.total_time.is_none() || self.total_time.as_ref().unwrap().value == 0 {
            return false;
        }
        
        let total_time_ns = self.total_time.as_ref().unwrap().value as f64;
        
        // 尝试从CommonMetrics获取指标
        if let Some(metric) = self.search_metric(
            SearchMode::Both,
            None,
            true,  // use_max = true，优先使用__MAX_OF_值
            &["CommonMetrics", metric_name]
        ) {
            if metric.unit == CounterUnit::TimeNs {
                let percentage = metric.value as f64 / total_time_ns;
                if percentage > time_thresholds::METRIC_CONSUMING_THRESHOLD {
                    return true;
                }
            }
        }
        
        // 尝试从UniqueMetrics获取指标
        if let Some(metric) = self.search_metric(
            SearchMode::Both,
            None,
            true,
            &["UniqueMetrics", metric_name]
        ) {
            if metric.unit == CounterUnit::TimeNs {
                let percentage = metric.value as f64 / total_time_ns;
                if percentage > time_thresholds::METRIC_CONSUMING_THRESHOLD {
                    return true;
                }
            }
        }
        
        false
    }
    
    /// sumUpMetric实现（对应ExplainAnalyzer.sumUpMetric, line 1304-1334）
    fn sum_up_metric(
        &self,
        search_mode: SearchMode,
        use_max_value: bool,
        metric_path: &[&str]
    ) -> Option<Counter> {
        let profiles = self.get_profiles_by_mode(search_mode);
        let mut sum = 0u64;
        let mut found = false;
        let mut unit = CounterUnit::None;
        
        for profile in profiles {
            if let Some(counter) = Self::get_metric_from_profile(profile, metric_path, use_max_value) {
                sum += counter.value;
                unit = counter.unit;
                found = true;
            }
        }
        
        if found {
            Some(Counter { value: sum, unit })
        } else {
            None
        }
    }
    
    /// searchMetric实现（对应ExplainAnalyzer.searchMetric, line 1256-1284）
    fn search_metric(
        &self,
        search_mode: SearchMode,
        pattern: Option<&str>,
        use_max_value: bool,
        metric_path: &[&str]
    ) -> Option<Counter> {
        let profiles = self.get_profiles_by_mode(search_mode);
        
        for profile in profiles {
            // 如果指定了pattern，检查operator名称
            if let Some(pat) = pattern {
                if !profile.name.contains(pat) {
                    continue;
                }
            }
            
            if let Some(counter) = Self::get_metric_from_profile(profile, metric_path, use_max_value) {
                return Some(counter);
            }
        }
        
        None
    }
    
    fn get_profiles_by_mode(&self, mode: SearchMode) -> Vec<&OperatorProfile> {
        match mode {
            SearchMode::NativeOnly => self.operator_profiles.iter().collect(),
            SearchMode::SubordinateOnly => self.subordinate_profiles.iter().collect(),
            SearchMode::Both => {
                let mut profiles = self.operator_profiles.iter().collect::<Vec<_>>();
                profiles.extend(self.subordinate_profiles.iter());
                profiles
            }
        }
    }
    
    fn get_metric_from_profile(
        profile: &OperatorProfile,
        metric_path: &[&str],
        use_max_value: bool
    ) -> Option<Counter> {
        // metric_path: ["CommonMetrics", "OperatorTotalTime"]
        let metrics_map = match metric_path[0] {
            "CommonMetrics" => &profile.common_metrics,
            "UniqueMetrics" => &profile.unique_metrics,
            _ => return None,
        };
        
        let metric_name = metric_path[1];
        
        // 如果use_max_value=true，优先查找__MAX_OF_前缀
        let value_str = if use_max_value {
            metrics_map.get(&format!("__MAX_OF_{}", metric_name))
                .or_else(|| metrics_map.get(metric_name))
        } else {
            metrics_map.get(metric_name)
        }?;
        
        // 解析value
        Self::parse_metric_value(value_str, metric_name)
    }
    
    fn parse_metric_value(value_str: &str, metric_name: &str) -> Option<Counter> {
        // 根据metric名称判断类型
        if metric_name.contains("Time") {
            // 时间类型
            ValueParser::parse_duration(value_str).ok().map(|duration| {
                Counter {
                    value: duration.as_nanos() as u64,
                    unit: CounterUnit::TimeNs,
                }
            })
        } else if metric_name.contains("Memory") || metric_name.contains("Bytes") {
            // 字节类型
            ValueParser::parse_bytes(value_str).ok().map(|bytes| {
                Counter {
                    value: bytes,
                    unit: CounterUnit::Bytes,
                }
            })
        } else if metric_name.contains("Rows") || metric_name.contains("RowNum") {
            // 行数类型
            value_str.parse::<u64>().ok().map(|rows| {
                Counter {
                    value: rows,
                    unit: CounterUnit::Rows,
                }
            })
        } else {
            // 默认为数字
            value_str.parse::<u64>().ok().map(|val| {
                Counter {
                    value: val,
                    unit: CounterUnit::None,
                }
            })
        }
    }
}

