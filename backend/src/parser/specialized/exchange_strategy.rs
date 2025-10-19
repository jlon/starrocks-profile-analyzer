//! # ExchangeStrategy - Exchange Operator 专用指标解析策略

use crate::models::{ExchangeSinkSpecializedMetrics, OperatorSpecializedMetrics};
use super::strategy::SpecializedMetricsStrategy;
use crate::parser::core::value_parser::ValueParser;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ExchangeSinkStrategy;

#[derive(Debug, Clone)]
pub struct ExchangeSourceStrategy;

impl SpecializedMetricsStrategy for ExchangeSinkStrategy {
    fn parse(&self, text: &str) -> OperatorSpecializedMetrics {
        OperatorSpecializedMetrics::ExchangeSink(Self::parse_exchange_sink(text))
    }
}

impl ExchangeSinkStrategy {
    fn parse_exchange_sink(text: &str) -> ExchangeSinkSpecializedMetrics {
        let mut part_type = String::from("UNPARTITIONED");
        let mut bytes_sent: Option<u64> = None;
        let mut bytes_pass_through: Option<u64> = None;
        let mut request_sent: Option<u64> = None;
        let mut network_time: Option<Duration> = None;
        let mut overall_time: Option<Duration> = None;
        let mut dest_fragment_ids = Vec::new();
        let dest_be_addresses = Vec::new();
        
        for line in text.lines() {
            let trimmed = line.trim();
            if let Some((key, value)) = Self::parse_kv_line(trimmed) {
                match key {
                    "PartType" => part_type = value.to_string(),
                    "BytesSent" => bytes_sent = ValueParser::parse_bytes(value).ok(),
                    "BytesPassThrough" => bytes_pass_through = ValueParser::parse_bytes(value).ok(),
                    "RequestSent" => request_sent = ValueParser::parse_number(value).ok(),
                    "NetworkTime" => network_time = ValueParser::parse_duration(value).ok(),
                    "OverallTime" => overall_time = ValueParser::parse_duration(value).ok(),
                    "DestFragments" => {
                        dest_fragment_ids = value.split(',').map(|s| s.trim().to_string()).collect();
                    }
                    _ => {}
                }
            }
        }
        
        ExchangeSinkSpecializedMetrics {
            dest_fragment_ids,
            dest_be_addresses,
            part_type,
            bytes_sent,
            bytes_pass_through,
            request_sent,
            network_time,
            overall_time,
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

impl SpecializedMetricsStrategy for ExchangeSourceStrategy {
    fn parse(&self, _text: &str) -> OperatorSpecializedMetrics {
        // ExchangeSource 目前使用通用指标即可
        // 如需特殊处理，可以在此实现
        OperatorSpecializedMetrics::None
    }
}

