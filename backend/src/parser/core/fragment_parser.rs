//! # FragmentParser - Fragment 解析器
//! 
//! 负责解析 Fragment 和 Pipeline 结构。

use crate::models::{Fragment, Pipeline, Operator};
use crate::parser::error::ParseResult;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;

static FRAGMENT_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s*Fragment\s+(\d+):").unwrap()
});

static PIPELINE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s*Pipeline\s+\(id=(\d+)\):").unwrap()
});

pub struct FragmentParser;

impl FragmentParser {
    /// 解析单个 Fragment
    /// 
    /// # Arguments
    /// * `text` - Fragment 的完整文本块
    /// * `id` - Fragment ID
    pub fn parse_fragment(text: &str, id: &str) -> ParseResult<Fragment> {
        let backend_addresses = Self::extract_backend_addresses(text);
        let instance_ids = Self::extract_instance_ids(text);
        let pipelines = Self::parse_pipelines(text)?;
        
        Ok(Fragment {
            id: id.to_string(),
            backend_addresses,
            instance_ids,
            pipelines,
        })
    }
    
    /// 从 Profile 文本中提取所有 Fragment（完全符合 SR 生成逻辑）
    pub fn extract_all_fragments(text: &str) -> Vec<Fragment> {
        let mut fragments = Vec::new();
        let lines: Vec<&str> = text.lines().collect();
        
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i];
            
            if let Some(caps) = FRAGMENT_REGEX.captures(line.trim()) {
                let id = caps.get(1).unwrap().as_str().to_string();
                let start_idx = i;
                let base_indent = Self::get_indent(line);
                
                // 查找下一个 Fragment 或文件结束
                let mut end_idx = lines.len();
                for j in (i + 1)..lines.len() {
                    let next_indent = Self::get_indent(lines[j]);
                    if next_indent <= base_indent && FRAGMENT_REGEX.is_match(lines[j].trim()) {
                        end_idx = j;
                        break;
                    }
                }
                
                let fragment_text = lines[start_idx..end_idx].join("\n");
                
                // 解析 Fragment
                if let Ok(fragment) = Self::parse_fragment(&fragment_text, &id) {
                    fragments.push(fragment);
                }
                
                i = end_idx;
            } else {
                i += 1;
            }
        }
        
        fragments
    }
    
    /// 解析 Fragment 中的所有 Pipeline
    fn parse_pipelines(text: &str) -> ParseResult<Vec<Pipeline>> {
        let mut pipelines = Vec::new();
        let lines: Vec<&str> = text.lines().collect();
        
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i];
            
            if let Some(caps) = PIPELINE_REGEX.captures(line.trim()) {
                let id = caps.get(1).unwrap().as_str().to_string();
                let start_idx = i;
                let base_indent = Self::get_indent(line);
                
                // 查找 Pipeline 结束位置
                let mut end_idx = lines.len();
                for j in (i + 1)..lines.len() {
                    let next_indent = Self::get_indent(lines[j]);
                    if next_indent <= base_indent && 
                       (PIPELINE_REGEX.is_match(lines[j].trim()) || 
                        FRAGMENT_REGEX.is_match(lines[j].trim())) {
                        end_idx = j;
                        break;
                    }
                }
                
                let pipeline_text = lines[start_idx..end_idx].join("\n");
                let pipeline = Self::parse_single_pipeline(&pipeline_text, &id)?;
                pipelines.push(pipeline);
                i = end_idx;
            } else {
                i += 1;
            }
        }
        
        Ok(pipelines)
    }
    
    /// 解析单个 Pipeline
    fn parse_single_pipeline(text: &str, id: &str) -> ParseResult<Pipeline> {
        let metrics = Self::extract_pipeline_metrics(text);
        let operators = Self::extract_operators(text);
        
        Ok(Pipeline {
            id: id.to_string(),
            metrics,
            operators,
        })
    }
    
    /// 提取 Pipeline 的指标
    fn extract_pipeline_metrics(text: &str) -> HashMap<String, String> {
        let mut metrics = HashMap::new();
        
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("- ") && trimmed.contains(": ") {
                let rest = trimmed.trim_start_matches("- ");
                let parts: Vec<&str> = rest.splitn(2, ": ").collect();
                if parts.len() == 2 {
                    metrics.insert(parts[0].trim().to_string(), parts[1].trim().to_string());
                }
            }
        }
        
        metrics
    }
    
    /// 提取 Pipeline 中的所有 Operator
    fn extract_operators(text: &str) -> Vec<Operator> {
        use crate::parser::core::operator_parser::OperatorParser;
        use crate::parser::core::MetricsParser;
        
        let mut operators = Vec::new();
        let lines: Vec<&str> = text.lines().collect();
        let mut i = 0;
        
        while i < lines.len() {
            let trimmed = lines[i].trim();
            
            if OperatorParser::is_operator_header(trimmed) {
                // 找到 operator header
                let full_header = trimmed.trim_end_matches(':').to_string();
                
                // 提取纯粹的操作符类型名称（去掉plan_node_id部分）
                let operator_name = if let Some(pos) = full_header.find(" (plan_node_id=") {
                    full_header[..pos].to_string()
                } else {
                    full_header.clone()
                };
                
                let base_indent = Self::get_indent(lines[i]);
                
                // 收集这个 operator 的所有内容（直到遇到下一个同级或更高级的内容）
                let mut operator_lines = vec![lines[i]];
                i += 1;
                
                while i < lines.len() {
                    let line = lines[i];
                    if line.trim().is_empty() {
                        i += 1;
                        continue;
                    }
                    
                    let current_indent = Self::get_indent(line);
                    
                    // 如果缩进小于等于 base_indent，说明遇到了同级或更高级的内容
                    if current_indent <= base_indent {
                        break;
                    }
                    
                    operator_lines.push(line);
                    i += 1;
                }
                
                // 解析这个 operator 的完整文本
                let operator_text = operator_lines.join("\n");
                
                // 提取 plan_node_id (从 full_header 中解析，如 "CONNECTOR_SCAN (plan_node_id=0)")
                let plan_node_id = if full_header.contains("plan_node_id=") {
                    full_header
                        .split("plan_node_id=")
                        .nth(1)
                        .and_then(|s| s.trim_end_matches(')').parse::<i32>().ok())
                        .map(|n| n.to_string())
                } else {
                    None
                };
                
                // 直接解析原始文本为HashMap，保留所有__MAX_OF_和__MIN_OF_指标
                // 这对于多backend的情况至关重要
                let common_metrics_text = MetricsParser::extract_common_metrics_block(&operator_text);
                let unique_metrics_text = MetricsParser::extract_unique_metrics_block(&operator_text);
                
                let common_metrics = Self::parse_metrics_to_hashmap(&common_metrics_text);
                let unique_metrics = Self::parse_metrics_to_hashmap(&unique_metrics_text);
                
                operators.push(Operator {
                    name: operator_name,
                    plan_node_id,
                    operator_id: None,
                    common_metrics,
                    unique_metrics,
                    children: Vec::new(),
                });
            } else {
                i += 1;
            }
        }
        
        operators
    }
    
    /// 将 metrics 文本解析为 HashMap
    fn parse_metrics_to_hashmap(text: &str) -> HashMap<String, String> {
        let mut metrics = HashMap::new();
        
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("- ") {
                let rest = trimmed.trim_start_matches("- ");
                // 只跳过 __MIN_OF_ 指标，保留 __MAX_OF_ 指标用于覆盖基础值
                if rest.starts_with("__MIN_OF_") {
                    continue;
                }
                
                if let Some(colon_pos) = rest.find(": ") {
                    let key = rest[..colon_pos].trim().to_string();
                    let value = rest[colon_pos + 2..].trim().to_string();
                    metrics.insert(key, value);
                } else if !rest.is_empty() {
                    // 没有值的指标（如 IsSubordinate）
                    metrics.insert(rest.to_string(), "true".to_string());
                }
            }
        }
        
        metrics
    }
    
    /// 提取 Backend 地址
    fn extract_backend_addresses(text: &str) -> Vec<String> {
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("- BackendAddresses:") {
                let addresses = trimmed.trim_start_matches("- BackendAddresses:").trim();
                return addresses.split(',').map(|s| s.trim().to_string()).collect();
            }
        }
        Vec::new()
    }
    
    /// 提取 Instance ID
    fn extract_instance_ids(text: &str) -> Vec<String> {
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("- InstanceIds:") {
                let ids = trimmed.trim_start_matches("- InstanceIds:").trim();
                return ids.split(',').map(|s| s.trim().to_string()).collect();
            }
        }
        Vec::new()
    }
    
    fn get_indent(line: &str) -> usize {
        line.chars().take_while(|c| c.is_whitespace()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extract_backend_addresses() {
        let text = "   - BackendAddresses: 192.168.1.1:9060, 192.168.1.2:9060";
        let addrs = FragmentParser::extract_backend_addresses(text);
        assert_eq!(addrs.len(), 2);
        assert_eq!(addrs[0], "192.168.1.1:9060");
    }
}

