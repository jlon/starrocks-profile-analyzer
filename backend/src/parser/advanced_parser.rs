use crate::models::*;
use regex::Regex;
use once_cell::sync::Lazy;
use std::time::Duration;
use crate::StarRocksProfileParser;

static OPERATOR_LINE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^([A-Z_]+)\s*\(plan_node_id=(-?\d+)(?:\s*\(operator\s+id=(\d+)\))?\)").unwrap()
});

static METRIC_LINE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s*-\s+([A-Za-z][A-Za-z0-9_]*(?:\s+[A-Za-z][A-Za-z0-9_]*)*):\s+(.+)$").unwrap()
});

pub struct AdvancedStarRocksProfileParser;

impl AdvancedStarRocksProfileParser {
    pub fn parse_advanced(input: &str) -> Result<Profile, String> {
        let mut profile = StarRocksProfileParser::parse(input)?;
        
        // 优先尝试从Topology JSON构建树
        if let Ok(execution_tree) = Self::build_tree_from_topology(&profile, input) {
            eprintln!("✅ Successfully built tree from Topology JSON");
            profile.execution_tree = Some(execution_tree);
        } else {
            // 如果Topology JSON解析失败，回退到详细operator解析
            eprintln!("⚠️ Topology JSON parsing failed, falling back to detailed operator parsing");
            let execution_tree = Self::build_execution_tree_from_profile(&profile, input)?;
            profile.execution_tree = Some(execution_tree);
        }
        
        Ok(profile)
    }
    
    fn build_tree_from_topology(profile: &Profile, profile_text: &str) -> Result<ExecutionTree, String> {
        let topology_str = &profile.execution.topology;
        eprintln!("🔍 Attempting to parse Topology JSON from: {}", &topology_str[..std::cmp::min(100, topology_str.len())]);
        
        // 提取JSON部分
        if let Some(start) = topology_str.find('{') {
            let json_str = &topology_str[start..];
            
            // 解析JSON
            if let Ok(topology) = serde_json::from_str::<serde_json::Value>(json_str) {
                eprintln!("✅ Topology JSON parsed successfully");
                
                // 从topology构建树
                let nodes = topology.get("nodes").and_then(|n| n.as_array()).ok_or("No nodes in topology")?;
                let root_id = topology.get("rootId").and_then(|r| r.as_i64()).ok_or("No rootId in topology")? as i32;
                
                eprintln!("📊 Topology has {} nodes, root_id={}", nodes.len(), root_id);
                
                let mut tree_nodes = Vec::new();
                let mut id_map = std::collections::HashMap::new();
                
                // 第0步：添加RESULT_SINK作为顶级节点
                let result_sink_metrics = Self::find_operator_metrics(profile_text, "RESULT_SINK");
                let result_sink = ExecutionTreeNode {
                    id: "node_result_sink".to_string(),
                    operator_name: "RESULT_SINK".to_string(),
                    node_type: Self::determine_node_type("RESULT_SINK"),
                    plan_node_id: Some(-1),
                    parent_plan_node_id: None,
                    metrics: result_sink_metrics.unwrap_or(OperatorMetrics {
                        operator_total_time: None,
                        push_chunk_num: None,
                        push_row_num: None,
                        pull_chunk_num: None,
                        pull_row_num: None,
                        push_total_time: None,
                        pull_total_time: None,
                        output_chunk_bytes: None,
                        memory_usage: None,
                        specialized: OperatorSpecializedMetrics::None,
                    }),
                    children: Vec::new(),
                    depth: 0,
                    is_hotspot: false,
                    hotspot_severity: HotSeverity::Normal,
                };
                tree_nodes.push(result_sink.clone());
                eprintln!("  📌 RESULT_SINK added as root (idx=0)");
                
                // 第一遍：创建所有Topology节点
                for node in nodes.iter() {
                    let node_id = node.get("id").and_then(|i| i.as_i64()).ok_or("Node missing id")? as i32;
                    let name = node.get("name").and_then(|n| n.as_str()).ok_or("Node missing name")?;
                    
                    let node_type = Self::determine_node_type(name);
                    let operator_metrics = Self::find_operator_metrics(profile_text, name);
                    let tree_node = ExecutionTreeNode {
                        id: format!("topo_{}", node_id),
                        operator_name: name.to_string(),
                        node_type,
                        plan_node_id: Some(node_id),
                        parent_plan_node_id: None,
                        metrics: operator_metrics.unwrap_or(OperatorMetrics {
                            operator_total_time: None,
                            push_chunk_num: None,
                            push_row_num: None,
                            pull_chunk_num: None,
                            pull_row_num: None,
                            push_total_time: None,
                            pull_total_time: None,
                            output_chunk_bytes: None,
                            memory_usage: None,
                            specialized: OperatorSpecializedMetrics::None,
                        }),
                        children: Vec::new(),
                        depth: 0,
                        is_hotspot: false,
                        hotspot_severity: HotSeverity::Normal,
                    };
                    
                    id_map.insert(node_id, tree_nodes.len());
                    tree_nodes.push(tree_node);
                    
                    eprintln!("  📌 Node {}: {} (type: {:?})", node_id, name, node_type);
                }
                
                // 第二遍：建立连接关系，并将Topology的root连接到RESULT_SINK
                for node in nodes.iter() {
                    let node_id = node.get("id").and_then(|i| i.as_i64()).ok_or("Node missing id")? as i32;
                    let node_idx = id_map.get(&node_id).copied().ok_or("Node index not found")?;
                    
                    if let Some(children) = node.get("children").and_then(|c| c.as_array()) {
                        for child_id_val in children {
                            let child_id = child_id_val.as_i64().ok_or("Invalid child id")? as i32;
                            if let Some(&child_idx) = id_map.get(&child_id) {
                                // 保持原方向：parent → child
                                let child_node_id = tree_nodes[child_idx].id.clone();
                                tree_nodes[node_idx].children.push(child_node_id);
                                tree_nodes[child_idx].parent_plan_node_id = Some(node_id);
                                eprintln!("  🔗 {} → {}", node_id, child_id);
                            }
                        }
                    }
                }
                
                // 连接RESULT_SINK到Topology root
                if let Some(&root_idx) = id_map.get(&root_id) {
                    let root_node_id = tree_nodes[root_idx].id.clone();
                    tree_nodes[0].children.push(root_node_id);
                    tree_nodes[root_idx].parent_plan_node_id = Some(-1);
                    eprintln!("  🔗 RESULT_SINK → {}", root_id);
                }
                
                // 计算深度和填充指标
                Self::calculate_depths(&mut tree_nodes);
                Self::detect_hotspots(&mut tree_nodes);
                
                let root = tree_nodes[0].clone();
                return Ok(ExecutionTree { root, nodes: tree_nodes });
            }
        }
        
        Err("Failed to parse Topology JSON".to_string())
    }
    
    fn fill_metrics_from_operators(nodes: &mut Vec<ExecutionTreeNode>, profile_text: &str) {
        // 从profile文本中提取operator metrics并关联到topology节点
        // 根据operator_name和plan_node_id匹配
        
        // 这是一个简化实现，实际可以更复杂
        for node in nodes.iter_mut() {
            // 根据operator名称搜索对应的metrics
            if let Some(metrics) = Self::find_operator_metrics(profile_text, &node.operator_name) {
                node.metrics = metrics;
            }
        }
    }
    
    fn find_operator_metrics(profile_text: &str, operator_name: &str) -> Option<OperatorMetrics> {
        // 简化实现：搜索operator块并提取metrics
        let pattern = format!(r"{}.*?OperatorTotalTime:\s*([^\n]+)", operator_name);
        if let Ok(regex) = Regex::new(&pattern) {
            if let Some(caps) = regex.captures(profile_text) {
                let total_time_str = caps.get(1)?.as_str();
                if let Ok(duration) = Self::parse_duration(total_time_str) {
                    return Some(OperatorMetrics {
                        operator_total_time: Some(duration),
                        push_chunk_num: None,
                        push_row_num: None,
                        pull_chunk_num: None,
                        pull_row_num: None,
                        push_total_time: None,
                        pull_total_time: None,
                        output_chunk_bytes: None,
                        memory_usage: None,
                        specialized: OperatorSpecializedMetrics::None,
                    });
                }
            }
        }
        None
    }
    
    fn detect_hotspots(nodes: &mut Vec<ExecutionTreeNode>) {
        // 计算最大执行时间
        let max_time = nodes.iter()
            .filter_map(|n| Self::get_duration_ms(&n.metrics.operator_total_time))
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(1.0);
        
        // 标记热点
        for node in nodes.iter_mut() {
            if let Some(time) = Self::get_duration_ms(&node.metrics.operator_total_time) {
                let percentage = (time / max_time) * 100.0;
                node.is_hotspot = percentage > 20.0;  // 超过20%则为热点
                node.hotspot_severity = if percentage > 80.0 {
                    HotSeverity::Critical
                } else if percentage > 50.0 {
                    HotSeverity::Severe
                } else if percentage > 20.0 {
                    HotSeverity::Moderate
                } else {
                    HotSeverity::Normal
                };
            }
        }
    }
    
    fn get_duration_ms(duration: &Option<Duration>) -> Option<f64> {
        duration.as_ref().map(|d| d.as_secs_f64() * 1000.0)
    }

    pub fn build_execution_tree_from_profile(profile: &Profile, profile_text: &str) -> Result<ExecutionTree, String> {
        let mut all_nodes = Vec::new();
        let mut _global_operator_index = 0usize;

        // Process fragments in order, then pipelines within each fragment, then operators within each pipeline
        for fragment in &profile.fragments {
            println!("🔍 Processing Fragment {}, pipelines: {}", fragment.id, fragment.pipelines.len());
            for pipeline in &fragment.pipelines {
                let pipeline_node_count = pipeline.operators.len();
                println!("  📊 Pipeline {}, operators: {}", pipeline.id, pipeline_node_count);
                let pipeline_start_idx = all_nodes.len();
                
                // Extract operators from this pipeline
                for (operator_position, operator) in pipeline.operators.iter().enumerate() {
                    if let Some((operator_name, plan_node_id, _operator_id)) = Self::parse_operator_name(&operator.name) {
                        let block_text = Self::extract_operator_metrics(profile_text, &operator_name, plan_node_id);
                        let metrics = Self::parse_metrics_from_text(&block_text);
                        let node_type = Self::determine_node_type(&operator_name);

                        let node_id = format!(
                            "f{}_p{}_op{}_{}",
                            fragment.id, pipeline.id, operator_position, operator_name
                        );
                        println!("    ✅ Created node: {}", node_id);

                        let node = ExecutionTreeNode {
                            id: node_id,
                            operator_name: operator_name.clone(),
                            node_type,
                            plan_node_id,
                            parent_plan_node_id: None,
                            metrics,
                            children: Vec::new(),
                            depth: 0,
                            is_hotspot: false,
                            hotspot_severity: HotSeverity::Normal,
                        };
                        all_nodes.push(node);
                        _global_operator_index += 1;
                    } else {
                        println!("    ❌ Failed to parse operator: {}", operator.name);
                    }
                }
                
                // Link operators within the pipeline (linear chain)
                for i in 0..pipeline_node_count.saturating_sub(1) {
                    let parent_idx = pipeline_start_idx + i;
                    let child_idx = pipeline_start_idx + i + 1;
                    if parent_idx < all_nodes.len() && child_idx < all_nodes.len() {
                        let child_id = all_nodes[child_idx].id.clone();
                        all_nodes[parent_idx].children.push(child_id);
                        println!("    🔗 Linked {} → {}", all_nodes[parent_idx].id, all_nodes[child_idx].id);
                    }
                }
            }
        }

        println!("✅ Total nodes collected: {}", all_nodes.len());
        
        if all_nodes.is_empty() {
            println!("⚠️ No nodes extracted, using fallback");
            return Self::fallback_parsing(profile_text);
        }

        // Link EXCHANGE connections between fragments
        Self::link_exchanges(&mut all_nodes);
        Self::calculate_depths(&mut all_nodes);
        
        let root = all_nodes[0].clone();
        println!("✅ Tree construction complete. Root: {}", root.id);
        Ok(ExecutionTree { root, nodes: all_nodes })
    }

    fn parse_operator_name(name: &str) -> Option<(String, Option<i32>, Option<String>)> {
        OPERATOR_LINE_REGEX.captures(name)
            .and_then(|caps| Some((
                caps.get(1)?.as_str().to_string(),
                caps.get(2).and_then(|m| m.as_str().parse::<i32>().ok()),
                caps.get(3).map(|m| m.as_str().to_string()),
            )))
    }

    fn extract_operator_metrics(profile_text: &str, operator_name: &str, _plan_node_id: Option<i32>) -> String {
        let needle = format!("{}(plan_node_id", operator_name);
        profile_text.find(&needle)
            .and_then(|start| {
                let end_idx = profile_text[start..].find("  ").unwrap_or(profile_text.len() - start) + start;
                Some(profile_text[start..end_idx].to_string())
            })
            .unwrap_or_default()
    }

    fn parse_metrics_from_text(text: &str) -> OperatorMetrics {
        let mut metrics = OperatorMetrics {
            operator_total_time: None, push_chunk_num: None, push_row_num: None,
            pull_chunk_num: None, pull_row_num: None, push_total_time: None,
            pull_total_time: None, memory_usage: None, output_chunk_bytes: None,
            specialized: OperatorSpecializedMetrics::None,
        };

        text.lines()
            .filter(|line| !line.trim().is_empty())
            .for_each(|line| {
                if let Some((key, value)) = Self::parse_metric_line(line) {
                    Self::set_metric_value(&mut metrics, &key, &value);
                }
            });

        metrics
    }

    fn fallback_parsing(profile_text: &str) -> Result<ExecutionTree, String> {
        let lines: Vec<&str> = profile_text.lines().collect();
        let mut operator_blocks = Vec::new();
        let mut current_block = (Vec::new(), 0);

        for (idx, line) in lines.iter().enumerate() {
            if Self::is_operator_start_line(line.trim()) {
                if !current_block.0.is_empty() { operator_blocks.push(current_block.clone()); }
                current_block = (vec![(idx, *line)], Self::get_indent(line));
            } else if !current_block.0.is_empty() {
                let indent = Self::get_indent(line);
                if !line.trim().is_empty() && indent <= current_block.1 && Self::is_new_section(line) {
                    operator_blocks.push(current_block.clone());
                    current_block = (Vec::new(), 0);
                } else {
                    current_block.0.push((idx, *line));
                }
            }
        }
        if !current_block.0.is_empty() { operator_blocks.push(current_block); }

        let mut nodes = operator_blocks.into_iter()
            .enumerate()
            .filter_map(|(op_index, (block, _))| {
                let block_text = block.iter().map(|(_, line)| *line).collect::<Vec<_>>().join("\n");
                Self::parse_operator_block_with_index(&block_text, op_index).ok()
            })
            .collect::<Vec<_>>();

        // Build linear tree relationships: each operator points to the next one
        for i in 0..nodes.len().saturating_sub(1) {
            let next_id = nodes[i + 1].id.clone();
            nodes[i].children.push(next_id);
        }
        
        if nodes.is_empty() { return Err("No operators found".to_string()); }
        
        // Calculate depths before returning
        Self::calculate_depths(&mut nodes);
        
        let root = nodes[0].clone();
        Ok(ExecutionTree { root, nodes })
    }

    fn is_operator_start_line(line: &str) -> bool {
        OPERATOR_LINE_REGEX.is_match(line) && line.trim().ends_with(':')
    }

    fn is_new_section(line: &str) -> bool {
        line.trim().starts_with("Fragment ") || 
        line.trim().starts_with("Pipeline ") ||
        (line.contains(':') && !line.trim().starts_with('-'))
    }

    fn get_indent(line: &str) -> usize {
        line.len() - line.trim_start().len()
    }

    fn parse_operator_block(block: &str) -> Result<ExecutionTreeNode, String> {
        let lines: Vec<&str> = block.lines().collect();
        let (operator_name, plan_node_id, _) = Self::parse_operator_header(lines[0].trim())?;
        
        let metrics = Self::parse_metrics_recursive(&lines[1..], Self::get_indent(lines[0]));
        let node_type = Self::determine_node_type(&operator_name);

        Ok(ExecutionTreeNode {
            id: format!("node_{}", plan_node_id.unwrap_or(-1)),
            operator_name: operator_name.clone(),
            node_type,
            plan_node_id,
            parent_plan_node_id: None,
            metrics,
            children: Vec::new(),
            depth: 0,
            is_hotspot: false,
            hotspot_severity: HotSeverity::Normal,
        })
    }

    fn parse_operator_block_with_index(block: &str, op_index: usize) -> Result<ExecutionTreeNode, String> {
        let lines: Vec<&str> = block.lines().collect();
        let (operator_name, plan_node_id, _) = Self::parse_operator_header(lines[0].trim())?;
        
        let metrics = Self::parse_metrics_recursive(&lines[1..], Self::get_indent(lines[0]));
        let node_type = Self::determine_node_type(&operator_name);

        // Generate unique ID based on operator index
        let node_id = format!("op_{}_plan_{}", op_index, plan_node_id.unwrap_or(-1));

        Ok(ExecutionTreeNode {
            id: node_id,
            operator_name: operator_name.clone(),
            node_type,
            plan_node_id,
            parent_plan_node_id: None,
            metrics,
            children: Vec::new(),
            depth: 0,
            is_hotspot: false,
            hotspot_severity: HotSeverity::Normal,
        })
    }

    fn parse_operator_header(line: &str) -> Result<(String, Option<i32>, Option<String>), String> {
        OPERATOR_LINE_REGEX.captures(line)
            .ok_or_else(|| format!("Invalid operator header: {}", line))
            .and_then(|caps| Ok((
                caps.get(1).ok_or("Missing name")?.as_str().to_string(),
                caps.get(2).and_then(|m| m.as_str().parse::<i32>().ok()),
                caps.get(3).map(|m| m.as_str().to_string()),
            )))
    }

    fn parse_metrics_recursive(lines: &[&str], _base_indent: usize) -> OperatorMetrics {
        let mut metrics = OperatorMetrics {
            operator_total_time: None, push_chunk_num: None, push_row_num: None,
            pull_chunk_num: None, pull_row_num: None, push_total_time: None,
            pull_total_time: None, memory_usage: None, output_chunk_bytes: None,
            specialized: OperatorSpecializedMetrics::None,
        };

        lines.iter()
            .filter(|line| !line.trim().is_empty())
            .for_each(|line| {
            if let Some((key, value)) = Self::parse_metric_line(line) {
                Self::set_metric_value(&mut metrics, &key, &value);
            }
            });

        metrics
    }

    fn parse_metric_line(line: &str) -> Option<(String, String)> {
        METRIC_LINE_REGEX.captures(line)
            .and_then(|caps| Some((
                caps.get(1)?.as_str().replace(" ", ""),
                caps.get(2)?.as_str().to_string(),
            )))
    }

    fn set_metric_value(metrics: &mut OperatorMetrics, key: &str, value: &str) {
        match key {
            "OperatorTotalTime" => { metrics.operator_total_time = Self::parse_duration(value).ok(); },
            "PushChunkNum" => { metrics.push_chunk_num = Self::parse_number(value).ok(); },
            "PushRowNum" => { metrics.push_row_num = Self::parse_number(value).ok(); },
            "PullChunkNum" => { metrics.pull_chunk_num = Self::parse_number(value).ok(); },
            "PullRowNum" => { metrics.pull_row_num = Self::parse_number(value).ok(); },
            "PushTotalTime" => { metrics.push_total_time = Self::parse_duration(value).ok(); },
            "PullTotalTime" => { metrics.pull_total_time = Self::parse_duration(value).ok(); },
            "OutputChunkBytes" => { metrics.output_chunk_bytes = Self::parse_bytes(value).ok(); },
            _ => {},
        };
    }

    fn parse_duration(value: &str) -> Result<Duration, String> {
        let parts: Vec<&str> = value.split_whitespace().collect();
        let duration_str = parts[0];
        
        let mut total_ms = 0.0;
        let mut current = String::new();
        
        for c in duration_str.chars() {
            if c.is_alphabetic() {
                let num: f64 = current.parse().map_err(|_| "Invalid number")?;
                total_ms += match c {
                    'h' => num * 3600000.0,
                    'm' => num * 60000.0,
                    's' => num * 1000.0,
                    'u' => num / 1000.0,
                    'n' => num / 1000000.0,
                    _ => 0.0,
                };
                current.clear();
        } else {
                current.push(c);
            }
        }
        
        Ok(Duration::from_millis(total_ms as u64))
    }

    fn parse_number(value: &str) -> Result<u64, String> {
        value.split_whitespace().next()
            .ok_or("Empty value")?
            .replace(",", "")
            .parse()
            .map_err(|_| "Invalid number".to_string())
    }

    fn parse_bytes(value: &str) -> Result<u64, String> {
        let clean = value.split_whitespace().next().unwrap_or(value).to_uppercase();
        let (num_str, unit) = if clean.contains("GB") {
            (clean.replace("GB", ""), 1024.0 * 1024.0 * 1024.0)
        } else if clean.contains("MB") {
            (clean.replace("MB", ""), 1024.0 * 1024.0)
        } else if clean.contains("KB") {
            (clean.replace("KB", ""), 1024.0)
        } else if clean.contains("B") {
            (clean.replace("B", ""), 1.0)
        } else {
            (clean.clone(), 1.0)
        };
        
        let num: f64 = num_str.parse().map_err(|_| "Invalid number")?;
        Ok((num * unit) as u64)
    }

    fn determine_node_type(operator_name: &str) -> NodeType {
        match operator_name {
            "OLAP_SCAN" => NodeType::OlapScan,
            "CONNECTOR_SCAN" => NodeType::ConnectorScan,
            "HASH_JOIN" | "NL_JOIN" | "CROSS_JOIN" => NodeType::HashJoin,
            "AGGREGATE" => NodeType::Aggregate,
            "LIMIT" => NodeType::Limit,
            "EXCHANGE_SINK" => NodeType::ExchangeSink,
            "EXCHANGE_SOURCE" => NodeType::ExchangeSource,
            "RESULT_SINK" => NodeType::ResultSink,
            "CHUNK_ACCUMULATE" => NodeType::ChunkAccumulate,
            "SORT" => NodeType::Sort,
            _ => NodeType::Unknown,
        }
    }

    fn build_tree_relationships(nodes: &mut [ExecutionTreeNode]) {
        for i in 0..nodes.len() {
            nodes[i].depth = i;
            if i > 0 { nodes[i].parent_plan_node_id = nodes.get(i - 1).and_then(|n| n.plan_node_id); }
            if i + 1 < nodes.len() { nodes[i].children.push(nodes[i + 1].id.clone()); }
        }
    }

    fn link_exchanges(nodes: &mut Vec<ExecutionTreeNode>) {
        // Find EXCHANGE_SINK and EXCHANGE_SOURCE pairs by plan_node_id
        let exchanges: Vec<(usize, Option<i32>, String)> = nodes.iter().enumerate()
            .filter_map(|(i, n)| {
                if n.operator_name == "EXCHANGE_SINK" || n.operator_name == "EXCHANGE_SOURCE" {
                    Some((i, n.plan_node_id, n.id.clone()))
                } else {
                    None
                }
            })
            .collect();

        // For each EXCHANGE_SINK, find the corresponding EXCHANGE_SOURCE and connect them
        let mut i = 0;
        while i < exchanges.len() {
            let (sink_idx, sink_plan_id, _sink_id) = exchanges[i].clone();
            
            if nodes[sink_idx].operator_name == "EXCHANGE_SINK" && sink_plan_id.is_some() {
                // Find matching EXCHANGE_SOURCE
                for j in (i + 1)..exchanges.len() {
                    let (source_idx, source_plan_id, source_id) = &exchanges[j];
                    
                    if nodes[*source_idx].operator_name == "EXCHANGE_SOURCE" && source_plan_id == &sink_plan_id {
                        // Connect EXCHANGE_SINK to EXCHANGE_SOURCE
                        nodes[sink_idx].children.push(source_id.clone());
                        break;
                    }
                }
            }
            i += 1;
        }
    }

    fn calculate_depths(nodes: &mut Vec<ExecutionTreeNode>) {
        // Implement BFS-style depth calculation from root
        // Root node has depth=0, and each child's depth = parent's depth + 1
        
        if nodes.is_empty() {
            return;
        }

        // Create a map from node ID to node index for quick lookup
        let id_to_idx: std::collections::HashMap<String, usize> = nodes.iter()
            .enumerate()
            .map(|(idx, node)| (node.id.clone(), idx))
            .collect();

        // Find root: the node that is not a child of any other node
        let mut has_parent = std::collections::HashSet::new();
        for node in nodes.iter() {
            for child_id in &node.children {
                has_parent.insert(child_id.clone());
            }
        }

        let mut root_idx = 0;
        for (idx, node) in nodes.iter().enumerate() {
            if !has_parent.contains(&node.id) {
                root_idx = idx;
                eprintln!("🌳 Root node found: {} (idx={})", node.id, idx);
                break;
            }
        }

        // BFS from root to calculate depths
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        
        queue.push_back((root_idx, 0)); // (node_index, depth)
        visited.insert(root_idx);
        nodes[root_idx].depth = 0;

        eprintln!("🔍 Starting BFS from root, initial queue size: {}", queue.len());

        while let Some((node_idx, depth)) = queue.pop_front() {
            if node_idx >= nodes.len() {
                continue;
            }

            eprintln!("  📍 Processing node idx={}, depth={}, name={}", node_idx, depth, nodes[node_idx].operator_name);

            // Get children of current node
            let children_ids: Vec<String> = nodes[node_idx].children.clone();
            eprintln!("    Children count: {}", children_ids.len());
            
            for child_id in children_ids {
                if let Some(&child_idx) = id_to_idx.get(&child_id) {
                    if !visited.contains(&child_idx) {
                        nodes[child_idx].depth = depth + 1;
                        visited.insert(child_idx);
                        queue.push_back((child_idx, depth + 1));
                        eprintln!("      ✓ Child: {} (idx={}) depth set to {}", child_id, child_idx, depth + 1);
                    } else {
                        eprintln!("      ✗ Child already visited: {}", child_id);
                    }
                } else {
                    eprintln!("      ✗ Child not found in id_to_idx: {}", child_id);
                }
            }
        }

        eprintln!("✅ Depth calculation complete. Visited: {}/{} nodes", visited.len(), nodes.len());

        // For any unvisited nodes (disconnected), assign depth 0
        for (idx, node) in nodes.iter_mut().enumerate() {
            if !visited.contains(&idx) {
                node.depth = 0;
                eprintln!("⚠️  Unvisited node set to depth 0: {}", node.id);
            }
        }
    }
}
