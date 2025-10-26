//! 
//! 


use crate::models::{Fragment, Operator};
use crate::parser::core::ValueParser;
use crate::parser::core::topology_parser::{TopologyNode, NodeClass};
use std::collections::HashMap;
use regex::Regex;
use once_cell::sync::Lazy;

#[allow(dead_code)]
static PLAN_NODE_ID_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"plan_node_id=(-?\d+)").unwrap()
});

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SearchMode {
    NativeOnly,
    SubordinateOnly,
    Both,
}

/// 
#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub plan_node_id: i32,
    pub node_class: NodeClass,
    
    pub operator_profiles: Vec<OperatorProfile>,
    pub subordinate_profiles: Vec<OperatorProfile>,
    
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
    /// 
    pub fn build_from_fragments_and_topology(
        topology_nodes: &[TopologyNode],
        fragments: &[Fragment]
    ) -> HashMap<i32, NodeInfo> {
        let mut all_node_infos = HashMap::new();
        
        let mut operators_by_plan_id: HashMap<i32, (Vec<Operator>, Vec<Operator>)> = HashMap::new();
        
        for fragment in fragments {
            let parser = ProfileNodeParser::new(fragment.clone());
            let node_map = parser.parse();
            
            for (plan_id, (native_ops, sub_ops)) in node_map {
                let entry = operators_by_plan_id.entry(plan_id).or_insert((Vec::new(), Vec::new()));
                entry.0.extend(native_ops);
                entry.1.extend(sub_ops);
            }
        }
        
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
    
    pub fn compute_time_usage(&mut self, cumulative_time: u64) {
        self.cpu_time = self.sum_up_metric(
            SearchMode::Both,
            true,
            &["CommonMetrics", "OperatorTotalTime"]
        );
        
        self.total_time = self.cpu_time.clone();
        
        match self.node_class {
            NodeClass::ExchangeNode => {
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
        
        if let Some(total) = &self.total_time {
            if cumulative_time > 0 {
                self.total_time_percentage = (total.value as f64 * 100.0) / cumulative_time as f64;
            }
        }
    }
    
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
    
    pub fn is_time_consuming_metric(&self, metric_name: &str) -> bool {
        use crate::constants::time_thresholds;
        
        if self.total_time.is_none() || self.total_time.as_ref().unwrap().value == 0 {
            return false;
        }
        
        let total_time_ns = self.total_time.as_ref().unwrap().value as f64;
        
        if let Some(metric) = self.search_metric(
            SearchMode::Both,
            None,
            true,
            &["CommonMetrics", metric_name]
        ) {
            if metric.unit == CounterUnit::TimeNs {
                let percentage = metric.value as f64 / total_time_ns;
                if percentage > time_thresholds::METRIC_CONSUMING_THRESHOLD {
                    return true;
                }
            }
        }
        
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
    
    fn search_metric(
        &self,
        search_mode: SearchMode,
        pattern: Option<&str>,
        use_max_value: bool,
        metric_path: &[&str]
    ) -> Option<Counter> {
        let profiles = self.get_profiles_by_mode(search_mode);
        
        for profile in profiles {
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
        let metrics_map = match metric_path[0] {
            "CommonMetrics" => &profile.common_metrics,
            "UniqueMetrics" => &profile.unique_metrics,
            _ => return None,
        };
        
        let metric_name = metric_path[1];
        
        let value_str = if use_max_value {
            metrics_map.get(&format!("__MAX_OF_{}", metric_name))
                .or_else(|| metrics_map.get(metric_name))
        } else {
            metrics_map.get(metric_name)
        }?;
        
        Self::parse_metric_value(value_str, metric_name)
    }
    
    fn parse_metric_value(value_str: &str, metric_name: &str) -> Option<Counter> {
        if metric_name.contains("Time") {

            ValueParser::parse_duration(value_str).ok().map(|duration| {
                Counter {
                    value: duration.as_nanos() as u64,
                    unit: CounterUnit::TimeNs,
                }
            })
        } else if metric_name.contains("Memory") || metric_name.contains("Bytes") {

            ValueParser::parse_bytes(value_str).ok().map(|bytes| {
                Counter {
                    value: bytes,
                    unit: CounterUnit::Bytes,
                }
            })
        } else if metric_name.contains("Rows") || metric_name.contains("RowNum") {

            value_str.parse::<u64>().ok().map(|rows| {
                Counter {
                    value: rows,
                    unit: CounterUnit::Rows,
                }
            })
        } else {

            value_str.parse::<u64>().ok().map(|val| {
                Counter {
                    value: val,
                    unit: CounterUnit::None,
                }
            })
        }
    }
}

/// 
pub struct ProfileNodeParser {
    fragment: Fragment,
}

impl ProfileNodeParser {
    pub fn new(fragment: Fragment) -> Self {
        Self { fragment }
    }
    
    /// 
    /// 
    pub fn parse(&self) -> HashMap<i32, (Vec<Operator>, Vec<Operator>)> {
        let mut node_map: HashMap<i32, (Vec<Operator>, Vec<Operator>)> = HashMap::new();
        
        for pipeline in &self.fragment.pipelines {
            for operator in &pipeline.operators {
                if let Some(ref plan_id_str) = operator.plan_node_id {
                    if let Ok(plan_id) = plan_id_str.parse::<i32>() {
                        let entry = node_map.entry(plan_id).or_insert((Vec::new(), Vec::new()));
                        
                        if Self::is_subordinate_operator(&operator.name) {
                            entry.1.push(operator.clone());
                        } else {
                            entry.0.push(operator.clone());
                        }
                    }
                }
            }
        }
        
        node_map
    }
    
    /// 
    /// 
    fn is_subordinate_operator(name: &str) -> bool {
        name.contains("LOCAL_EXCHANGE") ||
        name.contains("CHUNK_ACCUMULATE") ||
        name.contains("CACHE") ||
        name.contains("COLLECT_STATS")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_is_subordinate_operator() {
        assert!(ProfileNodeParser::is_subordinate_operator("LOCAL_EXCHANGE_SINK (plan_node_id=1)"));
        assert!(ProfileNodeParser::is_subordinate_operator("CHUNK_ACCUMULATE (plan_node_id=2)"));
        assert!(ProfileNodeParser::is_subordinate_operator("CACHE (plan_node_id=3)"));
        assert!(!ProfileNodeParser::is_subordinate_operator("EXCHANGE_SINK (plan_node_id=1)"));
        assert!(!ProfileNodeParser::is_subordinate_operator("AGGREGATE_BLOCKING_SOURCE (plan_node_id=2)"));
    }
}

