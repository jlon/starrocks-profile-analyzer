//! # ProfileNodeParser - Profile节点解析器
//! 
//! 从Fragment的RuntimeProfile中提取operators，按plan_node_id分组
//! 
//! 对应StarRocks的ExplainAnalyzer.ProfileNodeParser (line 1582-1751)

use crate::models::{Fragment, Operator};
use std::collections::HashMap;
use regex::Regex;
use once_cell::sync::Lazy;

static PLAN_NODE_ID_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"plan_node_id=(-?\d+)").unwrap()
});

/// ProfileNodeParser: 从Fragment中按plan_node_id聚合operators
/// 
/// 对应StarRocks的ExplainAnalyzer.ProfileNodeParser类
pub struct ProfileNodeParser {
    fragment: Fragment,
}

impl ProfileNodeParser {
    pub fn new(fragment: Fragment) -> Self {
        Self { fragment }
    }
    
    /// 解析所有operators，按plan_node_id分组
    /// 
    /// 返回: HashMap<plan_node_id, (native_operators, subordinate_operators)>
    /// 
    /// # Returns
    /// - native_operators: 直接对应plan node的operators（如EXCHANGE_SINK, AGGREGATE_BLOCKING_SOURCE）
    /// - subordinate_operators: 辅助operators（如LOCAL_EXCHANGE, CHUNK_ACCUMULATE）
    pub fn parse(&self) -> HashMap<i32, (Vec<Operator>, Vec<Operator>)> {
        let mut node_map: HashMap<i32, (Vec<Operator>, Vec<Operator>)> = HashMap::new();
        
        println!("DEBUG ProfileNodeParser: Parsing fragment {}", self.fragment.id);
        println!("DEBUG ProfileNodeParser: Fragment has {} pipelines", self.fragment.pipelines.len());
        
        // 遍历所有pipelines和operators
        for pipeline in &self.fragment.pipelines {
            println!("DEBUG ProfileNodeParser: Pipeline {} has {} operators", 
                pipeline.id, pipeline.operators.len());
            
            for operator in &pipeline.operators {
                println!("DEBUG ProfileNodeParser: Operator name: '{}', plan_node_id: {:?}", 
                    operator.name, operator.plan_node_id);
                
                // 直接使用operator.plan_node_id字段
                if let Some(ref plan_id_str) = operator.plan_node_id {
                    if let Ok(plan_id) = plan_id_str.parse::<i32>() {
                        println!("DEBUG ProfileNodeParser: Using plan_node_id={} from operator '{}'", 
                            plan_id, operator.name);
                        
                        let entry = node_map.entry(plan_id).or_insert((Vec::new(), Vec::new()));
                        
                        // 区分native和subordinate
                        if Self::is_subordinate_operator(&operator.name) {
                            println!("DEBUG ProfileNodeParser: {} is subordinate", operator.name);
                            entry.1.push(operator.clone());
                        } else {
                            println!("DEBUG ProfileNodeParser: {} is native", operator.name);
                            entry.0.push(operator.clone());
                        }
                    } else {
                        println!("DEBUG ProfileNodeParser: Failed to parse plan_node_id '{}' as i32", plan_id_str);
                    }
                } else {
                    println!("DEBUG ProfileNodeParser: Operator '{}' has no plan_node_id", operator.name);
                }
            }
        }
        
        println!("DEBUG ProfileNodeParser: Extracted {} plan_node_ids", node_map.len());
        for (plan_id, (native, sub)) in &node_map {
            println!("DEBUG ProfileNodeParser: plan_node_id={}: {} native, {} subordinate", 
                plan_id, native.len(), sub.len());
        }
        
        node_map
    }
    
    /// 从operator名称中提取plan_node_id
    /// 
    /// # Examples
    /// ```
    /// assert_eq!(extract_plan_node_id("EXCHANGE_SINK (plan_node_id=1)"), Some(1));
    /// assert_eq!(extract_plan_node_id("RESULT_SINK (plan_node_id=-1)"), Some(-1));
    /// ```
    fn extract_plan_node_id(operator_name: &str) -> Option<i32> {
        PLAN_NODE_ID_REGEX.captures(operator_name)
            .and_then(|caps| caps.get(1))
            .and_then(|m| m.as_str().parse().ok())
    }
    
    /// 判断是否为subordinate operator
    /// 
    /// Subordinate operators是辅助operators，不直接对应plan node
    /// 
    /// # Examples
    /// - LOCAL_EXCHANGE
    /// - CHUNK_ACCUMULATE
    /// - CACHE
    /// - COLLECT_STATS_SOURCE/SINK
    fn is_subordinate_operator(name: &str) -> bool {
        name.contains("LOCAL_EXCHANGE") ||
        name.contains("CHUNK_ACCUMULATE") ||
        name.contains("CACHE") ||
        name.contains("COLLECT_STATS")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extract_plan_node_id() {
        assert_eq!(
            ProfileNodeParser::extract_plan_node_id("EXCHANGE_SINK (plan_node_id=1)"),
            Some(1)
        );
        assert_eq!(
            ProfileNodeParser::extract_plan_node_id("RESULT_SINK (plan_node_id=-1)"),
            Some(-1)
        );
        assert_eq!(
            ProfileNodeParser::extract_plan_node_id("CONNECTOR_SCAN (plan_node_id=0)"),
            Some(0)
        );
        assert_eq!(
            ProfileNodeParser::extract_plan_node_id("INVALID_OPERATOR"),
            None
        );
    }
    
    #[test]
    fn test_is_subordinate_operator() {
        assert!(ProfileNodeParser::is_subordinate_operator("LOCAL_EXCHANGE_SINK (plan_node_id=1)"));
        assert!(ProfileNodeParser::is_subordinate_operator("CHUNK_ACCUMULATE (plan_node_id=2)"));
        assert!(ProfileNodeParser::is_subordinate_operator("CACHE (plan_node_id=3)"));
        assert!(!ProfileNodeParser::is_subordinate_operator("EXCHANGE_SINK (plan_node_id=1)"));
        assert!(!ProfileNodeParser::is_subordinate_operator("AGGREGATE_BLOCKING_SOURCE (plan_node_id=2)"));
    }
}

