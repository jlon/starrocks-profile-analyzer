//! 
//! 
//! ```json
//! {
//!     {
//!     },
//!     {
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
    #[serde(skip, default)]
    pub node_class: NodeClass,
    #[serde(default)]
    pub properties: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub children: Vec<i32>,
}

impl TopologyNode {
    /// 
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
    ///
    ///
    pub fn parse(json_str: &str, profile_text: &str) -> ParseResult<TopologyGraph> {
        let json = Self::extract_json(json_str)?;

        let value: serde_json::Value = serde_json::from_str(json)
            .map_err(|e| ParseError::TopologyError(format!("Invalid JSON: {}", e)))?;

        let root_id = value.get("rootId")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| ParseError::TopologyError("Missing rootId".to_string()))? as i32;

        let nodes_array = value.get("nodes")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ParseError::TopologyError("Missing nodes array".to_string()))?;

        let mut nodes = Vec::new();
        for node_value in nodes_array {
            let node = Self::parse_node(node_value)?;
            nodes.push(node);
        }

        let mut extended_nodes = nodes;
        Self::extract_and_add_sink_nodes(&mut extended_nodes, profile_text, &[], root_id)?;

        Ok(TopologyGraph { root_id, nodes: extended_nodes })
    }

    pub fn parse_without_profile(json_str: &str) -> ParseResult<TopologyGraph> {
        Self::parse(json_str, "")
    }

    ///
    ///
    pub fn parse_with_fragments(
        json_str: &str,
        profile_text: &str,
        fragments: &[crate::models::Fragment]
    ) -> ParseResult<TopologyGraph> {
        let json = Self::extract_json(json_str)?;

        let value: serde_json::Value = serde_json::from_str(json)
            .map_err(|e| ParseError::TopologyError(format!("Invalid JSON: {}", e)))?;

        let root_id = value.get("rootId")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| ParseError::TopologyError("Missing rootId".to_string()))? as i32;

        let nodes_array = value.get("nodes")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ParseError::TopologyError("Missing nodes array".to_string()))?;

        let mut nodes = Vec::new();
        for node_value in nodes_array {
            let node = Self::parse_node(node_value)?;
            nodes.push(node);
        }

        let mut extended_nodes = nodes;
        Self::extract_and_add_sink_nodes(&mut extended_nodes, profile_text, fragments, root_id)?;

        Ok(TopologyGraph { root_id, nodes: extended_nodes })
    }

    ///
    ///
    fn extract_and_add_sink_nodes(
        nodes: &mut Vec<TopologyNode>,
        _profile_text: &str,
        fragments: &[crate::models::Fragment],
        _root_id: i32,
    ) -> ParseResult<()> {
        let selected_sink = Self::select_sink_node(fragments);

        if let Some(sink_name) = selected_sink {
            let sink_plan_id = Self::find_sink_plan_node_id(fragments, &sink_name);
            
            let sink_id = sink_plan_id.unwrap_or(-1);
            
            if !nodes.iter().any(|n| n.id == sink_id) {
                let node_class = TopologyNode::infer_node_class(&sink_name);
                let sink_node = TopologyNode {
                    id: sink_id,
                    name: sink_name.clone(),
                    node_class,
                    properties: HashMap::new(),
                    children: vec![],
                };
                nodes.push(sink_node);


            }
        }

        Ok(())
    }

    ///
    ///
    fn extract_operator_name(full_name: &str) -> String {
        if let Some(pos) = full_name.find(" (plan_node_id=") {
            full_name[..pos].to_string()
        } else {
            full_name.to_string()
        }
    }
    
    ///
    ///
    ///
    fn select_sink_node(fragments: &[crate::models::Fragment]) -> Option<String> {
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
        
        sink_candidates.sort_by(|a, b| {
            match (a.1, b.1) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.2.cmp(&b.2),
            }
        });
        
        if let Some((name, _is_final, _priority)) = sink_candidates.first() {
            Some(name.clone())
        } else {
            None
        }
    }
    
    ///
    ///
    fn is_final_sink(sink_name: &str) -> bool {
        if sink_name.contains("EXCHANGE_SINK") || sink_name.contains("LOCAL_EXCHANGE_SINK") {
            return false;
        }
        
        if sink_name.contains("MULTI_CAST") {
            return false;
        }
        
        true
    }
    
    ///
    ///
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
            6
        }
    }

    ///
    ///
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

    ///
    ///
    #[allow(dead_code)]
    fn find_sink_operators(profile_text: &str) -> Vec<String> {
        let mut sink_operators = Vec::new();

        for line in profile_text.lines() {
            let trimmed = line.trim();

            if trimmed.contains(" (plan_node_id=") && trimmed.contains(":") {
                if let Some(paren_start) = trimmed.find(" (plan_node_id=") {
                    let operator_name = trimmed[..paren_start].trim();

                    if operator_name.ends_with("_SINK") {
                        sink_operators.push(operator_name.to_string());
                    }
                }
            }
        }

        sink_operators
    }
    

    /// 
    pub fn build_relationships(topology: &TopologyGraph) -> HashMap<i32, Vec<i32>> {
        let mut relationships = HashMap::new();
        
        for node in &topology.nodes {
            relationships.insert(node.id, node.children.clone());
        }
        
        relationships
    }
    

    pub fn get_leaf_nodes(topology: &TopologyGraph) -> Vec<i32> {
        topology.nodes.iter()
            .filter(|n| n.children.is_empty())
            .map(|n| n.id)
            .collect()
    }
    

    pub fn get_ancestors(topology: &TopologyGraph, node_id: i32) -> Vec<i32> {
        let mut path = Vec::new();
        Self::find_path_to_node(topology, topology.root_id, node_id, &mut path);
        path
    }
    

    /// 

    pub fn validate(topology: &TopologyGraph) -> ParseResult<()> {

        if !topology.nodes.iter().any(|n| n.id == topology.root_id) {
            return Err(ParseError::TopologyError(
                format!("Root node {} not found", topology.root_id)
            ));
        }
        
        let node_ids: std::collections::HashSet<i32> = 
            topology.nodes.iter().map(|n| n.id).collect();
        

        for node in &topology.nodes {
            for child_id in &node.children {
                if !node_ids.contains(child_id) {
                    return Err(ParseError::TopologyError(
                        format!("Child node {} referenced but not found", child_id)
                    ));
                }
            }
        }
        
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
    
    
    fn extract_json(s: &str) -> ParseResult<&str> {
        let s = s.trim();
        

        if let Some(start) = s.find('{') {

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
                    children: vec![1],
                },
            ],
        };
        
        assert!(TopologyParser::validate(&topology).is_err());
    }
}
