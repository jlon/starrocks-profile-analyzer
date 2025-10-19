//! # OperatorParser - 操作符解析器
//! 
//! 负责解析 Operator 的头部信息和提取 Operator 块。
//! 
//! ## Operator 头部格式
//! ```text
//! CONNECTOR_SCAN (plan_node_id=0):
//! HASH_JOIN (plan_node_id=1) (operator id=2):
//! EXCHANGE_SINK (plan_node_id=3):
//! RESULT_SINK (plan_node_id=-1):
//! ```

use crate::models::NodeType;
use crate::parser::error::{ParseError, ParseResult};
use once_cell::sync::Lazy;
use regex::Regex;

static OPERATOR_HEADER_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^([A-Z_]+)\s*\(plan_node_id=(-?\d+)\)(?:\s*\(operator\s+id=(\d+)\))?:?").unwrap()
});

#[derive(Debug, Clone, PartialEq)]
pub struct OperatorHeader {
    pub name: String,
    pub plan_node_id: i32,
    pub operator_id: Option<i32>,
}

pub struct OperatorParser;

impl OperatorParser {
    /// 解析 Operator 头部行
    /// 
    /// # Examples
    /// ```
    /// use starrocks_profile_analyzer::parser::OperatorParser;
    /// let header = OperatorParser::parse_header("CONNECTOR_SCAN (plan_node_id=0):").unwrap();
    /// assert_eq!(header.name, "CONNECTOR_SCAN");
    /// assert_eq!(header.plan_node_id, 0);
    /// ```
    pub fn parse_header(line: &str) -> ParseResult<OperatorHeader> {
        let line = line.trim();
        
        OPERATOR_HEADER_REGEX.captures(line)
            .ok_or_else(|| ParseError::OperatorError(format!("Invalid operator header: {}", line)))
            .and_then(|caps| {
                let name = caps.get(1)
                    .ok_or_else(|| ParseError::OperatorError("Missing operator name".to_string()))?
                    .as_str()
                    .to_string();
                
                let plan_node_id = caps.get(2)
                    .and_then(|m| m.as_str().parse::<i32>().ok())
                    .ok_or_else(|| ParseError::OperatorError("Invalid plan_node_id".to_string()))?;
                
                let operator_id = caps.get(3)
                    .and_then(|m| m.as_str().parse::<i32>().ok());
                
                Ok(OperatorHeader { name, plan_node_id, operator_id })
            })
    }
    
    /// 提取指定 Operator 的完整文本块
    /// 
    /// 支持通过 operator_name 和 plan_node_id 定位 Operator。
    /// 会自动处理 Topology 名称映射（如 OLAP_SCAN -> CONNECTOR_SCAN）。
    /// 
    /// # Arguments
    /// * `text` - 完整的 Profile 文本
    /// * `operator_name` - Operator 名称（可以是 Topology 名称）
    /// * `plan_node_id` - 计划节点 ID
    pub fn extract_operator_block(
        text: &str,
        operator_name: &str,
        plan_node_id: Option<i32>,
    ) -> String {
        // Topology 名称到实际 Profile 名称的映射
        let actual_names = Self::map_topology_to_actual_names(operator_name);
        
        // 优先使用 plan_node_id 精确匹配
        if let Some(plan_id) = plan_node_id {
            for actual_name in &actual_names {
                let patterns = vec![
                    format!("{} (plan_node_id={}):", actual_name, plan_id),
                    format!("{}(plan_node_id={})", actual_name, plan_id),
                ];
                
                for pattern in patterns {
                    if let Some(block) = Self::extract_block_by_pattern(text, &pattern) {
                        return block;
                    }
                }
            }
        }
        
        // 回退：只使用名称查找
        for actual_name in &actual_names {
            let pattern = format!("{} (plan_node_id", actual_name);
            if let Some(block) = Self::extract_first_block_by_name(text, &pattern) {
                return block;
            }
        }
        
        String::new()
    }
    
    /// 确定 Operator 的类型
    /// 
    /// 基于 Operator 名称映射到 NodeType 枚举。
    pub fn determine_node_type(operator_name: &str) -> NodeType {
        match operator_name {
            "OLAP_SCAN" => NodeType::OlapScan,
            "CONNECTOR_SCAN" | "ES_SCAN" => NodeType::ConnectorScan,
            "HASH_JOIN" | "NL_JOIN" | "CROSS_JOIN" | "NEST_LOOP_JOIN" => NodeType::HashJoin,
            "AGGREGATE" | "AGG" | "AGGREGATION" => NodeType::Aggregate,
            "LIMIT" => NodeType::Limit,
            "EXCHANGE_SINK" => NodeType::ExchangeSink,
            "EXCHANGE_SOURCE" | "EXCHANGE" | "MERGE_EXCHANGE" => NodeType::ExchangeSource,
            "RESULT_SINK" => NodeType::ResultSink,
            "CHUNK_ACCUMULATE" => NodeType::ChunkAccumulate,
            "SORT" | "LOCAL_SORT" => NodeType::Sort,
            "PROJECT" | "FILTER" | "TABLE_FUNCTION" => NodeType::Unknown, // 暂时映射到Unknown
            _ => NodeType::Unknown,
        }
    }
    
    /// 标准化 Operator 名称
    /// 
    /// 处理别名和大小写不一致问题。
    pub fn normalize_name(name: &str) -> String {
        let upper = name.to_uppercase();
        match upper.as_str() {
            "ES_SCAN" => "CONNECTOR_SCAN".to_string(),
            "AGG" => "AGGREGATE".to_string(),
            "AGGREGATION" => "AGGREGATE".to_string(),
            "NL_JOIN" | "NEST_LOOP_JOIN" | "CROSS_JOIN" => "HASH_JOIN".to_string(),
            _ => upper,
        }
    }

    /// 检查是否是 Operator 头部行
    pub fn is_operator_header(line: &str) -> bool {
        OPERATOR_HEADER_REGEX.is_match(line.trim())
    }

    /// 将各种 Operator 名称映射到 Topology 的规范名称
    ///
    /// 注意：保持 OLAP_SCAN 与 CONNECTOR_SCAN 的区分；
    /// EXCHANGE_SOURCE/EXCHANGE_SINK 归并为 EXCHANGE；
    /// LOCAL_SORT -> SORT；AGGREGATION -> AGGREGATE；NL_JOIN/CROSS_JOIN -> HASH_JOIN。
    pub fn canonical_topology_name(name: &str) -> String {
        match name.to_uppercase().as_str() {
            // 扫描类：保持区分
            "OLAP_SCAN" => "OLAP_SCAN".to_string(),
            "CONNECTOR_SCAN" | "ES_SCAN" => "CONNECTOR_SCAN".to_string(),
            // 交换类统一为 EXCHANGE
            "EXCHANGE_SOURCE" | "EXCHANGE_SINK" | "EXCHANGE" | "MERGE_EXCHANGE" => "EXCHANGE".to_string(),
            // 统计收集类：保持独立，不应该被聚合到其他操作符
            "COLLECT_STATS_SOURCE" | "COLLECT_STATS_SINK" => "COLLECT_STATS".to_string(),
            // 聚合统一
            "AGG" | "AGGREGATION" | "AGGREGATE" => "AGGREGATE".to_string(),
            // 排序统一
            "LOCAL_SORT" | "SORT" => "SORT".to_string(),
            // JOIN 统一到 HASH_JOIN（Topology 中 JOIN 类型通常以算子类别呈现）
            "NL_JOIN" | "NEST_LOOP_JOIN" | "CROSS_JOIN" | "HASH_JOIN" => "HASH_JOIN".to_string(),
            // 其它保持不变（大写形式）
            other => other.to_string(),
        }
    }

    // ========== Private Helper Methods ==========
    
    fn map_topology_to_actual_names(topology_name: &str) -> Vec<String> {
        let names: Vec<&str> = match topology_name {
            // OLAP_SCAN 在执行时可能实现为 CONNECTOR_SCAN，需要同时匹配两者
            "OLAP_SCAN" => vec!["OLAP_SCAN", "CONNECTOR_SCAN"],
            // CONNECTOR_SCAN 包含 ES_SCAN 别名
            "CONNECTOR_SCAN" => vec!["CONNECTOR_SCAN", "ES_SCAN"],
            // 交换类
            "EXCHANGE" => vec!["EXCHANGE_SOURCE", "EXCHANGE_SINK"],
            "MERGE_EXCHANGE" => vec!["MERGE_EXCHANGE", "EXCHANGE_SOURCE"],
            // SINK 类
            "RESULT_SINK" => vec!["RESULT_SINK"],
            // JOIN 类
            "HASH_JOIN" => vec!["HASH_JOIN", "NL_JOIN", "CROSS_JOIN"],
            // 聚合类
            "AGGREGATE" | "AGGREGATION" => vec!["AGGREGATE", "AGGREGATION"],
            // 其它
            "LIMIT" => vec!["LIMIT"],
            "SORT" => vec!["SORT", "LOCAL_SORT"],
            "PROJECT" => vec!["PROJECT"],
            "FILTER" => vec!["FILTER"],
            "TABLE_FUNCTION" => vec!["TABLE_FUNCTION"],
            _ => vec![topology_name],
        };
        names.into_iter().map(|s| s.to_string()).collect()
    }
    
    fn extract_block_by_pattern(text: &str, pattern: &str) -> Option<String> {
        text.find(pattern).and_then(|start_pos| {
            // 找到行首
            let line_start = text[..start_pos].rfind('\n').map(|i| i + 1).unwrap_or(0);
            
            // 获取基础缩进
            let base_indent = Self::get_indent(&text[line_start..start_pos]);
            
            // 找到块的结束位置
            let rest = &text[line_start..];
            let mut end_pos = rest.len();
            
            for (i, line) in rest.lines().enumerate().skip(1) {
                if !line.trim().is_empty() {
                    let line_indent = Self::get_indent(line);
                    
                    // 遇到同级或更低缩进的 Operator 头部，结束
                    if line_indent <= base_indent && Self::is_operator_header(line) {
                        // 计算到当前位置的字节数
                        end_pos = rest.lines().take(i).map(|l| l.len() + 1).sum();
                        break;
                    }
                }
            }
            
            Some(rest[..end_pos].to_string())
        })
    }
    
    fn extract_first_block_by_name(text: &str, name_pattern: &str) -> Option<String> {
        if let Some(start) = text.find(name_pattern) {
            let line_start = text[..start].rfind('\n').map(|i| i + 1).unwrap_or(0);
            Self::extract_block_by_pattern(text, &text[line_start..start + name_pattern.len()])
        } else {
            None
        }
    }
    
    fn get_indent(line: &str) -> usize {
        line.chars().take_while(|c| c.is_whitespace()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_header() {
        let header = OperatorParser::parse_header("CONNECTOR_SCAN (plan_node_id=0):").unwrap();
        assert_eq!(header.name, "CONNECTOR_SCAN");
        assert_eq!(header.plan_node_id, 0);
        assert_eq!(header.operator_id, None);
        
        let header2 = OperatorParser::parse_header("HASH_JOIN (plan_node_id=1) (operator id=2):").unwrap();
        assert_eq!(header2.operator_id, Some(2));
    }
    
    #[test]
    fn test_determine_node_type() {
        assert_eq!(OperatorParser::determine_node_type("CONNECTOR_SCAN"), NodeType::ConnectorScan);
        assert_eq!(OperatorParser::determine_node_type("HASH_JOIN"), NodeType::HashJoin);
        assert_eq!(OperatorParser::determine_node_type("UNKNOWN_OP"), NodeType::Unknown);
    }
    
    #[test]
    fn test_normalize_name() {
        assert_eq!(OperatorParser::normalize_name("es_scan"), "CONNECTOR_SCAN");
        assert_eq!(OperatorParser::normalize_name("AGG"), "AGGREGATE");
        assert_eq!(OperatorParser::normalize_name("nl_join"), "HASH_JOIN");
    }
    
    #[test]
    fn test_is_operator_header() {
        assert!(OperatorParser::is_operator_header("CONNECTOR_SCAN (plan_node_id=0):"));
        assert!(OperatorParser::is_operator_header("RESULT_SINK (plan_node_id=-1):"));
        assert!(!OperatorParser::is_operator_header("  - ScanTime: 5s"));
    }
}

