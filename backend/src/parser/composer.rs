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

///

///


#[derive(Debug, Clone)]
pub struct ProfileComposer {
    specialized_parser: SpecializedMetricsParser,
}

impl ProfileComposer {
    pub fn new() -> Self {
        Self {
            specialized_parser: SpecializedMetricsParser::new(),
        }
    }
    
    ///
    ///
    pub fn parse(&mut self, text: &str) -> ParseResult<Profile> {
        let mut summary = self.parse_summary(text)?;
        let planner_info = self.parse_planner(text)?;
        let execution_info = self.parse_execution(text)?;
        
        if summary.query_cumulative_operator_time_ms.is_none() {
            if let Some(qcot) = execution_info.metrics.get("QueryCumulativeOperatorTime") {
                summary.query_cumulative_operator_time_ms = ValueParser::parse_time_to_ms(qcot).ok();
            }
        }
        
        if summary.query_execution_wall_time_ms.is_none() {
            if let Some(qewt) = execution_info.metrics.get("QueryExecutionWallTime") {
                summary.query_execution_wall_time_ms = ValueParser::parse_time_to_ms(qewt).ok();
                summary.query_execution_wall_time = Some(qewt.clone());
            }
        }
        
        // Extract all execution metrics for overview diagnostics
        Self::extract_execution_metrics(&execution_info, &mut summary);
        
        let fragments = FragmentParser::extract_all_fragments(text);

        let topology_result = Self::extract_topology_json(&execution_info.topology)
            .and_then(|json| {
                TopologyParser::parse_with_fragments(&json, text, &fragments)
            }).ok();
        
        let execution_tree = if let Some(ref topology) = topology_result {
            println!("DEBUG: Using topology-based node building");
            let nodes = self.build_nodes_from_topology_and_fragments(topology, &fragments)?;
            TreeBuilder::build_from_topology(topology, nodes, &fragments, &summary)?
        } else {
            println!("DEBUG: Using fragment-based node building");
            let nodes = self.build_nodes_from_fragments(text, &fragments)?;
            TreeBuilder::build_from_fragments(nodes, &summary, &fragments)?
        };
        
        use crate::constants::top_n;
        let top_nodes = Self::compute_top_time_consuming_nodes(&execution_tree.nodes, top_n::TOP_NODES_LIMIT);
        summary.top_time_consuming_nodes = Some(top_nodes);
        
        
        
        Ok(Profile {
            summary,
            planner: planner_info,
            execution: execution_info,
            fragments,
            execution_tree: Some(execution_tree),
        })
    }

    
    fn parse_summary(&self, text: &str) -> ParseResult<ProfileSummary> {
        SectionParser::parse_summary(text)
    }
    
    fn parse_planner(&self, text: &str) -> ParseResult<crate::models::PlannerInfo> {
        SectionParser::parse_planner(text)
    }
    
    fn parse_execution(
        &self,
        text: &str,
    ) -> ParseResult<crate::models::ExecutionInfo> {
        SectionParser::parse_execution(text)
    }
    
    
    ///
    ///
    fn extract_topology_json(topology_text: &str) -> ParseResult<String> {
        if topology_text.trim().is_empty() {
            return Err(ParseError::TopologyError("Empty topology text".to_string()));
        }
        
        if let Some(start) = topology_text.find("Topology: ") {
            let json_start = start + "Topology: ".len();
            let json_part = &topology_text[json_start..];
            
            let json_end = json_part.find('\n').unwrap_or(json_part.len());
            let json = json_part[..json_end].trim();
            
            if json.is_empty() {
                return Err(ParseError::TopologyError("Empty JSON after Topology:".to_string()));
            }
            
            Ok(json.to_string())
        } else {
            Ok(topology_text.trim().to_string())
        }
    }
    
    
    ///
    fn build_nodes_from_topology_and_fragments(
        &self,
        topology: &TopologyGraph,
        fragments: &[Fragment],
    ) -> ParseResult<Vec<ExecutionTreeNode>> {
        use crate::models::{ExecutionTreeNode, HotSeverity, OperatorMetrics};
        use std::collections::HashMap;
    
        let mut operators_by_plan_id: HashMap<i32, Vec<(&crate::models::Operator, String, String)>> = HashMap::new();
        for fragment in fragments {
            for pipeline in &fragment.pipelines {
                for operator in &pipeline.operators {
                    if let Some(plan_id) = &operator.plan_node_id {
                        if let Ok(plan_id_int) = plan_id.parse::<i32>() {
                            println!("DEBUG: Found operator '{}' with plan_id={}", operator.name, plan_id_int);
                            operators_by_plan_id
                                .entry(plan_id_int)
                                .or_default()
                                .push((operator, fragment.id.clone(), pipeline.id.clone()));
                        }
                    }
                }
            }
        }
        
        println!("DEBUG: operators_by_plan_id keys: {:?}", operators_by_plan_id.keys().collect::<Vec<_>>());
 
        let mut nodes = Vec::new();
        for topo_node in &topology.nodes {
            println!("DEBUG: Processing topology node: id={}, name={}", topo_node.id, topo_node.name);
            let tree_node = if let Some(op_list) = operators_by_plan_id.get(&topo_node.id) {
                println!("DEBUG: Found {} operators for plan_id={}", op_list.len(), topo_node.id);
                let op_refs: Vec<&crate::models::Operator> = op_list.iter().map(|(op,_,_)| *op).collect();
                let aggregated_op = Self::aggregate_operators(&op_refs, &topo_node.name);
                
                let (frag_id, pipe_id) = {
                    if let Some((_, f, p)) = op_list.first() {
                        (Some(f.clone()), Some(p.clone()))
                    } else {
                        (None, None)
                    }
                };
                

                let mut metrics = MetricsParser::from_hashmap(&aggregated_op.common_metrics);

                // Debug: Check if unique_metrics is empty
                println!("DEBUG: aggregated_op.unique_metrics.len() = {}", aggregated_op.unique_metrics.len());
                if !aggregated_op.unique_metrics.is_empty() {
                    println!("DEBUG: unique_metrics keys: {:?}", aggregated_op.unique_metrics.keys().collect::<Vec<_>>());
                }

                // Parse specialized metrics using StarRocks official approach
                if !aggregated_op.unique_metrics.is_empty() {
                    let specialized_parser = SpecializedMetricsParser::new();
                    let pure_name = Self::extract_operator_name(&aggregated_op.name);
                    
                    // Build complete operator text including both common and unique metrics
                    let mut operator_text = String::new();
                    operator_text.push_str(&format!("{} (plan_node_id={}):\n", pure_name, topo_node.id));
                    operator_text.push_str("  CommonMetrics:\n");
                    for (key, value) in &aggregated_op.common_metrics {
                        operator_text.push_str(&format!("     - {}: {}\n", key, value));
                    }
                    operator_text.push_str("  UniqueMetrics:\n");
                    for (key, value) in &aggregated_op.unique_metrics {
                        operator_text.push_str(&format!("     - {}: {}\n", key, value));
                    }
                    
                    metrics.specialized = specialized_parser.parse(&pure_name, &operator_text);
                }

                ExecutionTreeNode {
                    id: format!("node_{}", topo_node.id),
                    plan_node_id: Some(topo_node.id),
                    operator_name: topo_node.name.clone(),
                    node_type: OperatorParser::determine_node_type(&aggregated_op.name),
                    parent_plan_node_id: None,
                    children: Vec::new(),
                    depth: 0,
                    metrics,
                    is_hotspot: false,
                    hotspot_severity: HotSeverity::Normal,
                    fragment_id: frag_id,
                    pipeline_id: pipe_id,
                    time_percentage: None,
                    is_most_consuming: false,
                    is_second_most_consuming: false,
                    unique_metrics: aggregated_op.unique_metrics.clone(),
                }
            } else {
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
                    unique_metrics: HashMap::new(),
                }
            };

            nodes.push(tree_node);
        }

        let mut sink_nodes = Vec::new();
        let mut next_sink_id = -1;
        
        for fragment in fragments {
            for pipeline in &fragment.pipelines {
                for operator in &pipeline.operators {
                    let pure_name = Self::extract_operator_name(&operator.name);
                    
                    if pure_name.ends_with("_SINK") {

                        let plan_id = operator.plan_node_id.as_ref()
                            .and_then(|id| id.parse::<i32>().ok())
                            .unwrap_or(next_sink_id);
                        

                        if !topology.nodes.iter().any(|n| n.id == plan_id) {
                            let mut metrics = MetricsParser::from_hashmap(&operator.common_metrics);
                            
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
                                unique_metrics: operator.unique_metrics.clone(),
                            };
                            
                            sink_nodes.push(sink_node);
                            next_sink_id -= 1;
                        }
                    }
                }
            }
        }
        
        nodes.extend(sink_nodes);
        
        Ok(nodes)
    }

    fn build_unique_metrics_text(unique_metrics: &HashMap<String, String>) -> String {
        unique_metrics
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<_>>()
            .join("\n")
    }

    ///

    fn extract_operator_name(full_name: &str) -> String {
        if let Some(pos) = full_name.find(" (plan_node_id=") {
            full_name[..pos].trim().to_string()
        } else {
            full_name.trim().to_string()
        }
    }


    ///

    fn aggregate_operators(operators: &[&crate::models::Operator], topology_name: &str) -> crate::models::Operator {
        if operators.is_empty() {
            panic!("Empty operators list");
        }

        println!("DEBUG: aggregate_operators called for topology_name: {}", topology_name);
        println!("DEBUG: Available operators: {:?}", operators.iter().map(|op| &op.name).collect::<Vec<_>>());
        

        let mut matching_operators = Vec::new();
        
        for &op in operators {
            let op_name = Self::extract_operator_name(&op.name);
            let op_canonical = crate::parser::core::OperatorParser::canonical_topology_name(&op_name);
            println!("DEBUG: Checking operator '{}' -> name: '{}' -> canonical: '{}' against topology: '{}'", op.name, op_name, op_canonical, topology_name);
            if op_canonical == topology_name {
                matching_operators.push(op);
                println!("DEBUG: Found matching operator: {}", op.name);
            }
        }

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

        if matching_operators.is_empty() && topology_name == "OLAP_SCAN" {
            for &op in operators {
                let op_name = Self::extract_operator_name(&op.name);
                let op_canonical = crate::parser::core::OperatorParser::canonical_topology_name(&op_name);
                if op_canonical == "CONNECTOR_SCAN" {
                    matching_operators.push(op);
                }
            }
        }

        if matching_operators.is_empty() {
            matching_operators.push(operators[0]);
        }

        let mut base_operator = matching_operators[0].clone();
        
        println!("DEBUG: Found {} matching operators for {}", matching_operators.len(), topology_name);
        
        let mut total_time_ns: u64 = 0;
        for &op in &matching_operators {
            if let Some(time_str) = op.common_metrics.get("OperatorTotalTime") {
                println!("DEBUG: Processing operator '{}' with OperatorTotalTime: '{}'", op.name, time_str);

                if let Some(time_ms) = Self::parse_time_to_ms(time_str) {
                    let time_ns = (time_ms * 1_000_000.0) as u64;
                    total_time_ns += time_ns;
                    println!("DEBUG: Parsed time: {}ms -> {}ns, running total: {}ns", time_ms, time_ns, total_time_ns);
                } else {
                    println!("DEBUG: Failed to parse time string: '{}'", time_str);
                }
            } else {
                println!("DEBUG: Operator '{}' has no OperatorTotalTime", op.name);
            }
        }
        

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

        // Aggregate unique_metrics from all matching operators
        let mut aggregated_unique_metrics = HashMap::new();
        println!("DEBUG: Aggregating unique_metrics from {} matching operators", matching_operators.len());
        for &op in &matching_operators {
            println!("DEBUG: Processing operator '{}' with {} unique_metrics", op.name, op.unique_metrics.len());
            for (key, value) in &op.unique_metrics {
                println!("DEBUG: Adding unique_metric: {} = {}", key, value);
                
                // For min/max metrics, we need to aggregate properly
                if key.starts_with("__MIN_OF_") || key.starts_with("__MAX_OF_") {
                    // For min/max metrics, keep the first occurrence (they should be the same across instances)
                    if !aggregated_unique_metrics.contains_key(key) {
                        aggregated_unique_metrics.insert(key.clone(), value.clone());
                    }
                } else {
                    // For regular metrics, we need to aggregate all values, not just the first one
                    // This is important for min/max values and other aggregated metrics
                    aggregated_unique_metrics.insert(key.clone(), value.clone());
                }
            }
        }
        
        // Merge unique_metrics into base_operator
        base_operator.unique_metrics = aggregated_unique_metrics;

        base_operator
    }


    /// 

    fn parse_time_to_ms(time_str: &str) -> Option<f64> {
        let time_str = time_str.trim();
        
        if time_str.ends_with("ms") {
            let num_str = time_str.trim_end_matches("ms");
            return num_str.parse::<f64>().ok();
        }
        
        if time_str.ends_with("us") {
            let num_str = time_str.trim_end_matches("us");
            return num_str.parse::<f64>().map(|us| us / 1000.0).ok();
        }
        
        if time_str.ends_with("ns") {
            let num_str = time_str.trim_end_matches("ns");
            return num_str.parse::<f64>().map(|ns| ns / 1_000_000.0).ok();
        }
        
        if time_str.ends_with("s") && !time_str.ends_with("ms") && !time_str.ends_with("us") && !time_str.ends_with("ns") {
            let num_str = time_str.trim_end_matches("s");
            return num_str.parse::<f64>().map(|s| s * 1000.0).ok();
        }
        
        if time_str.contains("m") {

            return None;
        }
        

        time_str.parse::<f64>().ok()
    }

    ///
    fn build_nodes_from_fragments(
        &self,
        text: &str,
        fragments: &[Fragment],
    ) -> ParseResult<Vec<ExecutionTreeNode>> {
        let mut nodes = Vec::new();
        let mut node_counter = 0;
        
        for fragment in fragments {
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

    #[allow(dead_code)]
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

    fn find_operator_text_by_plan_id(text: &str, operator_name: &str, plan_node_id: i32) -> String {
        OperatorParser::extract_operator_block(text, operator_name, Some(plan_node_id))
    }

    fn parse_operator_to_node(
        &self,
        operator_text: &str,
        operator_name: &str,
        plan_node_id: i32,
        fragment_id: Option<String>,
        pipeline_id: Option<String>,
    ) -> ParseResult<ExecutionTreeNode> {

        let mut metrics = MetricsParser::parse_common_metrics(operator_text);


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
            unique_metrics: HashMap::new(), // 这个方法中没有unique_metrics数据
        })
    }
    
    ///
    ///
    fn compute_top_time_consuming_nodes(
        nodes: &[crate::models::ExecutionTreeNode],
        limit: usize
    ) -> Vec<crate::models::TopNode> {
        use crate::models::TopNode;
        
        let mut sorted_nodes: Vec<_> = nodes.iter()
            .filter(|n| {
                n.time_percentage.is_some() && 
                n.time_percentage.unwrap() > 0.0 &&
                n.plan_node_id.is_some()
            })
            .collect();
        
        sorted_nodes.sort_by(|a, b| {
            let a_pct = a.time_percentage.unwrap_or(0.0);
            let b_pct = b.time_percentage.unwrap_or(0.0);
            b_pct.partial_cmp(&a_pct).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        sorted_nodes.iter()
            .take(limit)
            .enumerate()
            .map(|(i, node)| {
                let percentage = node.time_percentage.unwrap_or(0.0);
                {
                    use crate::constants::time_thresholds;
                    TopNode {
                        rank: (i + 1) as u32,
                        operator_name: node.operator_name.clone(),
                        plan_node_id: node.plan_node_id.unwrap_or(-1),
                        total_time: node.metrics.operator_total_time_raw
                            .clone()
                            .unwrap_or_else(|| "N/A".to_string()),
                        time_percentage: percentage,
                        is_most_consuming: percentage > time_thresholds::MOST_CONSUMING_THRESHOLD,
                        is_second_most_consuming: percentage > time_thresholds::SECOND_CONSUMING_THRESHOLD 
                            && percentage <= time_thresholds::MOST_CONSUMING_THRESHOLD,
                    }
                }
            })
            .collect()
    }
    
    /// Extract all execution metrics for overview diagnostics
    fn extract_execution_metrics(execution_info: &crate::models::ExecutionInfo, summary: &mut ProfileSummary) {
        // Memory metrics
        if let Some(val) = execution_info.metrics.get("QueryAllocatedMemoryUsage") {
            summary.query_allocated_memory = ValueParser::parse_bytes_to_u64(val).ok();
        }
        if let Some(val) = execution_info.metrics.get("QueryPeakMemoryUsagePerNode") {
            summary.query_peak_memory = ValueParser::parse_bytes_to_u64(val).ok();
        }
        if let Some(val) = execution_info.metrics.get("QuerySumMemoryUsage") {
            summary.query_sum_memory_usage = Some(val.clone());
        }
        if let Some(val) = execution_info.metrics.get("QueryDeallocatedMemoryUsage") {
            summary.query_deallocated_memory_usage = Some(val.clone());
        }
        
        // Time metrics
        if let Some(val) = execution_info.metrics.get("QueryCumulativeCpuTime") {
            summary.query_cumulative_cpu_time = Some(val.clone());
            summary.query_cumulative_cpu_time_ms = ValueParser::parse_time_to_ms(val).ok();
        }
        if let Some(val) = execution_info.metrics.get("QueryCumulativeScanTime") {
            summary.query_cumulative_scan_time = Some(val.clone());
            summary.query_cumulative_scan_time_ms = ValueParser::parse_time_to_ms(val).ok();
        }
        if let Some(val) = execution_info.metrics.get("QueryCumulativeNetworkTime") {
            summary.query_cumulative_network_time = Some(val.clone());
            summary.query_cumulative_network_time_ms = ValueParser::parse_time_to_ms(val).ok();
        }
        if let Some(val) = execution_info.metrics.get("QueryPeakScheduleTime") {
            summary.query_peak_schedule_time = Some(val.clone());
            summary.query_peak_schedule_time_ms = ValueParser::parse_time_to_ms(val).ok();
        }
        if let Some(val) = execution_info.metrics.get("ResultDeliverTime") {
            summary.result_deliver_time = Some(val.clone());
            summary.result_deliver_time_ms = ValueParser::parse_time_to_ms(val).ok();
        }
        
        // Spill metrics
        if let Some(val) = execution_info.metrics.get("QuerySpillBytes") {
            summary.query_spill_bytes = Some(val.clone());
        }
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
