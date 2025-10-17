//! # Composer - 解析器组合器
//! 
//! 负责协调所有解析器，完成从 Profile 文本到完整数据模型的转换。
//! 
//! ## 解析流程
//! 1. 提取 Summary（使用 SummaryParser）
//! 2. 提取 Planner（使用 PlannerParser）
//! 3. 提取 Execution（使用 ExecutionParser）
//!    - 解析 Topology（使用 TopologyParser）
//! 4. 提取 Fragments（使用 FragmentParser）
//!    - 解析每个 Fragment 的 Pipelines 和 Operators
//! 5. 解析 Operator 指标（使用 MetricsParser 和 SpecializedMetricsParser）
//! 6. 构建执行树（使用 TreeBuilder）
//! 7. 检测热点（使用 HotspotDetector）
//! 8. 组装最终的 Profile 数据模型

use crate::models::{
    ExecutionInfo, ExecutionTreeNode, OperatorMetrics,
    PlannerInfo, Profile, ProfileSummary, Fragment, Operator,
};
use crate::parser::error::ParseResult;
use crate::parser::core::fragment_parser::FragmentParser;
use crate::parser::core::topology_parser::{TopologyParser, TopologyGraph};
use crate::parser::core::tree_builder::TreeBuilder;
use crate::parser::analysis::hotspot_detector::{HotspotConfig, HotspotDetector};
use crate::parser::core::metrics_parser::MetricsParser;
use crate::parser::core::operator_parser::OperatorParser;
use crate::parser::specialized::SpecializedMetricsParser;
use crate::parser::core::value_parser::ValueParser;
use std::collections::HashMap;

pub struct ProfileComposer {
    specialized_parser: SpecializedMetricsParser,
    hotspot_config: HotspotConfig,
}

impl ProfileComposer {
    /// 创建新的 ProfileComposer
    pub fn new() -> Self {
        Self {
            specialized_parser: SpecializedMetricsParser::new(),
            hotspot_config: HotspotConfig::default(),
        }
    }
    
    /// 使用自定义热点检测配置
    pub fn with_hotspot_config(mut self, config: HotspotConfig) -> Self {
        self.hotspot_config = config;
        self
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
        let execution_info = self.parse_execution(text, &mut summary)?;
        
        // 2. 解析 Fragments（完全按照 SR 逻辑）
        let fragments = FragmentParser::extract_all_fragments(text);

        // 3. 解析 Topology（如果存在）
        let topology_result = Self::extract_topology_json(&execution_info.topology)
            .and_then(|json| {
                // 使用已解析的Fragments
                TopologyParser::parse_with_fragments(&json, text, &fragments).ok()
            });
        
        // 4. 构建执行树（完全按照 SR 逻辑）
        let mut execution_tree = if let Some(ref topology) = topology_result {
            // SR 逻辑：如果有 Topology，使用 Topology 作为树骨架
            // 从所有 Fragments 中收集 Operators，按 plan_node_id 映射到 Topology 节点
            let nodes = self.build_nodes_from_topology_and_fragments(topology, &fragments)?;
            TreeBuilder::build_from_topology(topology, nodes, &fragments)?
        } else {
            // SR 逻辑：没有 Topology 时，按 Fragment 顺序线性组织
            let nodes = self.build_nodes_from_fragments(text, &fragments)?;
            TreeBuilder::build_from_fragments(nodes)?
        };
        
        // 7. 检测热点
        HotspotDetector::detect(&mut execution_tree.nodes, self.hotspot_config.clone());
        
        // 8. 查找瓶颈
        let _bottlenecks = HotspotDetector::find_bottlenecks(&execution_tree, &self.hotspot_config);
        
        Ok(Profile {
            summary,
            planner: planner_info,
            execution: execution_info,
            fragments: Vec::new(), // TODO: 从 fragments 变量填充
            execution_tree: Some(execution_tree),
        })
    }
    
    // ========== Section Parsers ==========
    
    fn parse_summary(&self, text: &str) -> ParseResult<ProfileSummary> {
        let query_section = Self::extract_section(text, "Query:", "Planner:");
        let mut summary = ProfileSummary::default();
        
        for line in query_section.lines() {
            let trimmed = line.trim();
            if let Some((key, value)) = Self::parse_kv_line(trimmed) {
                match key {
                    "Query ID" | "QueryId" => summary.query_id = value.to_string(),
                    "Start Time" | "StartTime" => summary.start_time = value.to_string(),
                    "End Time" | "EndTime" => summary.end_time = value.to_string(),
                    "Total" => {
                        summary.total_time = value.to_string();
                        summary.total_time_ms = ValueParser::parse_time_to_ms(value).ok();
                    }
                    "Query State" | "QueryState" => summary.query_state = value.to_string(),
                    "StarRocks Version" | "StarRocksVersion" => summary.starrocks_version = value.to_string(),
                    "Sql Statement" | "SqlStatement" => summary.sql_statement = value.to_string(),
                    "Query Type" | "QueryType" => summary.query_type = Some(value.to_string()),
                    "User" => summary.user = Some(value.to_string()),
                    "Default Db" | "DefaultDb" => summary.default_db = Some(value.to_string()),
                    "Query Allocated Memory" | "QueryAllocatedMemory" | "QueryAllocatedMemoryUsage" => {
                        summary.query_allocated_memory = ValueParser::parse_bytes(value).ok();
                    }
                    "Query Peak Memory" | "QueryPeakMemory" | "QueryPeakMemoryUsagePerNode" => {
                        summary.query_peak_memory = ValueParser::parse_bytes(value).ok();
                    }
                    "Query Cumulative Operator Time" | "QueryCumulativeOperatorTime" => {
                        summary.push_total_time = ValueParser::parse_time_to_ms(value).ok();
                    }
                    "Query Cumulative Scan Time" | "QueryCumulativeScanTime" => {
                        summary.pull_total_time = ValueParser::parse_time_to_ms(value).ok();
                    }
                    _ => {}
                }
            }
        }
        
        Ok(summary)
    }
    
    fn parse_planner(&self, text: &str) -> ParseResult<PlannerInfo> {
        let planner_section = Self::extract_section(text, "Planner:", "Execution:");
        let mut details = HashMap::new();
        
        for line in planner_section.lines() {
            let trimmed = line.trim();
            if let Some((key, value)) = Self::parse_kv_line(trimmed) {
                details.insert(key.to_string(), value.to_string());
            }
        }
        
        Ok(PlannerInfo { details })
    }
    
    fn parse_execution(&mut self, text: &str, summary: &mut ProfileSummary) -> ParseResult<ExecutionInfo> {
        let execution_section = Self::extract_section(text, "Execution:", "Fragment ");
        let mut topology = String::new();
        let mut metrics = HashMap::new();

        // 提取 Topology
        if let Some(start) = execution_section.find("- Topology:") {
            let after = &execution_section[start + "- Topology:".len()..];
            if let Some(json) = Self::extract_topology_json(after) {
                topology = json.to_string();
            }
        }

        // 提取其他指标，包含summary相关的字段
        for line in execution_section.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("- Topology:") {
                continue;
            }
            if let Some((key, value)) = Self::parse_kv_line(trimmed) {
                match key {
                    "QueryAllocatedMemoryUsage" => {
                        summary.query_allocated_memory = ValueParser::parse_bytes(value).ok();
                    }
                    "QueryPeakMemoryUsagePerNode" => {
                        summary.query_peak_memory = ValueParser::parse_bytes(value).ok();
                    }
                    "QueryCumulativeOperatorTime" => {
                        summary.push_total_time = ValueParser::parse_time_to_ms(value).ok();
                    }
                    "QueryCumulativeScanTime" => {
                        summary.pull_total_time = ValueParser::parse_time_to_ms(value).ok();
                    }
                    _ => {
                        metrics.insert(key.to_string(), value.to_string());
                    }
                }
            }
        }

        Ok(ExecutionInfo { topology, metrics })
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
    
        // 1. 收集所有 Fragments 中的 Operators，按 plan_node_id 分组，并携带 fragment/pipeline 信息
        let mut operators_by_plan_id: HashMap<i32, Vec<(&Operator, String, String)>> = HashMap::new();
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
                // 针对同一个 plan_node_id 的多个 operators，选择最匹配 Topology 名称的
                let best_op = {
                    let op_refs: Vec<&Operator> = op_list.iter().map(|(op,_,_)| *op).collect();
                    Self::select_best_operator(&op_refs, &topo_node.name)
                };
                // 找到所选 operator 对应的 fragment/pipeline
                let (frag_id, pipe_id) = {
                    if let Some((_, f, p)) = op_list.iter().find(|(op, _, _)| op.name == best_op.name) {
                        (Some(f.clone()), Some(p.clone()))
                    } else {
                        (None, None)
                    }
                };
                // 创建节点
                let mut metrics = MetricsParser::from_hashmap(&best_op.common_metrics);
    
                // 解析 UniqueMetrics (specialized metrics)
                if !best_op.unique_metrics.is_empty() {
                    let specialized_parser = SpecializedMetricsParser::new();
    
                    // 提取操作符名称（去掉plan_node_id部分）
                    let operator_base_name = if let Some(idx) = best_op.name.find(" (plan_node_id=") {
                        &best_op.name[..idx]
                    } else {
                        &best_op.name
                    };
                    let unique_text = Self::build_unique_metrics_text(&best_op.unique_metrics);
                    metrics.specialized = specialized_parser.parse(operator_base_name, &unique_text);
                }
    
                ExecutionTreeNode {
                    id: format!("node_{}", topo_node.id),
                    plan_node_id: Some(topo_node.id),
                    // 始终使用 Topology 中定义的名称，确保树展示一致
                    operator_name: topo_node.name.clone(),
                    node_type: OperatorParser::determine_node_type(&topo_node.name),
                    parent_plan_node_id: None,
                    children: Vec::new(),
                    depth: 0,
                    metrics,
                    is_hotspot: false,
                    hotspot_severity: HotSeverity::Normal,
                    fragment_id: frag_id,
                    pipeline_id: pipe_id,
                }
            } else {
                // 没找到，创建占位符
                ExecutionTreeNode {
                    id: format!("node_{}", topo_node.id),
                    plan_node_id: Some(topo_node.id),
                    operator_name: format!("{} (no metrics)", topo_node.name),
                    node_type: NodeType::Unknown,
                    parent_plan_node_id: None,
                    children: Vec::new(),
                    depth: 0,
                    metrics: OperatorMetrics::default(),
                    is_hotspot: false,
                    hotspot_severity: HotSeverity::Normal,
                    fragment_id: None,
                    pipeline_id: None,
                }
            };
    
            nodes.push(tree_node);
        }
    
        // TODO: 可选：根据需要处理 SINK 节点的特殊逻辑
        // 当前实现直接返回已构建的拓扑节点列表
        Ok(nodes)
    }
    
    /// 从operator名称中提取纯名称（去掉plan_node_id部分）
    ///
    /// # Arguments
    /// * `full_name` - 完整的operator名称，如 "LOCAL_EXCHANGE_SINK (plan_node_id=-1)"
    ///
    /// # Returns
    /// * `String` - 纯的operator名称，如 "LOCAL_EXCHANGE_SINK"
    #[allow(dead_code)]
    fn extract_operator_name(full_name: &str) -> String {
        if let Some(pos) = full_name.find(" (plan_node_id=") {
            full_name[..pos].to_string()
        } else {
            full_name.to_string()
        }
    }
    
    /// 使用StarRocks的通用逻辑选择SINK节点
    ///
    /// StarRocks的isFinalSink逻辑：
    /// 1. 必须是DataSink类型（以_SINK结尾）
    /// 2. 不能是DataStreamSink类型（EXCHANGE_SINK, LOCAL_EXCHANGE_SINK等）
    /// 3. 不能是MultiCastDataSink类型
    ///
    /// # Arguments
    /// * `fragments` - 解析后的 Fragments 列表
    ///
    /// # Returns
    /// * `Option<String>` - 选中的 SINK 节点名称，如果没有找到则返回 None
    #[allow(dead_code)]
    fn select_sink_node(fragments: &[crate::models::Fragment]) -> Option<String> {
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
    
    /// 从多个 Operator 中选择最合适的（完全符合 SR 逻辑）
    /// 
    /// SR 逻辑：
    /// 1. 名称/别名精准匹配优先（严格按照 Topology 定义）
    /// 2. 同类别优先级（SCAN > 其它普通算子 > SINK/SOURCE）
    /// 3. 仍不匹配时回退第一个
    fn select_best_operator<'a>(operators: &[&'a Operator], topo_name: &str) -> &'a Operator {
        // 统一为 Topology 规范名称进行严格匹配
        let topo_canonical = OperatorParser::canonical_topology_name(topo_name);

        // 1) 精确的规范名匹配（优先）
        if let Some(op) = operators.iter().find(|op| {
            let pure_name = Self::extract_operator_name(&op.name);
            let op_canonical = OperatorParser::canonical_topology_name(&pure_name);
            op_canonical == topo_canonical
        }) {
            return op;
        }

        // 2) 次优：标准化名称匹配（兼容大小写/别名，但不改规范）
        if let Some(op) = operators.iter().find(|op| {
            let pure_name = Self::extract_operator_name(&op.name);
            let normalized = OperatorParser::normalize_name(&pure_name);
            OperatorParser::canonical_topology_name(&normalized) == topo_canonical
        }) {
            return op;
        }

        // 3) 类别优先：优先选择 "SCAN" 的算子，其次非 SINK/SOURCE
        if let Some(op) = operators.iter().find(|op| op.name.contains("SCAN")) {
            return op;
        }
        if let Some(op) = operators.iter().find(|op| {
            !op.name.contains("SINK") && !op.name.contains("SOURCE")
        }) {
            return op;
        }

        // 4) 默认第一个
        operators[0]
    }
    
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
                    let operator_text = Self::find_operator_text(text, &operator.name);
                    if operator_text.is_empty() {
                        continue;
                    }
                    
                    let plan_id_i32 = operator
                        .plan_node_id
                        .as_ref()
                        .and_then(|s| s.parse::<i32>().ok())
                        .unwrap_or(node_counter);

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
    
    fn parse_operator_to_node(
        &self,
        text: &str,
        operator_name: &str,
        plan_node_id: i32,
        fragment_id: Option<String>,
        pipeline_id: Option<String>,
    ) -> ParseResult<ExecutionTreeNode> {
        // 解析 Operator 头部
        let header = text.lines()
            .find(|line| OperatorParser::is_operator_header(line.trim()))
            .and_then(|line| OperatorParser::parse_header(line).ok());
        
        let actual_name = header.as_ref().map(|h| h.name.as_str()).unwrap_or(operator_name);
        let actual_plan_id = header.as_ref().and_then(|h| Some(h.plan_node_id)).unwrap_or(plan_node_id);
        
        // 解析通用指标
        let common_metrics = MetricsParser::parse_common_metrics(text);
        
        // 解析专用指标
        let specialized_metrics = self.specialized_parser.parse(actual_name, text);
        
        // 合并指标
        let mut metrics = OperatorMetrics::default();
        metrics.operator_total_time = common_metrics.operator_total_time;
        metrics.operator_total_time_raw = common_metrics.operator_total_time_raw;
        metrics.push_chunk_num = common_metrics.push_chunk_num;
        metrics.push_row_num = common_metrics.push_row_num;
        metrics.pull_chunk_num = common_metrics.pull_chunk_num;
        metrics.pull_row_num = common_metrics.pull_row_num;
        metrics.push_total_time = common_metrics.push_total_time;
        metrics.pull_total_time = common_metrics.pull_total_time;
        metrics.memory_usage = common_metrics.memory_usage;
        metrics.output_chunk_bytes = common_metrics.output_chunk_bytes;
        metrics.specialized = specialized_metrics;
        
        Ok(ExecutionTreeNode {
            id: format!("node_{}", actual_plan_id),
            operator_name: actual_name.to_string(),
            node_type: OperatorParser::determine_node_type(actual_name),
            plan_node_id: Some(actual_plan_id),
            parent_plan_node_id: None,
            metrics,
            children: Vec::new(),
            depth: 0,
            is_hotspot: false,
            hotspot_severity: crate::models::HotSeverity::Normal,
            fragment_id,
            pipeline_id,
        })
    }
    
    // ========== Helper Methods ==========
    
    fn extract_section<'a>(text: &'a str, start_marker: &str, end_marker: &str) -> &'a str {
        let start = text.find(start_marker).unwrap_or(0);
        let end = text[start..].find(end_marker).map(|i| start + i).unwrap_or(text.len());
        &text[start..end]
    }
    
    fn extract_topology_json(text: &str) -> Option<&str> {
        let trimmed = text.trim();
        if let Some(start) = trimmed.find('{') {
            let mut depth = 0;
            let mut end = start;
            
            for (i, ch) in trimmed[start..].char_indices() {
                match ch {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            end = start + i + 1;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            
            if depth == 0 {
                return Some(&trimmed[start..end]);
            }
        }
        None
    }
    
    fn parse_kv_line(line: &str) -> Option<(&str, &str)> {
        if !line.starts_with('-') {
            return None;
        }
        let rest = line.trim_start_matches('-').trim();
        let parts: Vec<&str> = rest.splitn(2, ':').collect();
        if parts.len() == 2 {
            Some((parts[0].trim(), parts[1].trim()))
        } else {
            None
        }
    }
    
    fn find_operator_text(text: &str, operator_name: &str) -> String {
        OperatorParser::extract_operator_block(text, operator_name, None)
    }
    
    /// 从 HashMap 构建 UniqueMetrics 文本块
    /// 
    /// 将 HashMap<String, String> 转换为类似 profile 格式的文本，
    /// 以便 SpecializedMetricsParser 可以解析
    fn build_unique_metrics_text(unique_metrics: &HashMap<String, String>) -> String {
        let mut lines = Vec::new();
        lines.push("UniqueMetrics:".to_string());
        
        for (key, value) in unique_metrics {
            if value == "true" {
                // 无值的标志（如 IsSubordinate）
                lines.push(format!("   - {}", key));
            } else {
                lines.push(format!("   - {}: {}", key, value));
            }
        }
        
        lines.join("\n")
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
