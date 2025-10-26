
use crate::models::{AggregateSpecializedMetrics, OperatorSpecializedMetrics};
use super::strategy::SpecializedMetricsStrategy;

#[derive(Debug, Clone)]
pub struct AggregateStrategy;

impl SpecializedMetricsStrategy for AggregateStrategy {
    fn parse(&self, text: &str) -> OperatorSpecializedMetrics {
        OperatorSpecializedMetrics::Aggregate(Self::parse_aggregate(text))
    }
}

impl AggregateStrategy {
    fn parse_aggregate(text: &str) -> AggregateSpecializedMetrics {
        let mut agg_mode = String::from("NORMAL");
        
        for line in text.lines() {
            let trimmed = line.trim();
            if let Some((key, value)) = Self::parse_kv_line(trimmed) {
                if key == "AggMode" {
                    agg_mode = value.to_string();
                }
            }
        }
        
        AggregateSpecializedMetrics {
            agg_mode,
            chunk_by_chunk: false,
            input_rows: None,
            agg_function_time: None,
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

