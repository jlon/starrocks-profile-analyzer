
use crate::models::{JoinSpecializedMetrics, OperatorSpecializedMetrics};
use super::strategy::SpecializedMetricsStrategy;
use crate::parser::core::ValueParser;

#[derive(Debug, Clone)]
pub struct JoinStrategy;

impl SpecializedMetricsStrategy for JoinStrategy {
    fn parse(&self, text: &str) -> OperatorSpecializedMetrics {
        OperatorSpecializedMetrics::Join(Self::parse_join(text))
    }
}

impl JoinStrategy {
    fn parse_join(text: &str) -> JoinSpecializedMetrics {
        let mut join_type = String::from("INNER");
        let mut build_rows: Option<u64> = None;
        let mut probe_rows: Option<u64> = None;
        let mut runtime_filter_num: Option<u64> = None;
        let mut runtime_filter_evaluate: Option<u64> = None;
        
        for line in text.lines() {
            let trimmed = line.trim();
            if let Some((key, value)) = Self::parse_kv_line(trimmed) {
                match key {
                    "JoinType" => join_type = value.to_string(),
                    "BuildRows" => build_rows = ValueParser::parse_number(value).ok(),
                    "ProbeRows" => probe_rows = ValueParser::parse_number(value).ok(),
                    "RuntimeFilterNum" => runtime_filter_num = ValueParser::parse_number(value).ok(),
                    "JoinRuntimeFilterEvaluate" => {
                        runtime_filter_evaluate = ValueParser::parse_number(value).ok();
                    }
                    _ => {}
                }
            }
        }
        
        JoinSpecializedMetrics {
            join_type,
            build_rows,
            probe_rows,
            runtime_filter_num,
            runtime_filter_evaluate,
        }
    }
    
    fn parse_kv_line(line: &str) -> Option<(&str, &str)> {
        if !line.starts_with('-') {
            return None;
        }
        let rest = line.trim_start_matches('-').trim();
        let parts: Vec<&str> = rest.splitn(2, ':').collect();
        if parts.len() == 2 {
            Some((parts[0].trim(), parts[1].trim()))
        } else {
            None
        }
    }
}

