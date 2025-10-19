//! # ResultSinkStrategy - ResultSink Operator 专用指标解析策略

use crate::models::{OperatorSpecializedMetrics, ResultSinkSpecializedMetrics};
use super::strategy::SpecializedMetricsStrategy;
use crate::parser::core::value_parser::ValueParser;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ResultSinkStrategy;

impl SpecializedMetricsStrategy for ResultSinkStrategy {
    fn parse(&self, text: &str) -> OperatorSpecializedMetrics {
        OperatorSpecializedMetrics::ResultSink(Self::parse_result_sink(text))
    }
}

impl ResultSinkStrategy {
    fn parse_result_sink(text: &str) -> ResultSinkSpecializedMetrics {
        let mut sink_type = String::new();
        let mut operator_total_time: Option<Duration> = None;
        let mut max_operator_total_time: Option<Duration> = None;
        let mut append_chunk_time: Option<Duration> = None;
        let mut result_rend_time: Option<Duration> = None;
        let mut tuple_convert_time: Option<Duration> = None;
        
        let mut in_unique_metrics = false;
        
        for line in text.lines() {
            let trimmed = line.trim();
            
            if trimmed == "UniqueMetrics:" {
                in_unique_metrics = true;
                continue;
            } else if trimmed.ends_with("Metrics:") && trimmed != "UniqueMetrics:" {
                in_unique_metrics = false;
            }
            
            if let Some((key, value)) = Self::parse_kv_line(trimmed) {
                if key.starts_with("__MAX_OF_") {
                    if key == "__MAX_OF_OperatorTotalTime" {
                        max_operator_total_time = ValueParser::parse_duration(value).ok();
                    }
                    continue;
                }
                
                match key {
                    "SinkType" if in_unique_metrics => sink_type = value.to_string(),
                    "OperatorTotalTime" if !in_unique_metrics => {
                        operator_total_time = ValueParser::parse_duration(value).ok();
                    }
                    "AppendChunkTime" => append_chunk_time = ValueParser::parse_duration(value).ok(),
                    "ResultRendTime" => result_rend_time = ValueParser::parse_duration(value).ok(),
                    "TupleConvertTime" => tuple_convert_time = ValueParser::parse_duration(value).ok(),
                    _ => {}
                }
            }
        }
        
        ResultSinkSpecializedMetrics {
            sink_type,
            operator_total_time,
            max_operator_total_time,
            append_chunk_time,
            result_rend_time,
            tuple_convert_time,
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

