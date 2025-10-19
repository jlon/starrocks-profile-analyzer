//! # SectionParser - 章节解析器
//! 
//! 负责识别和提取 Profile 的各个章节：
//! - Summary: 查询摘要信息
//! - Planner: 计划器信息
//! - Execution: 执行信息
//! - Fragment: Fragment 块

use crate::models::{ProfileSummary, PlannerInfo, ExecutionInfo};
use crate::parser::error::{ParseError, ParseResult};
use super::value_parser::ValueParser;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;

static SUMMARY_LINE_REGEX: Lazy<Regex> = 
    Lazy::new(|| Regex::new(r"^\s*-\s+([^:]+):\s*(.*)$").unwrap());

pub struct SectionParser;

impl SectionParser {
    /// 提取 Summary 章节并解析
    ///
    /// # Profile Format
    /// ```text
    /// Query:
    ///   Summary:
    ///      - Query ID: xxx
    ///      - Start Time: xxx
    ///      - Total: 1h30m
    /// ```
    pub fn parse_summary(text: &str) -> ParseResult<ProfileSummary> {
        let summary_block = Self::extract_block(text, "Summary:")?;
        
        let mut fields = HashMap::new();
        for line in summary_block.lines() {
            if let Some(cap) = SUMMARY_LINE_REGEX.captures(line) {
                let key = cap.get(1).map(|m| m.as_str().trim()).unwrap_or("");
                let value = cap.get(2).map(|m| m.as_str().trim()).unwrap_or("");
                fields.insert(key.to_string(), value.to_string());
            }
        }
        
        Ok(ProfileSummary {
            query_id: fields.get("Query ID").cloned().unwrap_or_default(),
            start_time: fields.get("Start Time").cloned().unwrap_or_default(),
            end_time: fields.get("End Time").cloned().unwrap_or_default(),
            total_time: fields.get("Total").cloned().unwrap_or_default(),
            query_state: fields.get("Query State").cloned().unwrap_or_default(),
            starrocks_version: fields.get("StarRocks Version").cloned().unwrap_or_default(),
            sql_statement: fields.get("Sql Statement").cloned().unwrap_or_default(),
            query_type: fields.get("Query Type").cloned(),
            user: fields.get("User").cloned(),
            default_db: fields.get("Default Db").cloned(),
            variables: HashMap::new(), // 需要进一步解析
            query_allocated_memory: None,
            query_peak_memory: None,
            push_total_time: None,
            pull_total_time: None,
            total_time_ms: Self::parse_total_time_ms(&fields.get("Total").cloned().unwrap_or_default()),
            // 解析 QueryCumulativeOperatorTime 字段
            query_cumulative_operator_time: fields.get("QueryCumulativeOperatorTime").cloned(),
            query_cumulative_operator_time_ms: fields.get("QueryCumulativeOperatorTime")
                .and_then(|time_str| Self::parse_total_time_ms(time_str)),
        })
    }
    
    /// 提取 Planner 章节并解析
    pub fn parse_planner(text: &str) -> ParseResult<PlannerInfo> {
        let planner_block = Self::extract_block(text, "Planner:")?;
        let mut details = HashMap::new();
        
        for line in planner_block.lines() {
            if let Some(cap) = SUMMARY_LINE_REGEX.captures(line) {
                let key = cap.get(1).map(|m| m.as_str().trim()).unwrap_or("");
                let value = cap.get(2).map(|m| m.as_str().trim()).unwrap_or("");
                details.insert(key.to_string(), value.to_string());
            }
        }
        
        Ok(PlannerInfo { details })
    }
    
    /// 提取 Execution 章节并解析
    pub fn parse_execution(text: &str) -> ParseResult<ExecutionInfo> {
        let execution_block = Self::extract_block(text, "Execution:")?;
        
        // 提取 Topology
        let topology = Self::extract_topology(&execution_block)?;
        
        // 提取其他指标
        let mut metrics = HashMap::new();
        for line in execution_block.lines() {
            if let Some(cap) = SUMMARY_LINE_REGEX.captures(line) {
                let key = cap.get(1).map(|m| m.as_str().trim()).unwrap_or("");
                let value = cap.get(2).map(|m| m.as_str().trim()).unwrap_or("");
                if !key.is_empty() && !value.is_empty() && key != "Topology" {
                    metrics.insert(key.to_string(), value.to_string());
                }
            }
        }
        
        Ok(ExecutionInfo { topology, metrics })
    }
    
    /// 提取 Fragment 块列表
    ///
    /// 返回每个 Fragment 的文本块及其 ID
    pub fn extract_fragments(text: &str) -> Vec<(String, String)> {
        let mut fragments = Vec::new();
        let lines: Vec<&str> = text.lines().collect();
        
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i].trim();
            
            // 匹配 "Fragment N:"
            if let Some(id) = Self::extract_fragment_id(line) {
                let start_idx = i;
                let base_indent = Self::get_indent(lines[i]);
                
                // 查找下一个 Fragment 或文件结束
                let mut end_idx = lines.len();
                for j in (i + 1)..lines.len() {
                    let next_indent = Self::get_indent(lines[j]);
                    if next_indent <= base_indent && Self::extract_fragment_id(lines[j].trim()).is_some() {
                        end_idx = j;
                        break;
                    }
                }
                
                let fragment_text = lines[start_idx..end_idx].join("\n");
                fragments.push((id, fragment_text));
                i = end_idx;
            } else {
                i += 1;
            }
        }
        
        fragments
    }
    
    // ========== Private Helper Methods ==========
    
    fn extract_block(text: &str, section_marker: &str) -> ParseResult<String> {
        if let Some(start) = text.find(section_marker) {
            // 找到section_marker所在行的缩进级别
            let before_marker = &text[..start];
            let marker_line_start = before_marker.rfind('\n').map(|pos| pos + 1).unwrap_or(0);
            let marker_line = &text[marker_line_start..start + section_marker.len()];
            let marker_indent = Self::get_indent(marker_line);
            
            let rest = &text[start + section_marker.len()..];
            let lines: Vec<&str> = rest.lines().collect();
            
            // 找到下一个同级或更高级别的章节
            let mut end_pos = rest.len();
            for (i, line) in lines.iter().enumerate().skip(1) {
                if !line.trim().is_empty() {
                    let curr_indent = Self::get_indent(line);
                    // 如果遇到同级或更高级别的章节标记（以冒号结尾），则结束
                    if curr_indent <= marker_indent && line.trim().ends_with(':') {
                        // 计算到当前行的字符位置
                        let mut char_count = 0;
                        for j in 0..i {
                            char_count += lines[j].len() + 1; // +1 for newline
                        }
                        end_pos = char_count;
                        break;
                    }
                }
            }
            
            Ok(rest[..end_pos].to_string())
        } else {
            Err(ParseError::SectionNotFound(section_marker.to_string()))
        }
    }
    
    fn extract_topology(text: &str) -> ParseResult<String> {
        // Topology 是一个 JSON，需要完整提取
        if let Some(start_pos) = text.find("- Topology:") {
            let after_label = &text[start_pos + 11..];
            if let Some(json_start) = after_label.find('{') {
                let json_part = &after_label[json_start..];
                
                // 简单的括号匹配来提取完整 JSON
                let mut depth = 0;
                let mut end_pos = 0;
                
                for (i, ch) in json_part.char_indices() {
                    match ch {
                        '{' => depth += 1,
                        '}' => {
                            depth -= 1;
                            if depth == 0 {
                                end_pos = i + 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                
                if end_pos > 0 {
                    return Ok(json_part[..end_pos].to_string());
                }
            }
        }
        
        Ok(String::new())
    }
    
    fn extract_fragment_id(line: &str) -> Option<String> {
        let re = Regex::new(r"^Fragment\s+(\d+):").ok()?;
        re.captures(line)?.get(1).map(|m| m.as_str().to_string())
    }
    
    fn get_indent(line: &str) -> usize {
        line.chars().take_while(|c| c.is_whitespace()).count()
    }
    
    fn parse_total_time_ms(time_str: &str) -> Option<f64> {
        ValueParser::parse_time_to_ms(time_str).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_summary() {
        let profile = r#"
Query:
  Summary:
     - Query ID: b1f9a935-a967-11f0-b3d8-f69e292b7593
     - Start Time: 2025-10-15 09:38:48
     - Total: 1h30m
     - Query State: Finished
"#;
        let summary = SectionParser::parse_summary(profile).unwrap();
        assert_eq!(summary.query_id, "b1f9a935-a967-11f0-b3d8-f69e292b7593");
        assert_eq!(summary.total_time, "1h30m");
    }
}

