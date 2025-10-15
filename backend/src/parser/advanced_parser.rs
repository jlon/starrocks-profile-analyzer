use crate::models::*;
use std::collections::HashMap;
use regex::Regex;
use once_cell::sync::Lazy;
use std::time::Duration;
use crate::StarRocksProfileParser;

// 预编译正则表达式
static OPERATOR_LINE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"([A-Z_]+)\s*\(plan_node_id=(-?\d+)(?:\s*\(operator\s+id=(\d+)\))?\)").unwrap()
});

static METRIC_LINE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s*-\s+([A-Za-z][A-Za-z0-9_]*(?:\s+[A-Za-z][A-Za-z0-9_]*)*):\s+(.+)$").unwrap()
});

pub struct AdvancedStarRocksProfileParser;

impl AdvancedStarRocksProfileParser {
    pub fn parse_advanced(input: &str) -> Result<Profile, String> {
        let mut profile = StarRocksProfileParser::parse(input)?;

        // 构建高级的执行树
        let execution_tree = Self::build_execution_tree(input)?;
        profile.execution_tree = Some(execution_tree);

        Ok(profile)
    }

    pub fn build_execution_tree(profile_text: &str) -> Result<ExecutionTree, String> {
        let mut nodes = Vec::new();
        let mut node_map = HashMap::new();

        // 分割Profile文本为操作符块
        let operator_blocks = Self::split_into_operator_blocks(profile_text)?;

        // 解析每个操作符块
        for block in operator_blocks {
            let node = Self::parse_operator_block(&block)?;
            node_map.insert(node.plan_node_id.unwrap_or(-1), node.clone());
            nodes.push(node);
        }

        // 构建树结构
        Self::build_tree_relationships(&mut nodes, &node_map);

        // 确定根节点（通常是第一个节点）
        if nodes.is_empty() {
            return Err("No operators found in profile".to_string());
        }

        let root = nodes[0].clone();

        Ok(ExecutionTree { root, nodes })
    }

    fn split_into_operator_blocks(profile_text: &str) -> Result<Vec<String>, String> {
        let lines: Vec<&str> = profile_text.lines().collect();
        let mut blocks = Vec::new();
        let mut current_block = String::new();
        let mut in_operator_block = false;

        for line in lines {
            let trimmed = line.trim();

            // 检查是否是操作符开始行
            if Self::is_operator_start_line(trimmed) {
                // 保存之前的块
                if !current_block.is_empty() {
                    blocks.push(current_block);
                }
                current_block = line.to_string();
                in_operator_block = true;
            } else if in_operator_block {
                current_block.push('\n');
                current_block.push_str(line);

                // 检查是否是操作符结束（下一行是空行或者开始新的操作符）
                if trimmed.is_empty() && !current_block.trim().is_empty() {
                    in_operator_block = false;
                }
            }
        }

        if !current_block.is_empty() {
            blocks.push(current_block);
        }

        Ok(blocks)
    }

    fn is_operator_start_line(line: &str) -> bool {
        // 根据StarRocks Profile格式，操作符行格式如：
        // "RESULT_SINK (plan_node_id=-1):"
        // "CHUNK_ACCUMULATE (plan_node_id=-1):"
        // "LIMIT (plan_node_id=1) (operator id=1):"
        OPERATOR_LINE_REGEX.is_match(line)
    }

    fn parse_operator_block(block: &str) -> Result<ExecutionTreeNode, String> {
        let lines: Vec<&str> = block.lines().collect();

        if lines.is_empty() {
            return Err("Empty operator block".to_string());
        }

        // 解析操作符标题行
        let first_line = lines[0].trim();
        let (operator_name, plan_node_id, operator_id) = Self::parse_operator_header(first_line)?;

        // 解析指标
        let metrics = Self::parse_operator_metrics(&lines[1..])?;

        // 确定节点类型
        let node_type = Self::determine_node_type(&operator_name);

        let node = ExecutionTreeNode {
            id: format!("node_{}", plan_node_id.unwrap_or(-1)),
            operator_name: operator_name.clone(),
            node_type,
            plan_node_id,
            parent_plan_node_id: None, // 稍后构建树关系时设置
            metrics,
            children: Vec::new(),
            depth: 0,
            is_hotspot: false, // 默认值，将在分析时设置
            hotspot_severity: HotSeverity::Normal,
        };

        Ok(node)
    }

    fn parse_operator_header(line: &str) -> Result<(String, Option<i32>, Option<String>), String> {
        println!("🔍 Debug parse header: {}", line);
        let captures = OPERATOR_LINE_REGEX.captures(line)
            .ok_or_else(|| format!("Invalid operator header: {}", line))?;

        let operator_name = captures.get(1)
            .ok_or("Missing operator name")?
            .as_str()
            .to_string();

        let plan_node_id = captures.get(2)
            .and_then(|m| m.as_str().parse::<i32>().ok());

        let operator_id = captures.get(3)
            .map(|m| m.as_str().to_string());

        println!("  ✅ Parsed: name={}, plan_id={:?}, op_id={:?}", operator_name, plan_node_id, operator_id);
        Ok((operator_name, plan_node_id, operator_id))
    }

    fn parse_operator_metrics(lines: &[&str]) -> Result<OperatorMetrics, String> {
        let mut metrics = OperatorMetrics {
            operator_total_time: None,
            push_chunk_num: None,
            push_row_num: None,
            pull_chunk_num: None,
            pull_row_num: None,
            push_total_time: None,
            pull_total_time: None,
            memory_usage: None,
            output_chunk_bytes: None,
            specialized: OperatorSpecializedMetrics::None,
        };

        for line in lines {
            if let Some((key, value)) = Self::parse_metric_line(line) {
                Self::set_metric_value(&mut metrics, &key, &value);
            }
        }

        Ok(metrics)
    }

    fn parse_metric_line(line: &str) -> Option<(String, String)> {
        METRIC_LINE_REGEX.captures(line)
            .and_then(|caps| {
                let key = caps.get(1)?.as_str().replace(" ", "");
                let value = caps.get(2)?.as_str().to_string();
                Some((key, value))
            })
    }

    fn set_metric_value(metrics: &mut OperatorMetrics, key: &str, value: &str) {
        match key {
            "OperatorTotalTime" => {
                metrics.operator_total_time = Self::parse_duration(value).ok();
            }
            "PushChunkNum" => {
                metrics.push_chunk_num = Self::parse_number(value).ok();
            }
            "PushRowNum" => {
                metrics.push_row_num = Self::parse_number(value).ok();
            }
            "PullChunkNum" => {
                metrics.pull_chunk_num = Self::parse_number(value).ok();
            }
            "PullRowNum" => {
                metrics.pull_row_num = Self::parse_number(value).ok();
            }
            "PushTotalTime" => {
                metrics.push_total_time = Self::parse_duration(value).ok();
            }
            "PullTotalTime" => {
                metrics.pull_total_time = Self::parse_duration(value).ok();
            }
            "OutputChunkBytes" => {
                metrics.output_chunk_bytes = Self::parse_bytes(value).ok();
            }
            // 处理专用指标，这里根据操作符类型处理
            _ => {}
        }
    }

    fn parse_duration(value: &str) -> Result<Duration, String> {
        // 解析时间格式，如 "1h30m", "5s499ms", "0ns"
        if value.contains("h") {
            let hours: f64 = value.split("h").next()
                .ok_or("Invalid hours format")?
                .parse()
                .map_err(|_| "Invalid hours number")?;
            let minutes: f64 = value.split("h").nth(1)
                .unwrap_or("0m")
                .trim_end_matches("m")
                .parse()
                .unwrap_or(0.0);
            Ok(Duration::from_millis(((hours * 3600.0 + minutes * 60.0) * 1000.0) as u64))
        } else if value.contains("m") {
            let minutes: f64 = value.split("m").next()
                .ok_or("Invalid minutes format")?
                .parse()
                .map_err(|_| "Invalid minutes number")?;
            Ok(Duration::from_millis((minutes * 60.0 * 1000.0) as u64))
        } else if value.contains("s") {
            let seconds: f64 = value.split("s").next()
                .ok_or("Invalid seconds format")?
                .parse()
                .map_err(|_| "Invalid seconds number")?;
            Ok(Duration::from_millis((seconds * 1000.0) as u64))
        } else if value.contains("ms") {
            let ms: f64 = value.split("ms").next()
                .ok_or("Invalid ms format")?
                .parse()
                .map_err(|_| "Invalid ms number")?;
            Ok(Duration::from_millis(ms as u64))
        } else if value.contains("us") || value.contains("μs") {
            let us: f64 = value.split("us").next()
                .unwrap_or(value.trim_end_matches("μs"))
                .parse()
                .map_err(|_| "Invalid us number")?;
            Ok(Duration::from_micros(us as u64))
        } else if value.contains("ns") {
            let ns: f64 = value.split("ns").next()
                .ok_or("Invalid ns format")?
                .parse()
                .map_err(|_| "Invalid ns number")?;
            Ok(Duration::from_nanos(ns as u64))
        } else {
            Err(format!("Unknown duration format: {}", value))
        }
    }

    fn parse_number(value: &str) -> Result<u64, String> {
        value.replace(",", "").parse().map_err(|_| format!("Invalid number: {}", value))
    }

    fn parse_bytes(value: &str) -> Result<u64, String> {
        // 解析字节格式，如 "2.174K (2174)", "1.463 KB", "18.604 MB"
        let clean_value = value.split_whitespace().next().unwrap_or(value);

        if clean_value.contains("GB") {
            let gb: f64 = clean_value.replace("GB", "").parse().map_err(|_| "Invalid GB format")?;
            Ok((gb * 1024.0 * 1024.0 * 1024.0) as u64)
        } else if clean_value.contains("MB") {
            let mb: f64 = clean_value.replace("MB", "").parse().map_err(|_| "Invalid MB format")?;
            Ok((mb * 1024.0 * 1024.0) as u64)
        } else if clean_value.contains("KB") || clean_value.contains("K") {
            let kb: f64 = clean_value.replace("KB", "").replace("K", "").parse().map_err(|_| "Invalid KB format")?;
            Ok((kb * 1024.0) as u64)
        } else {
            clean_value.replace(",", "").parse().map_err(|_| format!("Invalid bytes format: {}", value))
        }
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

    fn build_tree_relationships(nodes: &mut [ExecutionTreeNode], node_map: &HashMap<i32, ExecutionTreeNode>) {
        // 根据plan_node_id建立父子关系
        // 这是一个简化的实现，实际需要更复杂的逻辑来推断数据流
        for i in 0..nodes.len() {
            for j in 0..nodes.len() {
                if i != j {
                    // 如果另一个节点的plan_node_id是当前节点的依赖
                    // 这里需要更复杂的逻辑来建立正确的树关系
                    // 简化起见，我们假设按照plan_node_id的顺序构建简单的树
                }
            }
        }
    }
}
