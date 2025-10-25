//! # TopologyParser - 拓扑解析器
//! 
//! 负责解析 Execution 章节中的 Topology JSON 并构建节点关系图。
//! 
//! ## Topology 结构
//! ```json
//! {
//!   "rootId": 1,
//!   "nodes": [
//!     {
//!       "id": 1,
//!       "name": "EXCHANGE",
//!       "properties": {"sinkIds": [], "displayMem": true},
//!       "children": [0]
//!     },
//!     {
//!       "id": 0,
//!       "name": "OLAP_SCAN",
//!       "properties": {"sinkIds": [1], "displayMem": false},
//!       "children": []
//!     }
//!   ]
//! }
//! ```

use crate::parser::error::{ParseError, ParseResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyGraph {
    pub root_id: i32,
    pub nodes: Vec<TopologyNode>,
}

/// NodeClass: 对应StarRocks的ProfilingExecPlan.ProfilingElement.instanceOf
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeClass {
    ExchangeNode,
    ScanNode,
    JoinNode,
    AggregationNode,
    SortNode,
    ProjectNode,
    ResultSink,
    OlapTableSink,
    Unknown,
}

impl Default for NodeClass {
    fn default() -> Self {
        NodeClass::Unknown
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyNode {
    pub id: i32,
    pub name: String,
    #[serde(skip, default)]  // 不从JSON反序列化，而是从name推断
    pub node_class: NodeClass,
    #[serde(default)]
    pub properties: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub children: Vec<i32>,
}

impl TopologyNode {
    /// 从name推断node_class
    /// 
    /// 对应StarRocks的ProfilingExecPlan.ProfilingElement.instanceOf方法
    pub fn infer_node_class(name: &str) -> NodeClass {
        match name {
            "EXCHANGE" | "MERGE_EXCHANGE" => NodeClass::ExchangeNode,
            name if name.contains("SCAN") => NodeClass::ScanNode,
            name if name.contains("JOIN") => NodeClass::JoinNode,
            "AGGREGATE" | "AGGREGATION" => NodeClass::AggregationNode,
            "SORT" => NodeClass::SortNode,
            "PROJECT" => NodeClass::ProjectNode,
            "RESULT_SINK" => NodeClass::ResultSink,
            "OLAP_TABLE_SINK" => NodeClass::OlapTableSink,
            _ => NodeClass::Unknown,
        }
    }
}

pub struct TopologyParser;

impl TopologyParser {
    /// 解析 Topology JSON 字符串
    ///
    /// # Arguments
    /// * `json_str` - Topology JSON 字符串，可能包含 "Topology: " 前缀
    /// * `profile_text` - 完整的 Profile 文本，用于提取 SINK 节点
    ///
    /// # Returns
    /// * `Ok(TopologyGraph)` - 成功解析的拓扑图
    /// * `Err(ParseError)` - 解析失败
    pub fn parse(json_str: &str, profile_text: &str) -> ParseResult<TopologyGraph> {
        // 提取纯 JSON 部分（去除可能的前缀）
        let json = Self::extract_json(json_str)?;

        // 解析 JSON
        let value: serde_json::Value = serde_json::from_str(json)
            .map_err(|e| ParseError::TopologyError(format!("Invalid JSON: {}", e)))?;

        // 提取 rootId
        let root_id = value.get("rootId")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| ParseError::TopologyError("Missing rootId".to_string()))? as i32;

        // 提取 nodes
        let nodes_array = value.get("nodes")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ParseError::TopologyError("Missing nodes array".to_string()))?;

        let mut nodes = Vec::new();
        for node_value in nodes_array {
            let node = Self::parse_node(node_value)?;
            nodes.push(node);
        }

        // 提取并添加 SINK 节点
        let mut extended_nodes = nodes;
        Self::extract_and_add_sink_nodes(&mut extended_nodes, profile_text, &[], root_id)?;

        Ok(TopologyGraph { root_id, nodes: extended_nodes })
    }

    /// 解析 Topology JSON 字符串（兼容旧接口）
    pub fn parse_without_profile(json_str: &str) -> ParseResult<TopologyGraph> {
        Self::parse(json_str, "")
    }

    /// 解析 Topology JSON 字符串（带Fragments信息）
    ///
    /// # Arguments
    /// * `json_str` - Topology JSON 字符串，可能包含 "Topology: " 前缀
    /// * `profile_text` - 完整的 Profile 文本，用于提取 SINK 节点
    /// * `fragments` - 解析后的 Fragments 列表
    ///
    /// # Returns
    /// * `Ok(TopologyGraph)` - 成功解析的拓扑图
    /// * `Err(ParseError)` - 解析失败
    pub fn parse_with_fragments(
        json_str: &str,
        profile_text: &str,
        fragments: &[crate::models::Fragment]
    ) -> ParseResult<TopologyGraph> {
        // 提取纯 JSON 部分（去除可能的前缀）
        let json = Self::extract_json(json_str)?;

        // 解析 JSON
        let value: serde_json::Value = serde_json::from_str(json)
            .map_err(|e| ParseError::TopologyError(format!("Invalid JSON: {}", e)))?;

        // 提取 rootId
        let root_id = value.get("rootId")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| ParseError::TopologyError("Missing rootId".to_string()))? as i32;

        // 提取 nodes
        let nodes_array = value.get("nodes")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ParseError::TopologyError("Missing nodes array".to_string()))?;

        let mut nodes = Vec::new();
        for node_value in nodes_array {
            let node = Self::parse_node(node_value)?;
            nodes.push(node);
        }

        // 提取并添加 SINK 节点（使用Fragments信息）
        let mut extended_nodes = nodes;
        Self::extract_and_add_sink_nodes(&mut extended_nodes, profile_text, fragments, root_id)?;

        Ok(TopologyGraph { root_id, nodes: extended_nodes })
    }

    /// 提取并添加 SINK 节点到拓扑结构中
    ///
    /// # Arguments
    /// * `nodes` - 现有的拓扑节点列表（将被修改）
    /// * `profile_text` - 完整的 Profile 文本
    /// * `fragments` - 解析后的 Fragments 列表
    /// * `root_id` - 拓扑图的根节点ID
    ///
    /// # Returns
    /// * `Ok(())` - 成功添加 SINK 节点
    /// * `Err(ParseError)` - 处理失败
    fn extract_and_add_sink_nodes(
        nodes: &mut Vec<TopologyNode>,
        _profile_text: &str,
        fragments: &[crate::models::Fragment],
        _root_id: i32,
    ) -> ParseResult<()> {
        // 使用三层查找策略选择 SINK 节点
        let selected_sink = Self::select_sink_node(fragments);

        if let Some(sink_name) = selected_sink {
            // 从fragments中找到对应的SINK operator，获取其plan_node_id
            let sink_plan_id = Self::find_sink_plan_node_id(fragments, &sink_name);
            
            // 使用实际的plan_node_id，如果没有找到则使用-1
            let sink_id = sink_plan_id.unwrap_or(-1);
            
            // 检查是否已经存在相同ID的节点
            if !nodes.iter().any(|n| n.id == sink_id) {
                let node_class = TopologyNode::infer_node_class(&sink_name);
                let sink_node = TopologyNode {
                    id: sink_id,
                    name: sink_name.clone(),
                    node_class,
                    properties: HashMap::new(),
                    children: vec![], // SINK 节点的子节点关系将在tree_builder中建立
                };
                nodes.push(sink_node);

                // 注意：SINK节点与topology根节点的关系将在tree_builder中正确建立
                // 这里不建立任何父子关系，避免错误的topology结构
            }
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

    /// 从fragments中找到指定SINK节点的plan_node_id
    ///
    /// # Arguments
    /// * `fragments` - 解析后的Fragments列表
    /// * `sink_name` - SINK节点名称
    ///
    /// # Returns
    /// * `Option<i32>` - 找到的plan_node_id，如果没有找到则返回None
    fn find_sink_plan_node_id(fragments: &[crate::models::Fragment], sink_name: &str) -> Option<i32> {
        for fragment in fragments {
            for pipeline in &fragment.pipelines {
                for operator in &pipeline.operators {
                    if operator.name == sink_name {
                        if let Some(plan_id) = &operator.plan_node_id {
                            if let Ok(plan_id_int) = plan_id.parse::<i32>() {
                                return Some(plan_id_int);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// 从 profile 文本中找出所有以 _SINK 结尾的操作符
    ///
    /// # Arguments
    /// * `profile_text` - 完整的 Profile 文本
    ///
    /// # Returns
    /// * `Vec<String>` - SINK 操作符名称列表
    fn find_sink_operators(profile_text: &str) -> Vec<String> {
        let mut sink_operators = Vec::new();

        for line in profile_text.lines() {
            let trimmed = line.trim();

            // 查找操作符定义行，如 "OLAP_TABLE_SINK (plan_node_id=-1):"
            if trimmed.contains(" (plan_node_id=") && trimmed.contains(":") {
                if let Some(paren_start) = trimmed.find(" (plan_node_id=") {
                    let operator_name = trimmed[..paren_start].trim();

                    // 检查是否以 _SINK 结尾
                    if operator_name.ends_with("_SINK") {
                        sink_operators.push(operator_name.to_string());
                    }
                }
            }
        }

        sink_operators
    }
    
    /// 构建节点关系映射（快速查找）
    /// 
    /// # Returns
    /// * `HashMap<i32, Vec<i32>>` - 节点ID到其子节点ID列表的映射
    pub fn build_relationships(topology: &TopologyGraph) -> HashMap<i32, Vec<i32>> {
        let mut relationships = HashMap::new();
        
        for node in &topology.nodes {
            relationships.insert(node.id, node.children.clone());
        }
        
        relationships
    }
    
    /// 获取所有叶子节点（没有子节点的节点）
    pub fn get_leaf_nodes(topology: &TopologyGraph) -> Vec<i32> {
        topology.nodes.iter()
            .filter(|n| n.children.is_empty())
            .map(|n| n.id)
            .collect()
    }
    
    /// 获取节点的所有祖先（从根到该节点的路径）
    pub fn get_ancestors(topology: &TopologyGraph, node_id: i32) -> Vec<i32> {
        let mut path = Vec::new();
        Self::find_path_to_node(topology, topology.root_id, node_id, &mut path);
        path
    }
    
    /// 验证拓扑图的有效性
    /// 
    /// 检查：
    /// 1. rootId 对应的节点存在
    /// 2. 所有 children 引用的节点都存在
    /// 3. 没有环路
    pub fn validate(topology: &TopologyGraph) -> ParseResult<()> {
        // 检查 root 存在
        if !topology.nodes.iter().any(|n| n.id == topology.root_id) {
            return Err(ParseError::TopologyError(
                format!("Root node {} not found", topology.root_id)
            ));
        }
        
        // 构建节点ID集合
        let node_ids: std::collections::HashSet<i32> = 
            topology.nodes.iter().map(|n| n.id).collect();
        
        // 检查所有子节点引用
        for node in &topology.nodes {
            for child_id in &node.children {
                if !node_ids.contains(child_id) {
                    return Err(ParseError::TopologyError(
                        format!("Child node {} referenced but not found", child_id)
                    ));
                }
            }
        }
        
        // 检查环路（使用 DFS）
        let mut visited = std::collections::HashSet::new();
        let mut rec_stack = std::collections::HashSet::new();
        
        for node in &topology.nodes {
            if !visited.contains(&node.id) {
                if Self::has_cycle(topology, node.id, &mut visited, &mut rec_stack)? {
                    return Err(ParseError::TopologyError(
                        "Cycle detected in topology graph".to_string()
                    ));
                }
            }
        }
        
        Ok(())
    }
    
    // ========== Private Helper Methods ==========
    
    fn extract_json(s: &str) -> ParseResult<&str> {
        let s = s.trim();
        
        // 查找第一个 '{' 字符
        if let Some(start) = s.find('{') {
            // 找到匹配的 '}'
            let mut depth = 0;
            let mut end = start;
            
            for (i, ch) in s[start..].char_indices() {
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
                return Ok(&s[start..end]);
            }
        }
        
        Err(ParseError::TopologyError("No valid JSON found".to_string()))
    }
    
    fn parse_node(value: &serde_json::Value) -> ParseResult<TopologyNode> {
        let id = value.get("id")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| ParseError::TopologyError("Node missing id".to_string()))? as i32;
        
        let name = value.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ParseError::TopologyError("Node missing name".to_string()))?
            .to_string();
        
        let properties = value.get("properties")
            .and_then(|v| v.as_object())
            .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();
        
        let children = value.get("children")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_i64().map(|i| i as i32)).collect())
            .unwrap_or_default();
        
        // 从name推断node_class
        let node_class = TopologyNode::infer_node_class(&name);
        
        Ok(TopologyNode { id, name, node_class, properties, children })
    }
    
    fn find_path_to_node(
        topology: &TopologyGraph,
        current: i32,
        target: i32,
        path: &mut Vec<i32>,
    ) -> bool {
        path.push(current);
        
        if current == target {
            return true;
        }
        
        if let Some(node) = topology.nodes.iter().find(|n| n.id == current) {
            for &child in &node.children {
                if Self::find_path_to_node(topology, child, target, path) {
                    return true;
                }
            }
        }
        
        path.pop();
        false
    }
    
    fn has_cycle(
        topology: &TopologyGraph,
        node_id: i32,
        visited: &mut std::collections::HashSet<i32>,
        rec_stack: &mut std::collections::HashSet<i32>,
    ) -> ParseResult<bool> {
        visited.insert(node_id);
        rec_stack.insert(node_id);
        
        if let Some(node) = topology.nodes.iter().find(|n| n.id == node_id) {
            for &child_id in &node.children {
                if !visited.contains(&child_id) {
                    if Self::has_cycle(topology, child_id, visited, rec_stack)? {
                        return Ok(true);
                    }
                } else if rec_stack.contains(&child_id) {
                    return Ok(true);
                }
            }
        }
        
        rec_stack.remove(&node_id);
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_topology() {
        let json = r#"{
            "rootId": 1,
            "nodes": [
                {
                    "id": 1,
                    "name": "EXCHANGE",
                    "properties": {"sinkIds": []},
                    "children": [0]
                },
                {
                    "id": 0,
                    "name": "OLAP_SCAN",
                    "properties": {},
                    "children": []
                }
            ]
        }"#;
        
        let topology = TopologyParser::parse(json, "").unwrap();
        assert_eq!(topology.root_id, 1);
        assert_eq!(topology.nodes.len(), 2);
        assert_eq!(topology.nodes[0].name, "EXCHANGE");
    }
    
    #[test]
    fn test_validate_topology() {
        let topology = TopologyGraph {
            root_id: 1,
            nodes: vec![
                TopologyNode {
                    id: 1,
                    name: "ROOT".to_string(),
                    node_class: NodeClass::Unknown,
                    properties: HashMap::new(),
                    children: vec![0],
                },
                TopologyNode {
                    id: 0,
                    name: "LEAF".to_string(),
                    node_class: NodeClass::Unknown,
                    properties: HashMap::new(),
                    children: vec![],
                },
            ],
        };
        
        assert!(TopologyParser::validate(&topology).is_ok());
    }
    
    #[test]
    fn test_detect_cycle() {
        let topology = TopologyGraph {
            root_id: 1,
            nodes: vec![
                TopologyNode {
                    id: 1,
                    name: "A".to_string(),
                    node_class: NodeClass::Unknown,
                    properties: HashMap::new(),
                    children: vec![2],
                },
                TopologyNode {
                    id: 2,
                    name: "B".to_string(),
                    node_class: NodeClass::Unknown,
                    properties: HashMap::new(),
                    children: vec![1], // 环路！
                },
            ],
        };
        
        assert!(TopologyParser::validate(&topology).is_err());
    }
}
