//! 
//! 

//! 
//! 
//! 

use crate::parser::error::{ParseError, ParseResult};
use crate::models::OperatorMetrics;
use once_cell::sync::Lazy;
use regex::Regex;
use std::time::Duration;


static TIME_COMPONENT_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(\d+(?:\.\d+)?)\s*(ms|us|μs|ns|h|m|s)").unwrap()
});

static BYTES_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^([\d,.]+)\s*(TB|GB|MB|KB|K|M|G|T|B)$").unwrap()
});

static NUMBER_WITH_PAREN_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[\d,.]+[KMGB]?\s*\((\d+)\)").unwrap()
});


static NUMBER_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^([\d,.]+)").unwrap()
});

pub struct ValueParser;

impl ValueParser {
    
    /// 
    /// 

    /// 
    /// ```
    /// 
    /// ```
    pub fn parse_duration(input: &str) -> ParseResult<Duration> {
        let input = input.trim();
        
        if input == "0" {
            return Ok(Duration::from_nanos(0));
        }
        
        let mut total_ns: f64 = 0.0;
        let mut found_any = false;
        

        for cap in TIME_COMPONENT_REGEX.captures_iter(input) {
            found_any = true;
            
            let num_str = cap.get(1).unwrap().as_str();
            let num: f64 = num_str.parse().map_err(|_| ParseError::ParseDurationError(
                format!("Invalid number '{}' in duration '{}'", num_str, input)
            ))?;
            
            let unit = cap.get(2).unwrap().as_str();
            

            let ns = match unit {
                "h" => num * 3600.0 * 1_000_000_000.0,
                "m" => num * 60.0 * 1_000_000_000.0,
                "s" => num * 1_000_000_000.0,
                "ms" => num * 1_000_000.0,
                "us" | "μs" => num * 1_000.0,
                "ns" => num,
                _ => 0.0,
            };
            
            total_ns += ns;
        }
        
        if !found_any {
            return Err(ParseError::ParseDurationError(
                format!("No valid time components found in '{}'", input)
            ));
        }
        
        Ok(Duration::from_nanos(total_ns as u64))
    }
    
    /// 

    pub fn parse_time_to_ms(input: &str) -> ParseResult<f64> {
        let duration = Self::parse_duration(input)?;
        Ok(duration.as_nanos() as f64 / 1_000_000.0)
    }
    
    
    /// 
    /// 

    /// 
    /// ```
    /// 
    /// ```
    pub fn parse_bytes(input: &str) -> ParseResult<u64> {
        let original = input.trim();
        let input = original.to_uppercase();
        

        if let Some(cap) = NUMBER_WITH_PAREN_REGEX.captures(&input) {
            let raw = cap.get(1).unwrap().as_str();
            return raw.parse::<u64>().map_err(|e| ParseError::ParseBytesError(
                format!("Failed to parse raw bytes '{}': {}", raw, e)
            ));
        }
        

        if let Some(cap) = BYTES_REGEX.captures(&input) {
            let num_str = cap.get(1).unwrap().as_str().replace(",", "");
            let num: f64 = num_str.parse().map_err(|e| ParseError::ParseBytesError(
                format!("Invalid number '{}': {}", num_str, e)
            ))?;
            
            let unit = cap.get(2).unwrap().as_str();
            
            let multiplier: f64 = match unit {
                "B" => 1.0,
                "K" | "KB" => 1024.0,
                "M" | "MB" => 1024.0 * 1024.0,
                "G" | "GB" => 1024.0 * 1024.0 * 1024.0,
                "T" | "TB" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
                _ => return Err(ParseError::ParseBytesError(
                    format!("Unknown byte unit: {}", unit)
                )),
            };
            
            return Ok((num * multiplier).floor() as u64);
        }
        

        let temp = input.replace(",", "");
        let cleaned = temp.split_whitespace().next().unwrap_or(&input);
        cleaned.parse::<u64>().map_err(|e| ParseError::ParseBytesError(
            format!("Cannot parse bytes from '{}': {}", input, e)
        ))
    }
    
    

    /// 

    /// 
    /// ```
    /// 
    /// ```
    pub fn parse_number<T>(input: &str) -> ParseResult<T>
    where
        T: std::str::FromStr,
        <T as std::str::FromStr>::Err: std::fmt::Display,
    {
        let input = input.trim();
        
        if let Some(cap) = NUMBER_WITH_PAREN_REGEX.captures(input) {
            let raw = cap.get(1).unwrap().as_str();
            return raw.parse::<T>().map_err(|e| ParseError::ParseNumberError(
                format!("Failed to parse number from parentheses '{}': {}", raw, e)
            ));
        }
        
        if let Some(cap) = NUMBER_REGEX.captures(input) {
            let num_str = cap.get(1).unwrap().as_str().replace(",", "");
            return num_str.parse::<T>().map_err(|e| ParseError::ParseNumberError(
                format!("Failed to parse number '{}': {}", num_str, e)
            ));
        }
        
        Err(ParseError::ParseNumberError(
            format!("Cannot extract number from '{}'", input)
        ))
    }
    
    
    /// 
    /// ```
    /// 
    /// ```
    pub fn parse_percentage(input: &str) -> ParseResult<f64> {
        let input = input.trim().trim_end_matches('%');
        input.parse::<f64>().map_err(|e| ParseError::ParseNumberError(
            format!("Failed to parse percentage '{}': {}", input, e)
        ))
    }
    
    

    /// 
    pub fn parse_bool(input: &str) -> ParseResult<bool> {
        match input.trim().to_lowercase().as_str() {
            "true" | "yes" | "1" => Ok(true),
            "false" | "no" | "0" => Ok(false),
            _ => Err(ParseError::ValueError {
                value: input.to_string(),
                reason: "Invalid boolean value".to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    
    #[test]
    fn test_parse_duration_hours_minutes() {
        let d = ValueParser::parse_duration("1h30m").unwrap();
        assert_eq!(d.as_secs(), 5400);
    }
    
    #[test]
    fn test_parse_duration_seconds_millis() {
        let d = ValueParser::parse_duration("7s854ms").unwrap();
        assert_eq!(d.as_nanos(), 7_854_000_000);
    }
    
    #[test]
    fn test_parse_duration_millis() {
        let d = ValueParser::parse_duration("123ms").unwrap();
        assert_eq!(d.as_nanos(), 123_000_000);
        
        let d = ValueParser::parse_duration("123.456ms").unwrap();
        assert_eq!(d.as_nanos(), 123_456_000);
    }
    
    #[test]
    fn test_parse_duration_micros() {
        let d = ValueParser::parse_duration("5.540us").unwrap();
        assert_eq!(d.as_nanos(), 5540);
    }
    
    #[test]
    fn test_parse_duration_nanos() {
        let d = ValueParser::parse_duration("390ns").unwrap();
        assert_eq!(d.as_nanos(), 390);
    }
    
    #[test]
    fn test_parse_duration_zero() {
        let d = ValueParser::parse_duration("0ns").unwrap();
        assert_eq!(d.as_nanos(), 0);
    }
    
    #[test]
    fn test_parse_duration_zero_without_unit() {
        let d = ValueParser::parse_duration("0").unwrap();
        assert_eq!(d.as_nanos(), 0);
    }
    
    
    #[test]
    fn test_parse_bytes_with_unit() {
        assert_eq!(ValueParser::parse_bytes("2.167KB").unwrap(), 2219);
        assert_eq!(ValueParser::parse_bytes("12.768GB").unwrap(), 13709535608);
        assert_eq!(ValueParser::parse_bytes("0.000B").unwrap(), 0);
    }
    
    #[test]
    fn test_parse_bytes_with_parentheses() {
        assert_eq!(ValueParser::parse_bytes("2.174K (2174)").unwrap(), 2174);
        assert_eq!(ValueParser::parse_bytes("1.234M (1234567)").unwrap(), 1234567);
    }
    
    #[test]
    fn test_parse_bytes_plain_number() {
        assert_eq!(ValueParser::parse_bytes("1024").unwrap(), 1024);
    }
    
    
    #[test]
    fn test_parse_number_with_parentheses() {
        let n: u64 = ValueParser::parse_number("2.174K (2174)").unwrap();
        assert_eq!(n, 2174);
    }
    
    #[test]
    fn test_parse_number_with_commas() {
        let n: u64 = ValueParser::parse_number("1,234,567").unwrap();
        assert_eq!(n, 1234567);
    }
    
    #[test]
    fn test_parse_number_plain() {
        let n: u64 = ValueParser::parse_number("334").unwrap();
        assert_eq!(n, 334);
    }
    
    #[test]
    fn test_parse_number_float() {
        let n: f64 = ValueParser::parse_number("12.34").unwrap();
        assert!((n - 12.34).abs() < 0.001);
    }
    
    
    #[test]
    fn test_parse_percentage() {
        assert!((ValueParser::parse_percentage("85.5%").unwrap() - 85.5).abs() < 0.001);
        assert!((ValueParser::parse_percentage("12.34").unwrap() - 12.34).abs() < 0.001);
    }
}


static METRIC_LINE_REGEX: Lazy<Regex> = Lazy::new(|| {

    Regex::new(r"^\s*-\s+([A-Za-z_][A-Za-z0-9_]*)(?::\s+(.+))?$").unwrap()
});

pub struct MetricsParser;

impl MetricsParser {

    /// 
    /// 
    /// 
    pub fn parse_common_metrics(text: &str) -> OperatorMetrics {
        Self::parse_metrics_from_text(text)
    }
    
    /// 
    /// 
    pub fn from_hashmap(map: &std::collections::HashMap<String, String>) -> OperatorMetrics {
        let mut metrics = OperatorMetrics::default();
        
        for (key, value) in map {
            Self::set_metric_value(&mut metrics, key, value);
        }
        
        metrics
    }
    

    /// 

    pub fn parse_metrics_from_text(text: &str) -> OperatorMetrics {
        let mut metrics = OperatorMetrics::default();
        
        for line in text.lines() {
            if let Some((key, value)) = Self::parse_metric_line(line) {
                Self::set_metric_value(&mut metrics, &key, &value);
            }
        }
        
        metrics
    }
    

    /// 
    /// ```
    /// ```
    pub fn parse_metric_line(line: &str) -> Option<(String, String)> {
        if line.contains("__MIN_OF_") {
            return None;
        }
        
        METRIC_LINE_REGEX.captures(line).and_then(|caps| {
            let key = caps.get(1)?.as_str().trim().to_string();
            let value = caps.get(2)?.as_str().trim().to_string();
            Some((key, value))
        })
    }
    
    /// 
    pub fn extract_common_metrics_block(text: &str) -> String {
        Self::extract_section_block(text, "CommonMetrics:")
    }
    
    /// 
    pub fn extract_unique_metrics_block(text: &str) -> String {
        Self::extract_section_block(text, "UniqueMetrics:")
    }
    

    pub fn has_section(text: &str, section_name: &str) -> bool {
        text.contains(section_name)
    }
    
    
    fn extract_section_block(text: &str, section_marker: &str) -> String {
        if let Some(start) = text.find(section_marker) {
            let after_marker = &text[start + section_marker.len()..];
            let lines: Vec<&str> = after_marker.lines().collect();
            
            if lines.is_empty() {
                return String::new();
            }
            

            let base_indent = lines.iter()
                .find(|l| !l.trim().is_empty())
                .map(|l| Self::get_indent(l))
                .unwrap_or(0);
            
            let mut block_lines = Vec::new();
            
            for line in lines {
                let trimmed = line.trim();
                
                if trimmed.ends_with("Metrics:") && trimmed != section_marker.trim_end_matches(':') {
                    break;
                }
                

                if !trimmed.is_empty() {
                    let line_indent = Self::get_indent(line);
                    if line_indent <= base_indent && !trimmed.starts_with('-') {
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
                if let Ok(duration) = ValueParser::parse_duration(value) {
                    metrics.operator_total_time = Some(duration.as_nanos() as u64);
                }
            }
            "__MAX_OF_OperatorTotalTime" | "CPUTime" => {
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
        assert!(MetricsParser::parse_metric_line("  - __MIN_OF_OperatorTotalTime: 1ms").is_none());
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

