use crate::models::{
    ExecutionTreeNode, Fragment, HotSeverity,
    Profile, ProfileSummary,
};
use crate::parser::error::{ParseError, ParseResult};
use crate::parser::core::{
    ValueParser, TopologyParser, TopologyGraph, OperatorParser, TreeBuilder, MetricsParser,
    section_parser::SectionParser, fragment_parser::FragmentParser,
};
use crate::parser::SpecializedMetricsParser;
use std::collections::HashMap;

/// ProfileComposer 负责协调各个解析器，将 Profile 文本转换为数据模型
///
/// 这是解析器的主入口，负责：
/// 1. 协调各个专门的解析器
/// 2. 构建完整的执行树
/// 3. 检测性能热点
/// 4. 组装最终的 Profile 数据模型
///
/// 设计原则：
/// - 完全按照 StarRocks 官方逻辑解析
/// - 优先使用 Topology 构建树结构
/// - 回退到 Fragment 线性结构
/// - 支持热点检测和瓶颈分析
#[derive(Debug, Clone)]
pub struct ProfileComposer {
    specialized_parser: SpecializedMetricsParser,
}

impl ProfileComposer {
    /// 创建新的 ProfileComposer
    pub fn new() -> Self {
        Self {
            specialized_parser: SpecializedMetricsParser::new(),
        }
    }
    
    /// 解析 Profile 文本
    ///
    /// # Arguments
    /// * `text` - 完整的 Profile 文本
    ///
    /// # Returns
    /// * `Profile` - 解析后的完整 Profile 数据模型
    pub fn parse(&mut self, text: &str) -> ParseResult<Profile> {
        // 1. 提取各个章节
        let mut summary = self.parse_summary(text)?;
        let planner_info = self.parse_planner(text)?;
        let execution_info = self.parse_execution(text)?;
        
        // 1.5. 如果Summary中没有QueryCumulativeOperatorTime，尝试从Execution中获取
        if summary.query_cumulative_operator_time_ms.is_none() {
            if let Some(qcot) = execution_info.metrics.get("QueryCumulativeOperatorTime") {
                summary.query_cumulative_operator_time_ms = ValueParser::parse_time_to_ms(qcot).ok();
            }
        }
        
        // 1.6. 如果Summary中没有QueryExecutionWallTime，尝试从Execution中获取
        if summary.query_execution_wall_time_ms.is_none() {
            if let Some(qewt) = execution_info.metrics.get("QueryExecutionWallTime") {
                summary.query_execution_wall_time_ms = ValueParser::parse_time_to_ms(qewt).ok();
                summary.query_execution_wall_time = Some(qewt.clone());
            }
        }
        
        // 2. 解析 Fragments（完全按照 SR 逻辑）
        let fragments = FragmentParser::extract_all_fragments(text);

        // 3. 解析 Topology（如果存在）
        let topology_result = Self::extract_topology_json(&execution_info.topology)
            .and_then(|json| {
                // 使用已解析的Fragments
                TopologyParser::parse_with_fragments(&json, text, &fragments)
            }).ok();
        
        // 4. 构建执行树（完全按照 SR 逻辑）
        let mut execution_tree = if let Some(ref topology) = topology_result {
            // SR 逻辑：如果有 Topology，使用 Topology 作为树骨架
            // 从所有 Fragments 中收集 Operators，按 plan_node_id 映射到 Topology 节点
            let nodes = self.build_nodes_from_topology_and_fragments(topology, &fragments)?;
            TreeBuilder::build_from_topology(topology, nodes, &fragments, &summary)?
        } else {
            // SR 逻辑：没有 Topology 时，按 Fragment 顺序线性组织
            let nodes = self.build_nodes_from_fragments(text, &fragments)?;
            TreeBuilder::build_from_fragments(nodes, &summary, &fragments)?
        };
        
        // 7. 检测热点
        // 热点检测已集成到analyzer模块中
        
        // 8. 查找瓶颈
        // 瓶颈检测已集成到analyzer模块中
        
        Ok(Profile {
            summary,
            planner: planner_info,
            execution: execution_info,
            fragments, // 使用解析出的fragments
            execution_tree: Some(execution_tree),
        })
    }

    // ========== Section Parsing ==========
    
    /// 解析 Summary 章节
    fn parse_summary(&self, text: &str) -> ParseResult<ProfileSummary> {
        SectionParser::parse_summary(text)
    }
    
    /// 解析 Planner 章节
    fn parse_planner(&self, text: &str) -> ParseResult<crate::models::PlannerInfo> {
        SectionParser::parse_planner(text)
    }
    
    /// 解析 Execution 章节
    fn parse_execution(
        &self,
        text: &str,
    ) -> ParseResult<crate::models::ExecutionInfo> {
        SectionParser::parse_execution(text)
    }
    
    // ========== Topology Extraction ==========
    
    /// 从 Execution 信息中提取 Topology JSON
    ///
    /// # Arguments
    /// * `topology_text` - 包含 Topology 的文本
    ///
    /// # Returns
    /// * `Ok(String)` - 提取的 JSON 字符串
    /// * `Err(ParseError)` - 提取失败
    fn extract_topology_json(topology_text: &str) -> ParseResult<String> {
        if topology_text.trim().is_empty() {
            return Err(ParseError::TopologyError("Empty topology text".to_string()));
        }
        
        // 查找 "Topology: " 后的 JSON 部分
        if let Some(start) = topology_text.find("Topology: ") {
            let json_start = start + "Topology: ".len();
            let json_part = &topology_text[json_start..];
            
            // 找到 JSON 的结束位置（下一行或文本结束）
            let json_end = json_part.find('\n').unwrap_or(json_part.len());
            let json = json_part[..json_end].trim();
            
            if json.is_empty() {
                return Err(ParseError::TopologyError("Empty JSON after Topology:".to_string()));
            }
            
            Ok(json.to_string())
        } else {
            // 如果没有 "Topology: " 前缀，假设整个文本就是 JSON
            Ok(topology_text.trim().to_string())
        }
    }
    
    // ========== Node Building ==========
    
    /// 从 Topology 和 Fragments 构建节点（完全符合 SR 逻辑）
    ///
    /// SR 逻辑：
    /// 1. 优先使用 Topology 中的节点结构
    /// 2. 从 Fragments 中查找对应的 Operator 指标
    /// 3. 对于 SINK 节点，需要特殊处理，因为它们可能不在 Topology 中
    fn build_nodes_from_topology_and_fragments(
        &self,
        topology: &TopologyGraph,
        fragments: &[Fragment],
    ) -> ParseResult<Vec<ExecutionTreeNode>> {
        use crate::models::{ExecutionTreeNode, HotSeverity, NodeType, OperatorMetrics};
        use std::collections::HashMap;
    
        // 1. 收集所有 Fragments 中的 Operators，按 plan_node_id 分组
        let mut operators_by_plan_id: HashMap<i32, Vec<(&crate::models::Operator, String, String)>> = HashMap::new();
        for fragment in fragments {
            for pipeline in &fragment.pipelines {
                for operator in &pipeline.operators {
                    if let Some(plan_id) = &operator.plan_node_id {
                        if let Ok(plan_id_int) = plan_id.parse::<i32>() {
                            operators_by_plan_id
                                .entry(plan_id_int)
                                .or_default()
                                .push((operator, fragment.id.clone(), pipeline.id.clone()));
                        }
                    }
                }
            }
        }
 
        // 2. 为拓扑图中的每个节点创建 ExecutionTreeNode，选择与 Topology 名称最匹配的 Operator
        let mut nodes = Vec::new();
        for topo_node in &topology.nodes {
            let tree_node = if let Some(op_list) = operators_by_plan_id.get(&topo_node.id) {
                // 针对同一个 plan_node_id 的多个 operators，聚合相同类型的操作符
                let op_refs: Vec<&crate::models::Operator> = op_list.iter().map(|(op,_,_)| *op).collect();
                let aggregated_op = Self::aggregate_operators(&op_refs, &topo_node.name);
                
                // 找到第一个匹配的 operator 对应的 fragment/pipeline（用于获取fragment和pipeline信息）
                let (frag_id, pipe_id) = {
                    // 使用第一个操作符的fragment和pipeline信息
                    if let Some((_, f, p)) = op_list.first() {
                        (Some(f.clone()), Some(p.clone()))
                    } else {
                        (None, None)
                    }
                };
                
                // 创建节点
                let mut metrics = MetricsParser::from_hashmap(&aggregated_op.common_metrics);

                // 解析 UniqueMetrics (specialized metrics)
                if !aggregated_op.unique_metrics.is_empty() {
                    let specialized_parser = SpecializedMetricsParser::new();

                    // 提取操作符名称（去掉plan_node_id部分）
                    let pure_name = Self::extract_operator_name(&aggregated_op.name);
                    let unique_text = Self::build_unique_metrics_text(&aggregated_op.unique_metrics);
                    metrics.specialized = specialized_parser.parse(&pure_name, &unique_text);
                }

                ExecutionTreeNode {
                    id: format!("node_{}", topo_node.id),
                    plan_node_id: Some(topo_node.id),
                    operator_name: topo_node.name.clone(), // 使用Topology中的名称，而不是实际Operator的名称
                    node_type: OperatorParser::determine_node_type(&aggregated_op.name),
                    parent_plan_node_id: None, // 将在 TreeBuilder 中设置
                    children: Vec::new(),      // 将在 TreeBuilder 中设置
                    depth: 0,                  // 将在 TreeBuilder 中计算
                    metrics,
                    is_hotspot: false,
                    hotspot_severity: HotSeverity::Normal,
                    fragment_id: frag_id,
                    pipeline_id: pipe_id,
                    time_percentage: None,
                    is_most_consuming: false,
                    is_second_most_consuming: false,
                }
            } else {
                // 如果在 Fragments 中找不到对应的 Operator，创建一个基本节点
                ExecutionTreeNode {
                    id: format!("node_{}", topo_node.id),
                    plan_node_id: Some(topo_node.id),
                    operator_name: topo_node.name.clone(),
                    node_type: OperatorParser::determine_node_type(&topo_node.name),
                    parent_plan_node_id: None,
                    children: Vec::new(),
                    depth: 0,
                    metrics: OperatorMetrics::default(),
                    is_hotspot: false,
                    hotspot_severity: HotSeverity::Normal,
                    fragment_id: None,
                    pipeline_id: None,
                    time_percentage: None,
                    is_most_consuming: false,
                    is_second_most_consuming: false,
                }
            };

            nodes.push(tree_node);
        }

        // 3. 添加 SINK 节点（这些节点通常不在 Topology 中，但需要作为树的根节点）
        let mut sink_nodes = Vec::new();
        let mut next_sink_id = -1; // SINK节点使用负数ID
        
        for fragment in fragments {
            for pipeline in &fragment.pipelines {
                for operator in &pipeline.operators {
                    let pure_name = Self::extract_operator_name(&operator.name);
                    
                    // 只处理SINK节点，且不在topology中的节点
                    if pure_name.ends_with("_SINK") {
                        // 检查是否已经在topology中
                        let plan_id = operator.plan_node_id.as_ref()
                            .and_then(|id| id.parse::<i32>().ok())
                            .unwrap_or(next_sink_id);
                        
                        // 如果不在topology中，添加为新节点
                        if !topology.nodes.iter().any(|n| n.id == plan_id) {
                            let mut metrics = MetricsParser::from_hashmap(&operator.common_metrics);
                            
                            // 解析 UniqueMetrics
                            if !operator.unique_metrics.is_empty() {
                                let specialized_parser = SpecializedMetricsParser::new();
                                let unique_text = Self::build_unique_metrics_text(&operator.unique_metrics);
                                metrics.specialized = specialized_parser.parse(&pure_name, &unique_text);
                            }
                            
                            let sink_node = ExecutionTreeNode {
                                id: format!("sink_{}", plan_id.abs()),
                                plan_node_id: Some(plan_id),
                                operator_name: pure_name.clone(),
                                node_type: OperatorParser::determine_node_type(&pure_name),
                                parent_plan_node_id: None,
                                children: Vec::new(),
                                depth: 0,
                                metrics,
                                is_hotspot: false,
                            hotspot_severity: HotSeverity::Normal,
                            fragment_id: Some(fragment.id.clone()),
                            pipeline_id: Some(pipeline.id.clone()),
                            time_percentage: None,
                            is_most_consuming: false,
                            is_second_most_consuming: false,
                        };
                            
                            sink_nodes.push(sink_node);
                            next_sink_id -= 1;
                        }
                    }
                }
            }
        }
        
        // 4. 合并topology节点和sink节点
        nodes.extend(sink_nodes);
        
        Ok(nodes)
    }

    /// 从 UniqueMetrics HashMap 构建文本
    fn build_unique_metrics_text(unique_metrics: &HashMap<String, String>) -> String {
        unique_metrics
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// 提取操作符名称（去掉 plan_node_id 部分）
    ///
    /// 例如：
    /// - "OLAP_TABLE_SINK (plan_node_id=1)" -> "OLAP_TABLE_SINK"
    /// - "PROJECT (plan_node_id=2)" -> "PROJECT"
    fn extract_operator_name(full_name: &str) -> String {
        if let Some(pos) = full_name.find(" (plan_node_id=") {
            full_name[..pos].trim().to_string()
        } else {
            full_name.trim().to_string()
        }
    }

    /// 聚合相同类型的操作符
    ///
    /// 按照 StarRocks 逻辑：
    /// 1. 当多个操作符被规范化为同一个名称时（如EXCHANGE_SINK和EXCHANGE_SOURCE都变成EXCHANGE），
    ///    需要聚合它们的时间而不是只选择一个
    /// 2. 聚合所有匹配操作符的operator_total_time
    /// 3. 选择第一个匹配的操作符作为基础，但使用聚合后的时间
    fn aggregate_operators(operators: &[&crate::models::Operator], topology_name: &str) -> crate::models::Operator {
        if operators.is_empty() {
            panic!("Empty operators list");
        }

        println!("DEBUG: aggregate_operators called for topology_name: {}", topology_name);
        println!("DEBUG: Available operators: {:?}", operators.iter().map(|op| &op.name).collect::<Vec<_>>());
        
        // 找到所有匹配的操作符
        let mut matching_operators = Vec::new();
        
        // 1. 尝试 canonical_name 完全匹配
        for &op in operators {
            let op_name = Self::extract_operator_name(&op.name);
            let op_canonical = crate::parser::core::OperatorParser::canonical_topology_name(&op_name);
            println!("DEBUG: Checking operator '{}' -> name: '{}' -> canonical: '{}' against topology: '{}'", op.name, op_name, op_canonical, topology_name);
            if op_canonical == topology_name {
                matching_operators.push(op);
                println!("DEBUG: Found matching operator: {}", op.name);
            }
        }

        // 2. 如果没有完全匹配，尝试标准化名称匹配
        if matching_operators.is_empty() {
            let normalized_topology = topology_name.to_uppercase().replace("-", "_");
            for &op in operators {
                let op_name = Self::extract_operator_name(&op.name);
                let op_normalized = op_name.to_uppercase().replace("-", "_");
                if op_normalized == normalized_topology {
                    matching_operators.push(op);
                }
            }
        }

        // 3. 特殊处理：对于 OLAP_SCAN，优先选择 CONNECTOR_SCAN
        if matching_operators.is_empty() && topology_name == "OLAP_SCAN" {
            for &op in operators {
                let op_name = Self::extract_operator_name(&op.name);
                let op_canonical = crate::parser::core::OperatorParser::canonical_topology_name(&op_name);
                if op_canonical == "CONNECTOR_SCAN" {
                    matching_operators.push(op);
                }
            }
        }

        // 4. 如果还是没有匹配，使用第一个
        if matching_operators.is_empty() {
            matching_operators.push(operators[0]);
        }

        // 5. 聚合时间：将所有匹配操作符的时间相加
        let mut base_operator = matching_operators[0].clone();
        
        println!("DEBUG: Found {} matching operators for {}", matching_operators.len(), topology_name);
        
        // 聚合 operator_total_time
        let mut total_time_ns: u64 = 0;
        for &op in &matching_operators {
            if let Some(time_str) = op.common_metrics.get("OperatorTotalTime") {
                println!("DEBUG: Processing operator '{}' with OperatorTotalTime: '{}'", op.name, time_str);
                // 解析时间字符串为纳秒
                if let Some(time_ms) = Self::parse_time_to_ms(time_str) {
                    let time_ns = (time_ms * 1_000_000.0) as u64;
                    total_time_ns += time_ns; // 毫秒转纳秒
                    println!("DEBUG: Parsed time: {}ms -> {}ns, running total: {}ns", time_ms, time_ns, total_time_ns);
                } else {
                    println!("DEBUG: Failed to parse time string: '{}'", time_str);
                }
            } else {
                println!("DEBUG: Operator '{}' has no OperatorTotalTime", op.name);
            }
        }
        
        // 更新聚合后的时间
        if total_time_ns > 0 {
            let total_time_ms = total_time_ns as f64 / 1_000_000.0;
            println!("DEBUG: Final aggregated time for {}: {}ms ({}ns)", topology_name, total_time_ms, total_time_ns);
            base_operator.common_metrics.insert(
                "OperatorTotalTime".to_string(), 
                format!("{}ms", total_time_ms)
            );
        } else {
            println!("DEBUG: No time to aggregate for {}", topology_name);
        }

        // 聚合其他可能的时间指标
        let time_metrics = ["PushTotalTime", "PullTotalTime"];
        for metric_name in &time_metrics {
            let mut total_time_ns: u64 = 0;
            for &op in &matching_operators {
                if let Some(time_str) = op.common_metrics.get(*metric_name) {
                    if let Some(time_ms) = Self::parse_time_to_ms(time_str) {
                        total_time_ns += (time_ms * 1_000_000.0) as u64;
                    }
                }
            }
            if total_time_ns > 0 {
                let total_time_ms = total_time_ns as f64 / 1_000_000.0;
                base_operator.common_metrics.insert(
                    metric_name.to_string(), 
                    format!("{}ms", total_time_ms)
                );
            }
        }

        // 聚合数量指标（如果有的话）
        let count_metrics = ["PushChunkNum", "PushRowNum", "PullChunkNum", "PullRowNum"];
        for metric_name in &count_metrics {
            let mut total_count: u64 = 0;
            for &op in &matching_operators {
                if let Some(count_str) = op.common_metrics.get(*metric_name) {
                    if let Ok(count) = count_str.parse::<u64>() {
                        total_count += count;
                    }
                }
            }
            if total_count > 0 {
                base_operator.common_metrics.insert(
                    metric_name.to_string(), 
                    total_count.to_string()
                );
            }
        }

        base_operator
    }

    /// 解析时间字符串为毫秒
    /// 
    /// 支持格式：
    /// - "306.985ms" -> 306.985
    /// - "12.237us" -> 0.012237
    /// - "1.234s" -> 1234.0
    /// - "500ns" -> 0.0005
    /// - "2m30s" -> 150000.0
    fn parse_time_to_ms(time_str: &str) -> Option<f64> {
        let time_str = time_str.trim();
        
        // 处理毫秒格式：306.985ms
        if time_str.ends_with("ms") {
            let num_str = time_str.trim_end_matches("ms");
            return num_str.parse::<f64>().ok();
        }
        
        // 处理微秒格式：12.237us
        if time_str.ends_with("us") {
            let num_str = time_str.trim_end_matches("us");
            return num_str.parse::<f64>().map(|us| us / 1000.0).ok();
        }
        
        // 处理纳秒格式：500ns
        if time_str.ends_with("ns") {
            let num_str = time_str.trim_end_matches("ns");
            return num_str.parse::<f64>().map(|ns| ns / 1_000_000.0).ok();
        }
        
        // 处理秒格式：1.234s
        if time_str.ends_with("s") && !time_str.ends_with("ms") && !time_str.ends_with("us") && !time_str.ends_with("ns") {
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

    /// 从 Fragment 列表构建节点（回退方案）
    ///
    /// 当 Topology 不可用时使用此方法。
    /// 构建线性的树结构（每个 Operator 指向下一个）。
    fn build_nodes_from_fragments(
        &self,
        text: &str,
        fragments: &[Fragment],
    ) -> ParseResult<Vec<ExecutionTreeNode>> {
        let mut nodes = Vec::new();
        let mut node_counter = 0;
        
        for fragment in fragments {
            // 遍历 Fragment 中的所有 Pipelines
            for pipeline in &fragment.pipelines {
                for operator in &pipeline.operators {
                    let plan_id_i32 = operator
                        .plan_node_id
                        .as_ref()
                        .and_then(|s| s.parse::<i32>().ok())
                        .unwrap_or(node_counter);
                    
                    let operator_text = Self::find_operator_text_by_plan_id(text, &operator.name, plan_id_i32);
                    if operator_text.is_empty() {
                        continue;
                    }

                    let node = self.parse_operator_to_node(
                        &operator_text,
                        &operator.name,
                        plan_id_i32,
                        Some(fragment.id.clone()),
                        Some(pipeline.id.clone()),
                    )?;
                    nodes.push(node);
                    node_counter += 1;
                }
            }
        }
        
        Ok(nodes)
    }

    /// 在 Profile 文本中查找特定 Operator 的文本块
    fn find_operator_text(text: &str, operator_name: &str) -> String {
        let lines: Vec<&str> = text.lines().collect();
        let mut result = Vec::new();
        let mut in_operator = false;
        let mut indent_level = 0;

        for line in lines {
            if line.trim().starts_with(operator_name) {
                in_operator = true;
                indent_level = line.len() - line.trim_start().len();
                result.push(line);
            } else if in_operator {
                let current_indent = line.len() - line.trim_start().len();
                if line.trim().is_empty() || current_indent > indent_level {
                    result.push(line);
                } else {
                    break;
                }
            }
        }

        result.join("\n")
    }

    /// Find operator text by plan_node_id for precise matching
    fn find_operator_text_by_plan_id(text: &str, operator_name: &str, plan_node_id: i32) -> String {
        // Use OperatorParser's extract_operator_block for precise matching
        OperatorParser::extract_operator_block(text, operator_name, Some(plan_node_id))
    }

    /// 将 Operator 文本解析为 ExecutionTreeNode
    fn parse_operator_to_node(
        &self,
        operator_text: &str,
        operator_name: &str,
        plan_node_id: i32,
        fragment_id: Option<String>,
        pipeline_id: Option<String>,
    ) -> ParseResult<ExecutionTreeNode> {
        // 解析基础指标
        let mut metrics = MetricsParser::parse_common_metrics(operator_text);

        // 解析专门指标
        let pure_name = Self::extract_operator_name(operator_name);
        metrics.specialized = self.specialized_parser.parse(&pure_name, operator_text);

        Ok(ExecutionTreeNode {
            id: format!("node_{}", plan_node_id),
            plan_node_id: Some(plan_node_id),
            operator_name: pure_name.clone(),
            node_type: OperatorParser::determine_node_type(&pure_name),
            parent_plan_node_id: None,
            children: Vec::new(),
            depth: 0,
            metrics,
            is_hotspot: false,
            hotspot_severity: HotSeverity::Normal,
            fragment_id,
            pipeline_id,
            time_percentage: None,
            is_most_consuming: false,
            is_second_most_consuming: false,
        })
    }
}

impl Default for ProfileComposer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extract_topology_json() {
        let text = r#"  - Topology: {"rootId": 1, "nodes": [{"id": 1, "name": "TEST"}]}"#;
        let json = ProfileComposer::extract_topology_json(text).unwrap();
        assert!(json.contains("rootId"));
    }
}
