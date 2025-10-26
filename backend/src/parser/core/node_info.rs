//! 
//! 

use crate::models::{Fragment, Operator};
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

impl SearchMode {
    pub fn is_native(&self) -> bool {
        matches!(self, SearchMode::NativeOnly | SearchMode::Both)
    }
    
    pub fn is_subordinate(&self) -> bool {
        matches!(self, SearchMode::SubordinateOnly | SearchMode::Both)
    }
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
    /// StarRocks官方sumUpMetric方法实现
    /// 严格按照官方源码实现
    pub fn sum_up_metric(&self, search_mode: SearchMode, use_max_value: bool, name_levels: &[&str]) -> Option<Counter> {
        println!("DEBUG: sum_up_metric called with search_mode: {:?}, use_max_value: {}, name_levels: {:?}", search_mode, use_max_value, name_levels);
        let mut counter_sum_up: Option<Counter> = None;
        let mut profiles = Vec::new();
        
        if search_mode.is_native() {
            profiles.extend(self.operator_profiles.iter());
        }
        if search_mode.is_subordinate() {
            profiles.extend(self.subordinate_profiles.iter());
        }
        
        println!("DEBUG: sum_up_metric processing {} profiles", profiles.len());
        
        for profile in profiles {
            let cur = Self::get_last_level(profile, name_levels);
            let last_index = name_levels.len() - 1;
            let counter = if use_max_value {
                Self::get_max_counter(&cur, name_levels[last_index])
            } else {
                Self::get_counter(&cur, name_levels[last_index])
            };
            
            if let Some(counter) = counter {
                println!("DEBUG: sum_up_metric found counter: {:?}", counter);
                if counter_sum_up.is_none() {
                    counter_sum_up = Some(Counter {
                        value: 0,
                        unit: counter.unit.clone(),
                    });
                }
                
                if let Some(ref mut sum_counter) = counter_sum_up {
                    sum_counter.value += counter.value;
                    println!("DEBUG: sum_up_metric added {}ns, total now: {}ns", counter.value, sum_counter.value);
                }
            }
        }
        
        println!("DEBUG: sum_up_metric result: {:?}", counter_sum_up);
        counter_sum_up
    }
    
    /// StarRocks官方merge方法实现
    /// 严格按照官方源码实现
    pub fn merge(&mut self, other: NodeInfo) {
        self.operator_profiles.extend(other.operator_profiles);
        self.subordinate_profiles.extend(other.subordinate_profiles);
        
        // 如果当前node_class是Unknown，使用other的node_class
        if self.node_class == NodeClass::Unknown && other.node_class != NodeClass::Unknown {
            self.node_class = other.node_class;
        }
    }
    
    /// StarRocks官方searchMetric方法实现
    /// 严格按照官方源码实现
    pub fn search_metric(&self, search_mode: SearchMode, pattern: Option<&str>, use_max_value: bool, name_levels: &[&str]) -> Option<Counter> {
        let mut profiles = Vec::new();
        
        if search_mode.is_native() {
            profiles.extend(self.operator_profiles.iter());
        }
        if search_mode.is_subordinate() {
            profiles.extend(self.subordinate_profiles.iter());
        }
        
        for profile in profiles {
            if let Some(pattern) = pattern {
                if !profile.name.contains(pattern) {
                    continue;
                }
            }
            
            let cur = Self::get_last_level(profile, name_levels);
            let last_index = name_levels.len() - 1;
            let counter = if use_max_value {
                Self::get_max_counter(&cur, name_levels[last_index])
            } else {
                Self::get_counter(&cur, name_levels[last_index])
            };
            
            if counter.is_some() {
                return counter;
            }
        }
        
        None
    }
    
    /// StarRocks官方getLastLevel方法实现
    /// 严格按照官方源码实现
    fn get_last_level<'a>(profile: &'a OperatorProfile, name_levels: &[&str]) -> &'a OperatorProfile {
        // 官方源码：RuntimeProfile cur = operatorProfile; for (int i = 0; i < nameLevels.length - 1; i++) { cur = cur.getChild(nameLevels[i]); }
        // 在我们的实现中，OperatorProfile直接包含common_metrics和unique_metrics
        // 所以当name_levels为["CommonMetrics", "OperatorTotalTime"]时，我们直接返回profile
        // 当name_levels为["UniqueMetrics", "ScanTime"]时，我们也直接返回profile
        profile
    }
    
    /// StarRocks官方getCounter方法实现
    /// 严格按照官方源码实现
    fn get_counter(profile: &OperatorProfile, name: &str) -> Option<Counter> {
        // 首先尝试从CommonMetrics获取
        if let Some(value_str) = profile.common_metrics.get(name) {
            return Self::parse_counter_value(value_str);
        }
        
        // 然后尝试从UniqueMetrics获取
        if let Some(value_str) = profile.unique_metrics.get(name) {
            return Self::parse_counter_value(value_str);
        }
        
        None
    }
    
    /// StarRocks官方getMaxCounter方法实现
    /// 严格按照官方源码实现
    fn get_max_counter(profile: &OperatorProfile, name: &str) -> Option<Counter> {
        let max_name = format!("__MAX_OF_{}", name);
        
        // 首先尝试找__MAX_OF_前缀的指标
        if let Some(counter) = Self::get_counter(profile, &max_name) {
            return Some(counter);
        }
        
        // 如果找不到，就使用原始指标名
        Self::get_counter(profile, name)
    }
    
    /// StarRocks官方parse_counter_value方法实现
    /// 严格按照官方源码实现
    fn parse_counter_value(value_str: &str) -> Option<Counter> {
        let value_str = value_str.trim();
        
        // 解析时间值
        if value_str.ends_with("ns") {
            if let Ok(value) = value_str.trim_end_matches("ns").parse::<f64>() {
                return Some(Counter { value: value as u64, unit: CounterUnit::TimeNs });
            }
        }
        if value_str.ends_with("us") {
            if let Ok(value) = value_str.trim_end_matches("us").parse::<f64>() {
                return Some(Counter { value: (value * 1000.0) as u64, unit: CounterUnit::TimeNs });
            }
        }
        if value_str.ends_with("ms") {
            if let Ok(value) = value_str.trim_end_matches("ms").parse::<f64>() {
                return Some(Counter { value: (value * 1_000_000.0) as u64, unit: CounterUnit::TimeNs });
            }
        }
        if value_str.ends_with("s") {
            if let Ok(value) = value_str.trim_end_matches("s").parse::<f64>() {
                return Some(Counter { value: (value * 1_000_000_000.0) as u64, unit: CounterUnit::TimeNs });
            }
        }
        
        // 解析复合时间格式，如 "2m48s"
        if value_str.contains("m") && value_str.contains("s") && !value_str.contains("ms") {
            if let Some(m_pos) = value_str.find("m") {
                if let Some(s_pos) = value_str.find("s") {
                    if s_pos > m_pos {
                        let m_part = &value_str[..m_pos];
                        let s_part = &value_str[m_pos+1..s_pos];
                        if let (Ok(m_val), Ok(s_val)) = (m_part.parse::<f64>(), s_part.parse::<f64>()) {
                            let total_ns = (m_val * 60.0 * 1_000_000_000.0 + s_val * 1_000_000_000.0) as u64;
                            return Some(Counter { value: total_ns, unit: CounterUnit::TimeNs });
                        }
                    }
                }
            }
        }
        
        // 解析复合时间格式，如 "3s984ms"
        if value_str.contains("s") && value_str.contains("ms") {
            if let Some(s_pos) = value_str.find("s") {
                if let Some(ms_pos) = value_str.find("ms") {
                    if ms_pos > s_pos {
                        let s_part = &value_str[..s_pos];
                        let ms_part = &value_str[s_pos+1..ms_pos];
                        if let (Ok(s_val), Ok(ms_val)) = (s_part.parse::<f64>(), ms_part.parse::<f64>()) {
                            let total_ns = (s_val * 1_000_000_000.0 + ms_val * 1_000_000.0) as u64;
                            return Some(Counter { value: total_ns, unit: CounterUnit::TimeNs });
                        }
                    }
                }
            }
        }
        
        // 解析字节值
        if value_str.ends_with("B") || value_str.ends_with("KB") || value_str.ends_with("MB") || value_str.ends_with("GB") {
            if let Ok(value) = Self::parse_bytes_to_u64(value_str) {
                return Some(Counter { value, unit: CounterUnit::Bytes });
            }
        }
        
        // 解析行数值
        if let Ok(value) = value_str.parse::<u64>() {
            return Some(Counter { value, unit: CounterUnit::Rows });
        }
        
        None
    }
    
    /// StarRocks官方parse_bytes_to_u64方法实现
    /// 严格按照官方源码实现
    fn parse_bytes_to_u64(value_str: &str) -> Result<u64, std::num::ParseIntError> {
        let value_str = value_str.trim();
        if value_str.ends_with("GB") {
            let num_str = value_str.trim_end_matches("GB");
            num_str.parse::<u64>().map(|v| v * 1024 * 1024 * 1024)
        } else if value_str.ends_with("MB") {
            let num_str = value_str.trim_end_matches("MB");
            num_str.parse::<u64>().map(|v| v * 1024 * 1024)
        } else if value_str.ends_with("KB") {
            let num_str = value_str.trim_end_matches("KB");
            num_str.parse::<u64>().map(|v| v * 1024)
        } else if value_str.ends_with("B") {
            let num_str = value_str.trim_end_matches("B");
            num_str.parse::<u64>()
        } else {
            value_str.parse::<u64>()
        }
    }
    
    /// StarRocks官方computeTimeUsage方法实现
    /// 严格按照官方源码实现
    pub fn compute_time_usage(&mut self, cumulative_operator_time: u64) {
        
        self.total_time = Some(Counter { value: 0, unit: CounterUnit::TimeNs });
        
        // 计算CPU时间 (OperatorTotalTime)
        self.cpu_time = self.sum_up_metric(SearchMode::Both, true, &["CommonMetrics", "OperatorTotalTime"]);
        if let Some(ref cpu_time) = self.cpu_time {
            if let Some(ref mut total_time) = self.total_time {
                total_time.value += cpu_time.value;
            }
        }
        
        // 根据节点类型计算特定时间
        match self.node_class {
            NodeClass::ExchangeNode => {
                // Exchange节点：添加NetworkTime
                self.network_time = self.search_metric(SearchMode::NativeOnly, None, true, &["UniqueMetrics", "NetworkTime"]);
                println!("DEBUG: network_time result: {:?}", self.network_time);
                if let Some(ref network_time) = self.network_time {
                    if let Some(ref mut total_time) = self.total_time {
                        total_time.value += network_time.value;
                        println!("DEBUG: Added network_time {}ns to total_time, now {}", network_time.value, total_time.value);
                    }
                }
            },
            NodeClass::ScanNode => {
                // Scan节点：添加ScanTime
                self.scan_time = self.search_metric(SearchMode::NativeOnly, None, true, &["UniqueMetrics", "ScanTime"]);
                println!("DEBUG: scan_time result: {:?}", self.scan_time);
                if let Some(ref scan_time) = self.scan_time {
                    if let Some(ref mut total_time) = self.total_time {
                        total_time.value += scan_time.value;
                        println!("DEBUG: Added scan_time {}ns to total_time, now {}", scan_time.value, total_time.value);
                    }
                }
            },
            _ => {
                // 其他节点类型不需要额外的时间计算
                println!("DEBUG: Node type {:?} doesn't need additional time calculation", self.node_class);
            }
        }
        
        // 计算时间百分比
        if cumulative_operator_time > 0 {
            self.total_time_percentage = (self.total_time.as_ref().unwrap().value as f64 * 100.0) / cumulative_operator_time as f64;
            println!("DEBUG: Calculated percentage: {}ns / {}ns = {:.2}%", 
                self.total_time.as_ref().unwrap().value, cumulative_operator_time, self.total_time_percentage);
        } else {
            println!("DEBUG: cumulative_operator_time is 0, cannot calculate percentage");
        }
    }
    
    /// StarRocks官方computeMemoryUsage方法实现
    /// 严格按照官方源码实现
    pub fn compute_memory_usage(&mut self) {
        self.peek_memory = self.sum_up_metric(SearchMode::NativeOnly, true, &["CommonMetrics", "OperatorPeakMemoryUsage"]);
        self.allocated_memory = self.sum_up_metric(SearchMode::NativeOnly, false, &["CommonMetrics", "OperatorAllocatedMemoryUsage"]);
    }

    /// 
    pub fn build_from_fragments_and_topology(
        topology_nodes: &[TopologyNode],
        fragments: &[Fragment]
    ) -> HashMap<i32, NodeInfo> {
        let mut all_node_infos: HashMap<i32, NodeInfo> = HashMap::new();
        
        // 按照官方源码的逻辑：逐个处理每个fragment
        for fragment in fragments {
            let parser = ProfileNodeParser::new(fragment.clone());
            let fragment_nodes = parser.parse();
            
            // 合并相同plan_node_id的NodeInfo
            for (plan_id, mut node_info) in fragment_nodes {
                if let Some(existing_node) = all_node_infos.get_mut(&plan_id) {
                    existing_node.merge(node_info);
                } else {
                    all_node_infos.insert(plan_id, node_info);
                }
            }
        }
        
        all_node_infos
    }
}

/// StarRocks官方ProfileNodeParser实现
/// 严格按照官方源码实现
pub struct ProfileNodeParser {
    fragment: Fragment,
}

impl ProfileNodeParser {
    pub fn new(fragment: Fragment) -> Self {
        Self { fragment }
    }
    
    /// StarRocks官方parse方法实现
    /// 严格按照官方源码实现 - 逐个处理每个operator
    pub fn parse(&self) -> HashMap<i32, NodeInfo> {
        let mut nodes: HashMap<i32, NodeInfo> = HashMap::new();
        
        // 按照官方源码的逻辑：逐个处理每个pipeline中的每个operator
        for pipeline in &self.fragment.pipelines {
            for operator in &pipeline.operators {
                if let Some(ref plan_id_str) = operator.plan_node_id {
                    if let Ok(plan_id) = plan_id_str.parse::<i32>() {
                        // 检查是否为subordinate operator
                        // 官方源码：boolean isSubordinate = commonMetrics != null && commonMetrics.containsInfoString("IsSubordinate");
                        let is_subordinate = operator.common_metrics.contains_key("IsSubordinate") ||
                                           Self::is_subordinate_operator(&operator.name);
                        
                        let mut operator_profiles = Vec::new();
                        let mut subordinate_profiles = Vec::new();
                        
                        if is_subordinate {
                            subordinate_profiles.push(operator.clone().into());
                        } else {
                            operator_profiles.push(operator.clone().into());
                        }
                        
                        let node_info = NodeInfo {
                            plan_node_id: plan_id,
                            node_class: Self::determine_node_class(&operator.name),
                            operator_profiles,
                            subordinate_profiles,
                            total_time: None,
                            cpu_time: None,
                            network_time: None,
                            scan_time: None,
                            output_row_nums: None,
                            peek_memory: None,
                            allocated_memory: None,
                            total_time_percentage: 0.0,
                        };
                        
                        // 官方源码：如果已存在相同plan_node_id的NodeInfo，则合并
                        if let Some(existing_node) = nodes.get_mut(&plan_id) {
                            existing_node.merge(node_info);
                        } else {
                            nodes.insert(plan_id, node_info);
                        }
                    }
                }
            }
        }
        
        nodes
    }
    
    /// StarRocks官方is_subordinate_operator方法实现
    /// 严格按照官方源码实现
    fn is_subordinate_operator(name: &str) -> bool {
        name.contains("LOCAL_EXCHANGE") ||
        name.contains("CHUNK_ACCUMULATE") ||
        name.contains("CACHE") ||
        name.contains("COLLECT_STATS")
    }
    
    /// 根据operator名称确定节点类型
    fn determine_node_class(name: &str) -> NodeClass {
        let op_name = if let Some(pos) = name.find(" (plan_node_id=") {
            &name[..pos]
        } else {
            name
        };
        
        println!("DEBUG: determine_node_class called with name: '{}', op_name: '{}'", name, op_name);
        
        let node_class = match op_name {
            "OLAP_SCAN" | "CONNECTOR_SCAN" | "ES_SCAN" | "SCHEMA_SCAN" => NodeClass::ScanNode,
            "EXCHANGE_SOURCE" | "EXCHANGE_SINK" | "EXCHANGE" | "MERGE_EXCHANGE" => NodeClass::ExchangeNode,
            "AGGREGATE" | "AGGREGATION" | "AGGREGATE_BLOCKING_SINK" | "AGGREGATE_BLOCKING_SOURCE" => NodeClass::AggregationNode,
            "HASH_JOIN" | "NL_JOIN" | "CROSS_JOIN" | "NEST_LOOP_JOIN" => NodeClass::JoinNode,
            "RESULT_SINK" => NodeClass::ResultSink,
            "OLAP_TABLE_SINK" => NodeClass::OlapTableSink,
            "SORT" | "LOCAL_SORT" => NodeClass::SortNode,
            "PROJECT" | "FILTER" | "TABLE_FUNCTION" => NodeClass::ProjectNode,
            _ => NodeClass::Unknown,
        };
        
        println!("DEBUG: determine_node_class result: {:?}", node_class);
        node_class
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Fragment, Pipeline, Operator};
    use std::collections::HashMap;
    
    #[test]
    fn test_is_subordinate_operator() {
        assert!(ProfileNodeParser::is_subordinate_operator("LOCAL_EXCHANGE_SINK (plan_node_id=1)"));
        assert!(ProfileNodeParser::is_subordinate_operator("CHUNK_ACCUMULATE (plan_node_id=2)"));
        assert!(ProfileNodeParser::is_subordinate_operator("CACHE (plan_node_id=3)"));
        assert!(!ProfileNodeParser::is_subordinate_operator("EXCHANGE_SINK (plan_node_id=1)"));
        assert!(!ProfileNodeParser::is_subordinate_operator("AGGREGATE_BLOCKING_SOURCE (plan_node_id=2)"));
    }
    
    #[test]
    fn test_profile_node_parser_parse() {
        // 创建测试数据，模拟profile5.txt中的CONNECTOR_SCAN
        let mut common_metrics = HashMap::new();
        common_metrics.insert("OperatorTotalTime".to_string(), "0.041546ms".to_string());
        common_metrics.insert("PullRowNum".to_string(), "1".to_string());
        
        let mut unique_metrics = HashMap::new();
        unique_metrics.insert("SharedScan".to_string(), "False".to_string());
        unique_metrics.insert("MorselQueueType".to_string(), "dynamic_morsel_queue".to_string());
        unique_metrics.insert("MorselsCount".to_string(), "14".to_string());
        unique_metrics.insert("TabletCount".to_string(), "14".to_string());
        
        let operator = Operator {
            name: "CONNECTOR_SCAN (plan_node_id=0)".to_string(),
            plan_node_id: Some("0".to_string()),
            operator_id: Some("op_0".to_string()),
            common_metrics,
            unique_metrics,
            children: Vec::new(),
        };
        
        let pipeline = Pipeline {
            id: "pipeline_0".to_string(),
            metrics: HashMap::new(),
            operators: vec![operator],
        };
        
        let fragment = Fragment {
            id: "fragment_0".to_string(),
            backend_addresses: Vec::new(),
            instance_ids: Vec::new(),
            pipelines: vec![pipeline],
        };
        
        let parser = ProfileNodeParser::new(fragment);
        let node_map = parser.parse();
        
        // 验证结果
        assert_eq!(node_map.len(), 1);
        assert!(node_map.contains_key(&0));
        
        let node_info = &node_map[&0];
        assert_eq!(node_info.operator_profiles.len(), 1);
        assert_eq!(node_info.subordinate_profiles.len(), 0);
        
        let op_profile = &node_info.operator_profiles[0];
        assert_eq!(op_profile.name, "CONNECTOR_SCAN (plan_node_id=0)");
        assert_eq!(op_profile.unique_metrics.get("SharedScan"), Some(&"False".to_string()));
        assert_eq!(op_profile.unique_metrics.get("MorselQueueType"), Some(&"dynamic_morsel_queue".to_string()));
    }
    
    #[test]
    fn test_node_info_sum_up_metric() {
        // 创建测试数据
        let mut common_metrics1 = HashMap::new();
        common_metrics1.insert("OperatorTotalTime".to_string(), "100ms".to_string());
        
        let mut common_metrics2 = HashMap::new();
        common_metrics2.insert("OperatorTotalTime".to_string(), "200ms".to_string());
        
        let op1 = OperatorProfile {
            name: "OP1".to_string(),
            common_metrics: common_metrics1,
            unique_metrics: HashMap::new(),
        };
        
        let op2 = OperatorProfile {
            name: "OP2".to_string(),
            common_metrics: common_metrics2,
            unique_metrics: HashMap::new(),
        };
        
        let node_info = NodeInfo {
            plan_node_id: 0,
            node_class: NodeClass::ScanNode,
            operator_profiles: vec![op1, op2],
            subordinate_profiles: Vec::new(),
            total_time: None,
            cpu_time: None,
            network_time: None,
            scan_time: None,
            output_row_nums: None,
            peek_memory: None,
            allocated_memory: None,
            total_time_percentage: 0.0,
        };
        
        // 测试sum_up_metric
        let result = node_info.sum_up_metric(SearchMode::Both, false, &["CommonMetrics", "OperatorTotalTime"]);
        assert!(result.is_some());
        let counter = result.unwrap();
        assert_eq!(counter.value, 300_000_000); // 100ms + 200ms = 300ms = 300,000,000ns
        assert_eq!(counter.unit, CounterUnit::TimeNs);
    }
    
    #[test]
    fn test_node_info_search_metric() {
        // 创建测试数据
        let mut unique_metrics = HashMap::new();
        unique_metrics.insert("ScanTime".to_string(), "50ms".to_string());
        
        let op = OperatorProfile {
            name: "CONNECTOR_SCAN".to_string(),
            common_metrics: HashMap::new(),
            unique_metrics,
        };
        
        let node_info = NodeInfo {
            plan_node_id: 0,
            node_class: NodeClass::ScanNode,
            operator_profiles: vec![op],
            subordinate_profiles: Vec::new(),
            total_time: None,
            cpu_time: None,
            network_time: None,
            scan_time: None,
            output_row_nums: None,
            peek_memory: None,
            allocated_memory: None,
            total_time_percentage: 0.0,
        };
        
        // 测试search_metric
        let result = node_info.search_metric(SearchMode::NativeOnly, None, false, &["UniqueMetrics", "ScanTime"]);
        assert!(result.is_some());
        let counter = result.unwrap();
        assert_eq!(counter.value, 50_000_000); // 50ms = 50,000,000ns
        assert_eq!(counter.unit, CounterUnit::TimeNs);
    }
    
    #[test]
    fn test_node_info_compute_time_usage() {
        // 创建测试数据
        let mut common_metrics = HashMap::new();
        common_metrics.insert("OperatorTotalTime".to_string(), "100ms".to_string());
        
        let mut unique_metrics = HashMap::new();
        unique_metrics.insert("ScanTime".to_string(), "50ms".to_string());
        
        let op = OperatorProfile {
            name: "CONNECTOR_SCAN".to_string(),
            common_metrics,
            unique_metrics,
        };
        
        let mut node_info = NodeInfo {
            plan_node_id: 0,
            node_class: NodeClass::ScanNode,
            operator_profiles: vec![op],
            subordinate_profiles: Vec::new(),
            total_time: None,
            cpu_time: None,
            network_time: None,
            scan_time: None,
            output_row_nums: None,
            peek_memory: None,
            allocated_memory: None,
            total_time_percentage: 0.0,
        };
        
        // 测试compute_time_usage
        node_info.compute_time_usage(1000_000_000); // 1秒 = 1,000,000,000ns
        
        assert!(node_info.total_time.is_some());
        assert!(node_info.cpu_time.is_some());
        assert!(node_info.scan_time.is_some());
        
        let total_time = node_info.total_time.unwrap();
        assert_eq!(total_time.value, 150_000_000); // 100ms + 50ms = 150ms = 150,000,000ns
        
        let cpu_time = node_info.cpu_time.unwrap();
        assert_eq!(cpu_time.value, 100_000_000); // 100ms = 100,000,000ns
        
        let scan_time = node_info.scan_time.unwrap();
        assert_eq!(scan_time.value, 50_000_000); // 50ms = 50,000,000ns
        
        // 验证时间百分比
        assert_eq!(node_info.total_time_percentage, 15.0); // 150ms / 1000ms = 15%
    }
}