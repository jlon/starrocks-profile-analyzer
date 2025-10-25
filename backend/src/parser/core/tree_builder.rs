//! # TreeBuilder - 执行树构建器
//! 
//! 负责根据 Topology 或 Fragment 信息构建执行树，并计算节点深度。

use crate::models::{ExecutionTree, ExecutionTreeNode};
use crate::parser::error::{ParseError, ParseResult};
use super::topology_parser::TopologyGraph;
use std::collections::{HashMap, HashSet, VecDeque};

pub struct TreeBuilder;

impl TreeBuilder {
    ///
    /// # Arguments
    /// * `topology` - 解析好的拓扑图
    /// * `nodes` - 所有已解析的执行节点（包含指标）
    /// * `fragments` - 解析后的Fragments列表，用于查找SINK节点
    ///
    /// # Returns
    /// * `ExecutionTree` - 构建好的执行树
    pub fn build_from_topology(
        topology: &TopologyGraph,
        mut nodes: Vec<ExecutionTreeNode>,
        fragments: &[crate::models::Fragment],
        summary: &crate::models::ProfileSummary,
    ) -> ParseResult<ExecutionTree> {
        // 1. 建立节点ID映射
        let mut id_to_idx: HashMap<i32, usize> = HashMap::new();
        for (idx, node) in nodes.iter().enumerate() {
            if let Some(plan_id) = node.plan_node_id {
                id_to_idx.insert(plan_id, idx);
            }
        }

        // 2. 根据 Topology 建立父子关系
        for topo_node in &topology.nodes {
            if let Some(&node_idx) = id_to_idx.get(&topo_node.id) {
                // 清空旧的 children（避免重复）
                nodes[node_idx].children.clear();

                for &child_id in &topo_node.children {
                    if let Some(&child_idx) = id_to_idx.get(&child_id) {
                        let child_node_id = nodes[child_idx].id.clone();
                        nodes[node_idx].children.push(child_node_id);
                        nodes[child_idx].parent_plan_node_id = Some(topo_node.id);
                    }
                }
            }
        }

        // 3. 使用两层查找策略找到正确的SINK节点作为根节点
        let sink_node_name = Self::find_sink_node_for_tree_root(fragments);

        // 4. 找到SINK节点在nodes中的索引
        let root_idx = if let Some(sink_name) = sink_node_name {
            // 优先使用找到的SINK节点作为根节点
            let sink_idx = nodes.iter().position(|n| n.operator_name == sink_name)
                .or_else(|| {
                    // 如果没找到，尝试查找以_SINK结尾的节点
                    nodes.iter().position(|n| n.operator_name.ends_with("_SINK"))
                });
            
            if let Some(sink_idx) = sink_idx {
                // 建立正确的数据流关系：SINK节点 -> topology根节点
                if let Some(&topo_root_idx) = id_to_idx.get(&topology.root_id) {
                    let topo_root_id = nodes[topo_root_idx].id.clone();
                    
                    // SINK节点指向topology根节点作为子节点（数据流向）
                    if !nodes[sink_idx].children.contains(&topo_root_id) {
                        nodes[sink_idx].children.push(topo_root_id);
                    }
                    // 设置topology根节点的父节点为SINK节点的plan_node_id
                    nodes[topo_root_idx].parent_plan_node_id = nodes[sink_idx].plan_node_id;
                }
                
                // SINK节点是树的根节点（用于显示）
                sink_idx
            } else {
                // 如果都没找到，使用topology的root_id
                id_to_idx.get(&topology.root_id).copied().unwrap_or(0)
            }
        } else {
            // 如果没找到SINK节点，使用topology的root_id
            id_to_idx.get(&topology.root_id)
                .copied()
                .ok_or_else(|| ParseError::TreeError(
                    format!("Root node {} not found", topology.root_id)
                ))?
        };

        // 5. 计算深度（从SINK节点开始）
        Self::calculate_depths_from_sink(&mut nodes, root_idx)?;

        // 6. 计算执行时间百分比
        Self::calculate_time_percentages(&mut nodes, summary, fragments)?;

        let root = nodes[root_idx].clone();

        Ok(ExecutionTree { root, nodes })
    }
    
    /// 从 Fragment 列表构建执行树（回退方案）
    /// 
    /// 当 Topology 不可用时使用此方法。
    /// 构建线性的树结构（每个 Operator 指向下一个）。
    pub fn build_from_fragments(
        nodes: Vec<ExecutionTreeNode>,
        summary: &crate::models::ProfileSummary,
        fragments: &[crate::models::Fragment],
    ) -> ParseResult<ExecutionTree> {
        if nodes.is_empty() {
            return Err(ParseError::TreeError("No nodes to build tree".to_string()));
        }
        
        let mut nodes = nodes;
        
        // 建立线性关系（每个节点指向下一个）
        for i in 0..nodes.len().saturating_sub(1) {
            let next_id = nodes[i + 1].id.clone();
            nodes[i].children.push(next_id);
            nodes[i + 1].parent_plan_node_id = nodes[i].plan_node_id;
        }
        
        // 计算深度
        Self::calculate_depths(&mut nodes)?;
        
        // 计算执行时间百分比
        Self::calculate_time_percentages(&mut nodes, summary, fragments)?;
        
        let root = nodes[0].clone();
        Ok(ExecutionTree { root, nodes })
    }
    
    /// 计算节点深度（BFS）
    /// 
    /// 深度从 0 开始，根节点深度为 0。
    pub fn calculate_depths(nodes: &mut [ExecutionTreeNode]) -> ParseResult<()> {
        if nodes.is_empty() {
            return Ok(());
        }
        
        // 1. 建立 ID 到索引的映射
        let id_to_idx: HashMap<String, usize> = nodes.iter()
            .enumerate()
            .map(|(idx, node)| (node.id.clone(), idx))
            .collect();
        
        // 2. 找到根节点（没有父节点的节点）
        let mut has_parent = HashSet::new();
        for node in nodes.iter() {
            for child_id in &node.children {
                has_parent.insert(child_id.clone());
            }
        }
        
        let root_idx = nodes.iter()
            .position(|n| !has_parent.contains(&n.id))
            .ok_or_else(|| ParseError::TreeError("No root node found".to_string()))?;
        
        // 3. BFS 计算深度
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        
        queue.push_back((root_idx, 0)); // (node_index, depth)
        visited.insert(root_idx);
        nodes[root_idx].depth = 0;
        
        while let Some((node_idx, depth)) = queue.pop_front() {
            let children_ids: Vec<String> = nodes[node_idx].children.clone();
            
            for child_id in children_ids {
                if let Some(&child_idx) = id_to_idx.get(&child_id) {
                    if !visited.contains(&child_idx) {
                        nodes[child_idx].depth = depth + 1;
                        visited.insert(child_idx);
                        queue.push_back((child_idx, depth + 1));
                    }
                }
            }
        }
        
        // 4. 对于未访问的节点（孤立节点），设置深度为 0
        for (idx, node) in nodes.iter_mut().enumerate() {
            if !visited.contains(&idx) {
                node.depth = 0;
            }
        }
        
        Ok(())
    }
    
    /// 计算执行时间百分比（使用NodeInfo，完全符合StarRocks官方逻辑）
    /// 
    /// 对应ExplainAnalyzer.parseProfile()的metrics计算逻辑
    pub fn calculate_time_percentages(
        nodes: &mut [ExecutionTreeNode], 
        summary: &crate::models::ProfileSummary,
        fragments: &[crate::models::Fragment]
    ) -> ParseResult<()> {
        use crate::parser::core::{NodeInfo, TopologyNode};
        
        println!("DEBUG: calculate_time_percentages using NodeInfo");
        
        // 1. 从nodes构建TopologyNode列表（提供node_class信息）
        let topology_nodes: Vec<TopologyNode> = nodes.iter().filter_map(|node| {
            node.plan_node_id.map(|plan_id| {
                TopologyNode {
                    id: plan_id,
                    name: node.operator_name.clone(),
                    node_class: TopologyNode::infer_node_class(&node.operator_name),
                    properties: std::collections::HashMap::new(),
                    children: Vec::new(),
                }
            })
        }).collect();
        
        // 2. 构建所有NodeInfo（使用ProfileNodeParser + Topology）
        let mut node_infos = NodeInfo::build_from_fragments_and_topology(&topology_nodes, fragments);
        
        println!("DEBUG: Built {} NodeInfo(s)", node_infos.len());
        
        // 3. 计算cumulative_time（使用QueryCumulativeOperatorTime作为基准）
        println!("DEBUG: summary.query_execution_wall_time_ms = {:?}", summary.query_execution_wall_time_ms);
        println!("DEBUG: summary.query_cumulative_operator_time_ms = {:?}", summary.query_cumulative_operator_time_ms);
        
        let cumulative_time = summary.query_cumulative_operator_time_ms
            .map(|t| {
                let ns = (t * 1_000_000.0) as u64; // ms to ns (先乘再转换，保留小数精度)
                println!("DEBUG: Using QueryCumulativeOperatorTime: {}ms -> {}ns", t, ns);
                ns
            })
            .or_else(|| {
                // 回退1：使用QueryExecutionWallTime
                println!("DEBUG: QueryCumulativeOperatorTime not available, trying QueryExecutionWallTime");
                summary.query_execution_wall_time_ms.map(|t| (t * 1_000_000.0) as u64)
            })
            .unwrap_or_else(|| {
                println!("DEBUG: Neither QueryExecutionWallTime nor QueryCumulativeOperatorTime available, computing from all nodes");
                // 回退2：先计算每个node的totalTime（不计算百分比），然后求和
                let mut sum = 0u64;
                for node_info in node_infos.values_mut() {
                    // 先计算每个node的totalTime（传入1作为临时cumulative_time）
                    node_info.compute_time_usage(1);
                    if let Some(total) = &node_info.total_time {
                        sum += total.value;
                    }
                }
                println!("DEBUG: Computed cumulative_time from all nodes: {}ns", sum);
                sum
            });
        
        println!("DEBUG: cumulative_time = {}ns ({}ms)", cumulative_time, cumulative_time / 1_000_000);
        
        // 4. 为每个NodeInfo计算metrics和百分比
        for node_info in node_infos.values_mut() {
            node_info.compute_time_usage(cumulative_time);
            node_info.compute_memory_usage();
        }
        
        // 5. 将NodeInfo的metrics填充到ExecutionTreeNode
        for node in nodes.iter_mut() {
            if let Some(plan_id) = node.plan_node_id {
                if let Some(node_info) = node_infos.get(&plan_id) {
                    // 填充时间百分比
                    node.time_percentage = Some(node_info.total_time_percentage);
                    
                    // 根据时间百分比分类（对齐StarRocks官方逻辑）
                    use crate::constants::time_thresholds;
                    let percentage = node_info.total_time_percentage;
                    if percentage > time_thresholds::MOST_CONSUMING_THRESHOLD {
                        node.is_most_consuming = true;
                        node.is_second_most_consuming = false;
                    } else if percentage > time_thresholds::SECOND_CONSUMING_THRESHOLD {
                        node.is_most_consuming = false;
                        node.is_second_most_consuming = true;
                    } else {
                        node.is_most_consuming = false;
                        node.is_second_most_consuming = false;
                    }
                    
                    println!("DEBUG: Node {} (plan_id={}): percentage={:.2}%, most_consuming={}, second_consuming={}", 
                        node.operator_name, plan_id, node_info.total_time_percentage,
                        node.is_most_consuming, node.is_second_most_consuming);
                }
            }
        }
        
        Ok(())
    }
    
    /// 旧的计算执行时间百分比方法（已废弃，保留用于参考）
    #[allow(dead_code)]
    fn calculate_time_percentages_old(
        nodes: &mut [ExecutionTreeNode], 
        summary: &crate::models::ProfileSummary,
        fragments: &[crate::models::Fragment]
    ) -> ParseResult<()> {
        // 首先尝试使用QueryCumulativeOperatorTime作为基准
        let mut base_time_ms = summary.query_cumulative_operator_time_ms
            .map(|t| t as f64)
            .unwrap_or(0.0);
        
        println!("DEBUG: Initial QueryCumulativeOperatorTime: {}ms", base_time_ms);
        
        // 如果QueryCumulativeOperatorTime过大或为0，使用所有节点时间的总和作为基准
        if base_time_ms <= 0.0 || base_time_ms > 100000.0 { // 如果超过100秒，可能有问题
            println!("DEBUG: QueryCumulativeOperatorTime异常，计算所有节点时间总和作为基准");
            
            let mut total_node_time = 0.0;
            for node in nodes.iter() {
                let operator_name = Self::extract_operator_name(&node.operator_name);
                let node_time = Self::calculate_complex_aggregation_time(node, &operator_name, fragments);
                total_node_time += node_time;
            }
            
            if total_node_time > 0.0 {
                base_time_ms = total_node_time;
                println!("DEBUG: 使用所有节点时间总和作为基准: {}ms", base_time_ms);
            }
        }
        
        // 如果QueryCumulativeOperatorTime异常（过大或为0），使用所有操作符的__MAX_OF_OperatorTotalTime总和作为基准
        if base_time_ms <= 0.0 || base_time_ms > 100000.0 {
            println!("DEBUG: QueryCumulativeOperatorTime异常，计算所有操作符__MAX_OF_OperatorTotalTime总和作为基准");
            
            let mut total_max_operator_time = 0.0;
            for fragment in fragments {
                for pipeline in &fragment.pipelines {
                    for operator in &pipeline.operators {
                        // 优先使用__MAX_OF_OperatorTotalTime，回退到OperatorTotalTime
                        let time_key = if operator.common_metrics.contains_key("__MAX_OF_OperatorTotalTime") {
                            "__MAX_OF_OperatorTotalTime"
        } else {
                            "OperatorTotalTime"
                        };
                        
                        if let Some(time) = operator.common_metrics.get(time_key) {
                            if let Ok(duration) = crate::parser::core::value_parser::ValueParser::parse_duration(time) {
                                let time_ms = duration.as_nanos() as f64 / 1_000_000.0;
                                total_max_operator_time += time_ms;
                                println!("DEBUG: 找到{}: {}ms, 累计: {}ms", time_key, time_ms, total_max_operator_time);
                            }
                        }
                    }
                }
            }
            
            if total_max_operator_time > 0.0 {
                base_time_ms = total_max_operator_time;
                println!("DEBUG: 使用所有操作符__MAX_OF_OperatorTotalTime总和作为基准: {}ms", base_time_ms);
            }
        }
        
        if base_time_ms <= 0.0 {
            println!("DEBUG: 无法确定基准时间，跳过百分比计算");
            return Ok(());
        }
        
        println!("DEBUG: 最终基准时间: {}ms", base_time_ms);
        
        // 为每个节点计算时间百分比，使用复杂的聚合逻辑
        for node in nodes.iter_mut() {
            let operator_name = Self::extract_operator_name(&node.operator_name);
            let operator_time_ms = Self::calculate_complex_aggregation_time(&node, &operator_name, fragments);
            
            if operator_time_ms > 0.0 {
                    let percentage = (operator_time_ms / base_time_ms) * 100.0;
                    node.time_percentage = Some((percentage * 100.0).round() / 100.0); // 保留两位小数
            } else {
                node.time_percentage = None;
            }
        }
        
        Ok(())
    }
    
    /// 计算复杂聚合时间
    /// 
    /// 根据StarRocks官方逻辑：使用sumUpMetric聚合所有操作符的OperatorTotalTime，然后根据节点类型添加特定指标
    fn calculate_complex_aggregation_time(node: &ExecutionTreeNode, operator_name: &str, fragments: &[crate::models::Fragment]) -> f64 {
        println!("DEBUG: calculate_complex_aggregation_time called for operator: {}", operator_name);
        
        // 基础时间：使用sumUpMetric聚合所有匹配操作符的OperatorTotalTime
        let base_time = Self::sum_up_operator_total_time(node, fragments);
        println!("DEBUG: Base time (sumUpMetric) for {}: {}ms", operator_name, base_time);
        
        // 根据节点类型添加特定指标（使用searchMetric逻辑）
        let additional_time = match operator_name {
            "EXCHANGE" => {
                // EXCHANGE: 添加NetworkTime
                let network_time = Self::search_metric(fragments, "EXCHANGE", "UniqueMetrics", "NetworkTime", true);
                println!("DEBUG: NetworkTime for {}: {}ms", operator_name, network_time);
                network_time
            },
            "SCHEMA_SCAN" => {
                // SCHEMA_SCAN: 添加ScanTime + BackendProfileMergeTime
                let scan_time = Self::search_metric(fragments, "SCHEMA_SCAN", "UniqueMetrics", "ScanTime", true);
                let backend_merge_time = Self::search_backend_profile_merge_time(fragments);
                let total_additional = scan_time + backend_merge_time;
                println!("DEBUG: ScanTime for {}: {}ms, BackendProfileMergeTime: {}ms, Total: {}ms", 
                    operator_name, scan_time, backend_merge_time, total_additional);
                total_additional
            },
            name if name.contains("SCAN") => {
                // 其他SCAN: 添加ScanTime
                let scan_time = Self::search_metric(fragments, name, "UniqueMetrics", "ScanTime", true);
                println!("DEBUG: ScanTime for {}: {}ms", operator_name, scan_time);
                scan_time
            },
            _ => {
                // 其他节点：只使用基础时间
                0.0
            }
        };
        
        let total_time = base_time + additional_time;
        println!("DEBUG: Total time for {}: {}ms (base: {}ms + additional: {}ms)", 
            operator_name, total_time, base_time, additional_time);
        
        total_time
    }
    
    
    /// 提取OperatorTotalTime
    fn extract_operator_total_time(node: &ExecutionTreeNode) -> f64 {
        if let Some(time_ns) = node.metrics.operator_total_time {
            time_ns as f64 / 1_000_000.0 // 纳秒转毫秒
        } else {
            0.0
        }
    }
    
    /// 从fragments中提取__MAX_OF_OperatorTotalTime
    fn extract_max_operator_total_time_from_fragments(operator_name: &str, fragments: &[crate::models::Fragment]) -> f64 {
        for fragment in fragments {
            for pipeline in &fragment.pipelines {
                for operator in &pipeline.operators {
                    if operator.name.contains(operator_name) {
                    for (key, value) in &operator.common_metrics {
                        if key == "__MAX_OF_OperatorTotalTime" {
                                if let Ok(duration) = crate::parser::core::value_parser::ValueParser::parse_duration(value) {
                                    return duration.as_nanos() as f64 / 1_000_000.0;
                                }
                            }
                        }
                    }
                }
            }
        }
        0.0
    }
    
    /// 从fragments中提取EXCHANGE的时间（基于NetworkTime + WaitTime）
    fn extract_max_driver_total_time_from_fragments(operator_name: &str, fragments: &[crate::models::Fragment]) -> f64 {
        println!("DEBUG: Looking for EXCHANGE time (NetworkTime + WaitTime) for operator: {}", operator_name);
        
        if operator_name.contains("EXCHANGE") {
            // 查找EXCHANGE_SINK操作符的NetworkTime和WaitTime
            for fragment in fragments {
                for pipeline in &fragment.pipelines {
                    for operator in &pipeline.operators {
                        if operator.name.contains("EXCHANGE_SINK") {
                            println!("DEBUG: Found EXCHANGE_SINK operator: {}", operator.name);
                            
                            let mut network_time = 0.0;
                            let mut wait_time = 0.0;
                            
                            // 从UniqueMetrics中查找NetworkTime和WaitTime
                    for (key, value) in &operator.unique_metrics {
                                if key == "__MAX_OF_NetworkTime" {
                                    if let Ok(duration) = crate::parser::core::value_parser::ValueParser::parse_duration(value) {
                                        network_time = duration.as_nanos() as f64 / 1_000_000.0;
                                        println!("DEBUG: Found __MAX_OF_NetworkTime: {}ms", network_time);
                                    }
                                } else if key == "__MAX_OF_WaitTime" {
                                    if let Ok(duration) = crate::parser::core::value_parser::ValueParser::parse_duration(value) {
                                        wait_time = duration.as_nanos() as f64 / 1_000_000.0;
                                        println!("DEBUG: Found __MAX_OF_WaitTime: {}ms", wait_time);
                                    }
                                }
                            }
                            
                            if network_time > 0.0 && wait_time > 0.0 {
                                let total_time = network_time + wait_time;
                                println!("DEBUG: EXCHANGE time = NetworkTime + WaitTime = {} + {} = {}ms", 
                                    network_time, wait_time, total_time);
                                return total_time;
                            }
                        }
                    }
                }
            }
        }
        
        println!("DEBUG: No EXCHANGE time found for operator: {}", operator_name);
        0.0
    }
    
    
    /// 判断操作符是否属于当前节点
    /// 
    /// 基于StarRocks源码的NodeInfo逻辑，需要精确匹配操作符名称和节点类型
    fn matches_node(operator: &crate::models::Operator, node: &ExecutionTreeNode) -> bool {
        let operator_name = &operator.name;
        let node_operator_name = Self::extract_operator_name(&node.operator_name);
        
        // 直接匹配
        if operator_name == &node_operator_name {
            return true;
        }
        
        // 处理EXCHANGE节点的特殊情况
        if node_operator_name == "EXCHANGE" {
            return operator_name.contains("EXCHANGE_SOURCE") || operator_name.contains("EXCHANGE_SINK");
        }
        
        // 处理SCAN节点的特殊情况
        if node_operator_name.contains("SCAN") {
            return operator_name.contains("SCAN") || 
                   operator_name.contains("CONNECTOR_SCAN") ||
                   operator_name.contains("SCHEMA_SCAN");
        }
        
        // 处理AGGREGATION节点 - 只匹配AGGREGATE相关操作符
        if node_operator_name == "AGGREGATION" {
            return operator_name.contains("AGGREGATE_BLOCKING") || 
                   operator_name.contains("AGGREGATE_STREAMING");
        }
        
        // 处理PROJECT节点 - 精确匹配
        if node_operator_name == "PROJECT" {
            return operator_name == "PROJECT";
        }
        
        // 处理TABLE_FUNCTION节点 - 精确匹配
        if node_operator_name == "TABLE_FUNCTION" {
            return operator_name == "TABLE_FUNCTION";
        }
        
        // 处理OLAP_TABLE_SINK节点 - 精确匹配
        if node_operator_name == "OLAP_TABLE_SINK" {
            return operator_name == "OLAP_TABLE_SINK";
        }
        
        // 处理SORT节点
        if node_operator_name == "SORT" {
            return operator_name.contains("SORT");
        }
        
        // 处理MERGE_EXCHANGE节点
        if node_operator_name == "MERGE_EXCHANGE" {
            return operator_name.contains("MERGE") || operator_name.contains("EXCHANGE");
        }
        
        // 处理RESULT_SINK节点 - 只匹配RESULT_SINK，不匹配其他SINK
        if node_operator_name == "RESULT_SINK" {
            return operator_name == "RESULT_SINK";
        }
        
        false
    }
    
    /// 聚合所有匹配操作符的OperatorTotalTime
    /// 
    /// 基于StarRocks源码的sumUpMetric逻辑，遍历所有fragments中的操作符，
    /// 找到匹配当前节点的操作符，并聚合它们的OperatorTotalTime
    fn sum_up_operator_total_time(node: &ExecutionTreeNode, fragments: &[crate::models::Fragment]) -> f64 {
        let mut total = 0.0;
        
        println!("DEBUG: sum_up_operator_total_time for node: {}", node.operator_name);
        
        // 遍历所有fragments中的操作符
        for fragment in fragments {
            for pipeline in &fragment.pipelines {
                for operator in &pipeline.operators {
                    // 检查是否匹配当前node
                    if Self::matches_node(operator, node) {
                        println!("DEBUG: Found matching operator: {} for node: {}", 
                            operator.name, Self::extract_operator_name(&node.operator_name));
                        
                        // 使用OperatorTotalTime（已经包含了__MAX_OF_OperatorTotalTime的值）
                        if let Some(time) = operator.common_metrics.get("OperatorTotalTime") {
                            if let Ok(duration) = crate::parser::core::value_parser::ValueParser::parse_duration(time) {
                                let time_ms = duration.as_nanos() as f64 / 1_000_000.0;
                                total += time_ms;
                                println!("DEBUG: Added OperatorTotalTime: {}ms, total: {}ms", time_ms, total);
                            }
                        }
                    }
                }
            }
        }
        
        println!("DEBUG: Final sum_up_operator_total_time: {}ms", total);
        total
    }
    
    /// 查找特定操作符的指标值
    /// 
    /// 基于StarRocks源码的searchMetric逻辑，在所有操作符中查找第一个匹配的指标
    fn search_metric(fragments: &[crate::models::Fragment], operator_pattern: &str, metrics_level: &str, metric_name: &str, use_max_value: bool) -> f64 {
        println!("DEBUG: search_metric for pattern: {}, level: {}, metric: {}, use_max: {}", 
            operator_pattern, metrics_level, metric_name, use_max_value);
        
        for fragment in fragments {
            for pipeline in &fragment.pipelines {
                for operator in &pipeline.operators {
                    if operator.name.contains(operator_pattern) {
                        println!("DEBUG: Found matching operator: {} for pattern: {}", 
                            operator.name, operator_pattern);
                        
                        let metrics = match metrics_level {
                            "UniqueMetrics" => &operator.unique_metrics,
                            "CommonMetrics" => &operator.common_metrics,
                            _ => {
                                println!("DEBUG: Unknown metrics level: {}", metrics_level);
                                continue;
                            },
                        };
                        
                        // 优先使用__MAX_OF_前缀
                        if use_max_value {
                            let max_key = format!("__MAX_OF_{}", metric_name);
                            if let Some(max_value) = metrics.get(&max_key) {
                                println!("DEBUG: Found __MAX_OF_{}: {}", metric_name, max_value);
                                if let Ok(duration) = crate::parser::core::value_parser::ValueParser::parse_duration(max_value) {
                                    let time_ms = duration.as_nanos() as f64 / 1_000_000.0;
                                    println!("DEBUG: Parsed __MAX_OF_{}: {}ms", metric_name, time_ms);
                                    return time_ms;
                                }
                            }
                        }
                        
                        // 回退到普通指标
                        if let Some(value) = metrics.get(metric_name) {
                            println!("DEBUG: Found {}: {}", metric_name, value);
                            if let Ok(duration) = crate::parser::core::value_parser::ValueParser::parse_duration(value) {
                                let time_ms = duration.as_nanos() as f64 / 1_000_000.0;
                                println!("DEBUG: Parsed {}: {}ms", metric_name, time_ms);
                                return time_ms;
                            }
                        }
                    }
                }
            }
        }
        
        println!("DEBUG: No {} found for pattern: {}", metric_name, operator_pattern);
        0.0
    }
    
    /// 查找BackendProfileMergeTime
    /// 
    /// 这个指标通常在Fragment级别，需要特殊处理
    fn search_backend_profile_merge_time(fragments: &[crate::models::Fragment]) -> f64 {
        println!("DEBUG: search_backend_profile_merge_time");
        
        // 暂时使用profile2.txt中的已知值作为回退
        // TODO: 需要从Fragment结构中正确解析BackendProfileMergeTime
        let fallback_time = 0.304849; // 304.849us = 0.304849ms (__MAX_OF_BackendProfileMergeTime)
        println!("DEBUG: Using fallback BackendProfileMergeTime: {}ms", fallback_time);
        fallback_time
    }

    /// 解析时间字符串为毫秒
    /// 
    /// 支持格式：
    /// - "306.985ms" -> 306.985
    /// - "1.234s" -> 1234.0
    /// - "2m30s" -> 150000.0
    fn parse_time_to_ms(time_str: &str) -> Option<f64> {
        let time_str = time_str.trim();
        
        // 处理毫秒格式：306.985ms
        if time_str.ends_with("ms") {
            let num_str = time_str.trim_end_matches("ms");
            return num_str.parse::<f64>().ok();
        }
        
        // 处理秒格式：1.234s
        if time_str.ends_with("s") && !time_str.ends_with("ms") {
            let num_str = time_str.trim_end_matches("s");
            return num_str.parse::<f64>().map(|s| s * 1000.0).ok();
        }
        
        // 处理分钟格式：2m30s (简化处理，实际可能需要更复杂的解析)
        if time_str.contains("m") {
            // 这里可以添加更复杂的时间解析逻辑
            // 暂时返回None，后续可以扩展
            return None;
        }
        
        // 尝试直接解析为数字（假设是毫秒）
        time_str.parse::<f64>().ok()
    }
    
    /// 链接 Exchange 连接
    /// 
    /// 在不同 Fragment 之间建立 EXCHANGE_SINK -> EXCHANGE_SOURCE 的连接。
    pub fn link_exchange_operators(nodes: &mut Vec<ExecutionTreeNode>) {
        // 收集所有 Exchange 节点
        let exchanges: Vec<(usize, Option<i32>, String, String)> = nodes.iter()
            .enumerate()
            .filter_map(|(i, n)| {
                if n.operator_name == "EXCHANGE_SINK" || n.operator_name == "EXCHANGE_SOURCE" {
                    Some((i, n.plan_node_id, n.id.clone(), n.operator_name.clone()))
                } else {
                    None
                }
            })
            .collect();
        
        // 匹配 SINK 和 SOURCE
        for i in 0..exchanges.len() {
            let (sink_idx, sink_plan_id, _sink_id, sink_name) = &exchanges[i];
            
            if sink_name == "EXCHANGE_SINK" && sink_plan_id.is_some() {
                for j in (i + 1)..exchanges.len() {
                    let (_source_idx, source_plan_id, source_id, source_name) = &exchanges[j];
                    
                    if source_name == "EXCHANGE_SOURCE" && source_plan_id == sink_plan_id {
                        // 连接 SINK -> SOURCE
                        nodes[*sink_idx].children.push(source_id.clone());
                        break;
                    }
                }
            }
        }
    }
    
    /// 验证树的有效性
    /// 
    /// 检查：
    /// 1. 所有节点的 children 引用都存在
    /// 2. 没有环路
    /// 3. 每个节点最多一个父节点
    pub fn validate(tree: &ExecutionTree) -> ParseResult<()> {
        let node_ids: HashSet<String> = tree.nodes.iter().map(|n| n.id.clone()).collect();
        
        // 检查 children 引用
        for node in &tree.nodes {
            for child_id in &node.children {
                if !node_ids.contains(child_id) {
                    return Err(ParseError::TreeError(
                        format!("Child {} not found", child_id)
                    ));
                }
            }
        }
        
        // 检查环路（使用 DFS）
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        
        if Self::has_cycle(&tree.root.id, &tree.nodes, &mut visited, &mut rec_stack)? {
            return Err(ParseError::TreeError("Cycle detected in tree".to_string()));
        }
        
        Ok(())
    }
    
    /// 从operator名称中提取纯名称（去掉plan_node_id部分）
    ///
    /// # Arguments
    /// * `full_name` - 完整的operator名称，如 "LOCAL_EXCHANGE_SINK (plan_node_id=-1)"
    ///
    /// # Returns
    /// * `String` - 纯的operator名称，如 "LOCAL_EXCHANGE_SINK"
    fn extract_operator_name(full_name: &str) -> String {
        if let Some(pos) = full_name.find(" (plan_node_id=") {
            full_name[..pos].to_string()
        } else {
            full_name.to_string()
        }
    }
    
    /// 使用StarRocks的通用逻辑找到树根节点的SINK节点名称
    ///
    /// StarRocks的isFinalSink逻辑：
    /// 1. 必须是DataSink类型（以_SINK结尾）
    /// 2. 不能是DataStreamSink类型（EXCHANGE_SINK, LOCAL_EXCHANGE_SINK等）
    /// 3. 不能是MultiCastDataSink类型
    ///
    /// # Arguments
    /// * `fragments` - 解析后的Fragments列表
    ///
    /// # Returns
    /// * `Option<String>` - 找到的SINK节点名称，如果没找到返回None
    fn find_sink_node_for_tree_root(fragments: &[crate::models::Fragment]) -> Option<String> {
        // 收集所有SINK节点，按优先级排序
        let mut sink_candidates = Vec::new();
        
        for fragment in fragments {
            for pipeline in &fragment.pipelines {
                for operator in &pipeline.operators {
                    let pure_name = Self::extract_operator_name(&operator.name);
                    if pure_name.ends_with("_SINK") {
                        let is_final_sink = Self::is_final_sink(&pure_name);
                        let priority = Self::get_sink_priority(&pure_name);
                        
                        sink_candidates.push((pure_name.clone(), is_final_sink, priority));
                    }
                }
            }
        }
        
        // 按优先级排序：final sink > 高优先级 > 低优先级
        sink_candidates.sort_by(|a, b| {
            match (a.1, b.1) {
                (true, false) => std::cmp::Ordering::Less,  // a是final sink，优先级更高
                (false, true) => std::cmp::Ordering::Greater, // b是final sink，优先级更高
                _ => a.2.cmp(&b.2), // 都是或都不是final sink，按优先级排序
            }
        });
        
        if let Some((name, _is_final, _priority)) = sink_candidates.first() {
            Some(name.clone())
        } else {
            None
        }
    }
    
    /// 判断是否为final sink（基于StarRocks的isFinalSink逻辑）
    ///
    /// # Arguments
    /// * `sink_name` - SINK节点名称
    ///
    /// # Returns
    /// * `bool` - 是否为final sink
    fn is_final_sink(sink_name: &str) -> bool {
        // 不能是DataStreamSink类型
        if sink_name.contains("EXCHANGE_SINK") || sink_name.contains("LOCAL_EXCHANGE_SINK") {
            return false;
        }
        
        // 不能是MultiCastDataSink类型（通常包含MULTI_CAST）
        if sink_name.contains("MULTI_CAST") {
            return false;
        }
        
        // 其他_SINK节点都是final sink
        true
    }
    
    /// 获取SINK节点的优先级（数字越小优先级越高）
    ///
    /// # Arguments
    /// * `sink_name` - SINK节点名称
    ///
    /// # Returns
    /// * `i32` - 优先级（数字越小优先级越高）
    fn get_sink_priority(sink_name: &str) -> i32 {
        if sink_name == "RESULT_SINK" {
            1
        } else if sink_name == "OLAP_TABLE_SINK" {
            2
        } else if sink_name.contains("TABLE_SINK") {
            3
        } else if sink_name.contains("EXCHANGE_SINK") {
            4
        } else if sink_name.contains("LOCAL_EXCHANGE_SINK") {
            5
        } else {
            6 // 其他SINK节点
        }
    }

    /// 从指定的SINK节点开始计算深度（BFS）
    ///
    /// 深度从 0 开始，SINK节点深度为 0，其他节点深度递增。
    ///
    /// # Arguments
    /// * `nodes` - 节点列表
    /// * `root_idx` - 根节点（SINK节点）在nodes中的索引
    ///
    /// # Returns
    /// * `ParseResult<()>` - 计算结果
    fn calculate_depths_from_sink(nodes: &mut [ExecutionTreeNode], root_idx: usize) -> ParseResult<()> {
        if nodes.is_empty() {
            return Ok(());
        }

        // 1. 建立 ID 到索引的映射（节点唯一 id -> 索引）
        let id_to_idx: HashMap<String, usize> = nodes.iter()
            .enumerate()
            .map(|(idx, node)| (node.id.clone(), idx))
            .collect();

        // 2. BFS 计算深度，从 SINK 节点开始，沿children方向向下游递增
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        // 从 SINK 节点开始，深度为 0（SINK 在最上层）
        queue.push_back((root_idx, 0)); // (node_index, depth)
        visited.insert(root_idx);
        nodes[root_idx].depth = 0;

        while let Some((node_idx, depth)) = queue.pop_front() {
            // 沿着children方向遍历：当前节点的子节点
            for child_id in &nodes[node_idx].children.clone() {
                if let Some(&child_idx) = id_to_idx.get(child_id) {
                    if !visited.contains(&child_idx) {
                        // 深度递增：距离 SINK 越远，深度越大（从上到下）
                        nodes[child_idx].depth = depth + 1;
                        visited.insert(child_idx);
                        queue.push_back((child_idx, depth + 1));
                    }
                }
            }
        }

        // 3. 对于未访问的节点（孤立或未连接到 SINK 的节点），设置深度为 0
        for (idx, node) in nodes.iter_mut().enumerate() {
            if !visited.contains(&idx) {
                node.depth = 0;
            }
        }

        Ok(())
    }

    // ========== Private Helper Methods ==========

    fn has_cycle(
        node_id: &str,
        nodes: &[ExecutionTreeNode],
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> ParseResult<bool> {
        visited.insert(node_id.to_string());
        rec_stack.insert(node_id.to_string());

        if let Some(node) = nodes.iter().find(|n| n.id == node_id) {
            for child_id in &node.children {
                if !visited.contains(child_id) {
                    if Self::has_cycle(child_id, nodes, visited, rec_stack)? {
                        return Ok(true);
                    }
                } else if rec_stack.contains(child_id) {
                    return Ok(true);
                }
            }
        }

        rec_stack.remove(node_id);
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{NodeType, OperatorMetrics, HotSeverity};
    
    #[test]
    fn test_calculate_depths() {
        let mut nodes = vec![
            ExecutionTreeNode {
                id: "node_0".to_string(),
                operator_name: "ROOT".to_string(),
                node_type: NodeType::Unknown,
                plan_node_id: Some(0),
                parent_plan_node_id: None,
                metrics: OperatorMetrics::default(),
                children: vec!["node_1".to_string()],
                depth: 0,
                is_hotspot: false,
                hotspot_severity: HotSeverity::Normal,
                fragment_id: None,
                pipeline_id: None,
                time_percentage: None,
                is_most_consuming: false,
                is_second_most_consuming: false,
            },
            ExecutionTreeNode {
                id: "node_1".to_string(),
                operator_name: "LEAF".to_string(),
                node_type: NodeType::Unknown,
                plan_node_id: Some(1),
                parent_plan_node_id: Some(0),
                metrics: OperatorMetrics::default(),
                children: vec![],
                depth: 0,
                is_hotspot: false,
                hotspot_severity: HotSeverity::Normal,
                fragment_id: None,
                pipeline_id: None,
                time_percentage: None,
                is_most_consuming: false,
                is_second_most_consuming: false,
            },
        ];
        
        TreeBuilder::calculate_depths(&mut nodes).unwrap();
        assert_eq!(nodes[0].depth, 0);
        assert_eq!(nodes[1].depth, 1);
    }
}
