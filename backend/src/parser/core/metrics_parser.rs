//! # MetricsParser - 指标解析器
//! 
//! 负责解析 Operator 的通用指标（CommonMetrics）。
//! 
//! ## CommonMetrics 结构
//! ```text
//! CONNECTOR_SCAN (plan_node_id=0):
//!   CommonMetrics:
//!      - OperatorTotalTime: 7s854ms
//!      - PullChunkNum: 1
//!      - PullRowNum: 1
//!      - PushChunkNum: 0
//!      - PushRowNum: 0
//! ```

use crate::models::OperatorMetrics;
use super::value_parser::ValueParser;
use once_cell::sync::Lazy;
use regex::Regex;

static METRIC_LINE_REGEX: Lazy<Regex> = Lazy::new(|| {
    // 修复：
    // 1. 允许下划线开头: [A-Za-z_] - 支持 __MAX_OF_, __MIN_OF_ 等
    // 2. 冒号和值都是可选的: (?::\s+(.+))? - 支持 "IsFinalSink" 等标志
    Regex::new(r"^\s*-\s+([A-Za-z_][A-Za-z0-9_]*)(?::\s+(.+))?$").unwrap()
});

pub struct MetricsParser;

impl MetricsParser {
    /// 解析通用指标块
    /// 
    /// 从 Operator 文本中解析通用指标，包括 CommonMetrics 块和 __MAX_OF_* 指标。
    /// 
    /// # Arguments
    /// * `text` - Operator 的完整文本块
    /// 
    /// # Returns
    /// * `OperatorMetrics` - 解析后的通用指标
    pub fn parse_common_metrics(text: &str) -> OperatorMetrics {
        // 修复：解析完整的操作符块，确保 __MAX_OF_* 指标能覆盖基础指标
        // 这样可以正确处理 OperatorTotalTime 和 __MAX_OF_OperatorTotalTime 的覆盖关系
        Self::parse_metrics_from_text(text)
    }
    
    /// 从 HashMap 转换为 OperatorMetrics
    /// 
    /// # Arguments
    /// * `map` - 指标键值对的 HashMap
    /// 
    /// # Returns
    /// * `OperatorMetrics` - 解析后的通用指标
    pub fn from_hashmap(map: &std::collections::HashMap<String, String>) -> OperatorMetrics {
        let mut metrics = OperatorMetrics::default();
        
        for (key, value) in map {
            Self::set_metric_value(&mut metrics, key, value);
        }
        
        metrics
    }
    
    /// 从文本中解析所有指标
    /// 
    /// 遍历所有行，识别并解析指标键值对。
    pub fn parse_metrics_from_text(text: &str) -> OperatorMetrics {
        let mut metrics = OperatorMetrics::default();
        
        for line in text.lines() {
            if let Some((key, value)) = Self::parse_metric_line(line) {
                Self::set_metric_value(&mut metrics, &key, &value);
            }
        }
        
        metrics
    }
    
    /// 解析单行指标
    /// 
    /// # Examples
    /// ```
    /// use starrocks_profile_analyzer::parser::MetricsParser;
    /// let (key, value) = MetricsParser::parse_metric_line("  - OperatorTotalTime: 7s854ms").unwrap();
    /// assert_eq!(key, "OperatorTotalTime");
    /// assert_eq!(value, "7s854ms");
    /// ```
    pub fn parse_metric_line(line: &str) -> Option<(String, String)> {
        // 只跳过 __MIN_OF_ 行，保留 __MAX_OF_ 行用于覆盖基础值
        if line.contains("__MIN_OF_") {
            return None;
        }
        
        METRIC_LINE_REGEX.captures(line).and_then(|caps| {
            let key = caps.get(1)?.as_str().trim().to_string();
            let value = caps.get(2)?.as_str().trim().to_string();
            Some((key, value))
        })
    }
    
    /// 提取 CommonMetrics 块
    /// 
    /// 从 Operator 文本中定位并提取 CommonMetrics 章节。
    pub fn extract_common_metrics_block(text: &str) -> String {
        Self::extract_section_block(text, "CommonMetrics:")
    }
    
    /// 提取 UniqueMetrics 块
    /// 
    /// 从 Operator 文本中定位并提取 UniqueMetrics 章节。
    pub fn extract_unique_metrics_block(text: &str) -> String {
        Self::extract_section_block(text, "UniqueMetrics:")
    }
    
    /// 检查文本中是否包含指定章节
    pub fn has_section(text: &str, section_name: &str) -> bool {
        text.contains(section_name)
    }
    
    // ========== Private Helper Methods ==========
    
    fn extract_section_block(text: &str, section_marker: &str) -> String {
        if let Some(start) = text.find(section_marker) {
            let after_marker = &text[start + section_marker.len()..];
            let lines: Vec<&str> = after_marker.lines().collect();
            
            if lines.is_empty() {
                return String::new();
            }
            
            // 确定起始行的基础缩进
            let base_indent = lines.iter()
                .find(|l| !l.trim().is_empty())
                .map(|l| Self::get_indent(l))
                .unwrap_or(0);
            
            let mut block_lines = Vec::new();
            
            for line in lines {
                let trimmed = line.trim();
                
                // 遇到下一个 Metrics 章节，结束
                if trimmed.ends_with("Metrics:") && trimmed != section_marker.trim_end_matches(':') {
                    break;
                }
                
                // 遇到同级或更高级别的非空行且不是指标行，结束
                if !trimmed.is_empty() {
                    let line_indent = Self::get_indent(line);
                    if line_indent <= base_indent && !trimmed.starts_with('-') {
                        // 检查是否是新的 section
                        if trimmed.ends_with(':') || Self::is_operator_line(trimmed) {
                            break;
                        }
                    }
                }
                
                block_lines.push(line);
            }
            
            block_lines.join("\n")
        } else {
            String::new()
        }
    }
    
    fn set_metric_value(metrics: &mut OperatorMetrics, key: &str, value: &str) {
        match key {
            "OperatorTotalTime" => {
                // 解析为纳秒 (u64) 以保持微秒精度
                if let Ok(duration) = ValueParser::parse_duration(value) {
                    metrics.operator_total_time = Some(duration.as_nanos() as u64);
                }
            }
            // 只有在基础值不存在时才使用 MAX 值
            "__MAX_OF_OperatorTotalTime" | "CPUTime" => {
                // 优先使用__MAX_OF_OperatorTotalTime，覆盖基础值
                if let Ok(duration) = ValueParser::parse_duration(value) {
                    metrics.operator_total_time = Some(duration.as_nanos() as u64);
                }
            }
            "PushChunkNum" => {
                metrics.push_chunk_num = Self::extract_number(value);
            }
            "PushRowNum" => {
                metrics.push_row_num = Self::extract_number(value);
            }
            "PullChunkNum" => {
                metrics.pull_chunk_num = Self::extract_number(value);
            }
            "PullRowNum" => {
                metrics.pull_row_num = Self::extract_number(value);
            }
            "PushTotalTime" => {
                if let Ok(duration) = ValueParser::parse_duration(value) {
                    metrics.push_total_time = Some(duration.as_nanos() as u64);
                }
            }
            "__MAX_OF_PushTotalTime" => {
                if let Ok(duration) = ValueParser::parse_duration(value) {
                    metrics.push_total_time = Some(duration.as_nanos() as u64);
                }
            }
            "PullTotalTime" => {
                if let Ok(duration) = ValueParser::parse_duration(value) {
                    metrics.pull_total_time = Some(duration.as_nanos() as u64);
                }
            }
            "__MAX_OF_PullTotalTime" => {
                if let Ok(duration) = ValueParser::parse_duration(value) {
                    metrics.pull_total_time = Some(duration.as_nanos() as u64);
                }
            }
            "MemoryUsage" => {
                metrics.memory_usage = ValueParser::parse_bytes(value).ok();
            }
            "OutputChunkBytes" => {
                metrics.output_chunk_bytes = ValueParser::parse_bytes(value).ok();
            }
            _ => {
                // 未识别的指标，忽略（可能在 specialized metrics 中）
            }
        }
    }
    
    fn extract_number(value: &str) -> Option<u64> {
        ValueParser::parse_number(value).ok()
    }
    
    fn get_indent(line: &str) -> usize {
        line.chars().take_while(|c| c.is_whitespace()).count()
    }
    
    fn is_operator_line(line: &str) -> bool {
        line.contains("(plan_node_id")
    }
}

// impl Default moved to models.rs to avoid duplication

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_metric_line() {
        let (key, value) = MetricsParser::parse_metric_line("  - OperatorTotalTime: 7s854ms").unwrap();
        assert_eq!(key, "OperatorTotalTime");
        assert_eq!(value, "7s854ms");
        
        let (key2, value2) = MetricsParser::parse_metric_line("     - PullChunkNum: 1").unwrap();
        assert_eq!(key2, "PullChunkNum");
        assert_eq!(value2, "1");
    }
    
    #[test]
    fn test_skip_min_max() {
        // 跳过 MIN 行
        assert!(MetricsParser::parse_metric_line("  - __MIN_OF_OperatorTotalTime: 1ms").is_none());
        // 保留 MAX 行用于覆盖基础值
        assert!(MetricsParser::parse_metric_line("  - __MAX_OF_PullRowNum: 100").is_some());
    }
    
    #[test]
    fn test_parse_common_metrics() {
        let text = r#"
CommonMetrics:
   - OperatorTotalTime: 7s854ms
   - PullChunkNum: 1
   - PullRowNum: 100
   - PushChunkNum: 1
   - PushRowNum: 100
"#;
        
        let metrics = MetricsParser::parse_common_metrics(text);
        assert!(metrics.operator_total_time.is_some());
        assert_eq!(metrics.pull_chunk_num, Some(1));
        assert_eq!(metrics.pull_row_num, Some(100));
    }
}

