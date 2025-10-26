//! 

use crate::models::{
    OperatorSpecializedMetrics, OlapScanSpecializedMetrics, ConnectorScanSpecializedMetrics,
};
use super::strategy::SpecializedMetricsStrategy;
use crate::parser::core::ValueParser;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ScanStrategy;

impl SpecializedMetricsStrategy for ScanStrategy {
    fn parse(&self, text: &str) -> OperatorSpecializedMetrics {
        let is_connector_scan = text.contains("CONNECTOR_SCAN") || text.contains("DataSourceType");
        
        if is_connector_scan {
            OperatorSpecializedMetrics::ConnectorScan(Self::parse_connector_scan(text))
        } else {
            OperatorSpecializedMetrics::OlapScan(Self::parse_olap_scan(text))
        }
    }
}

impl ScanStrategy {
    fn parse_olap_scan(text: &str) -> OlapScanSpecializedMetrics {
        let mut table = String::new();
        let mut rollup = String::new();
        let mut scan_time: Option<Duration> = None;
        let mut io_time: Option<Duration> = None;
        let mut bytes_read: Option<u64> = None;
        let mut rows_read: Option<u64> = None;
        
        for line in text.lines() {
            let trimmed = line.trim();
            if let Some((key, value)) = Self::parse_kv_line(trimmed) {
                match key {
                    "Table" => table = value.to_string(),
                    "Rollup" => rollup = value.to_string(),
                    "ScanTime" => scan_time = ValueParser::parse_duration(value).ok(),
                    "IOTime" => io_time = ValueParser::parse_duration(value).ok(),
                    "BytesRead" => bytes_read = ValueParser::parse_bytes(value).ok(),
                    "RowsRead" => rows_read = ValueParser::parse_number(value).ok(),
                    _ => {}
                }
            }
        }
        
        OlapScanSpecializedMetrics {
            table,
            rollup,
            shared_scan: false,
            scan_time,
            io_time,
            bytes_read,
            rows_read,
        }
    }
    
    fn parse_connector_scan(text: &str) -> ConnectorScanSpecializedMetrics {
        let mut data_source_type = String::new();
        let mut table = String::new();
        let mut rollup = String::new();
        let mut morsel_queue_type = String::new();
        let mut io_time: Option<Duration> = None;
        let mut io_task_exec_time: Option<Duration> = None;
        let mut scan_time: Option<Duration> = None;
        let mut bytes_read: Option<u64> = None;
        let mut uncompressed_bytes_read: Option<u64> = None;
        let mut rows_read: Option<u64> = None;
        let mut raw_rows_read: Option<u64> = None;
        let mut compressed_bytes_read_local_disk: Option<u64> = None;
        let mut compressed_bytes_read_remote: Option<u64> = None;
        let mut compressed_bytes_read_request: Option<u64> = None;
        let mut io_count_local_disk: Option<u64> = None;
        let mut io_count_remote: Option<u64> = None;
        let mut io_time_local_disk: Option<Duration> = None;
        let mut io_time_remote: Option<Duration> = None;
        let mut segment_init: Option<Duration> = None;
        let mut segment_read: Option<Duration> = None;
        let mut segment_read_count: Option<u64> = None;
        
        let mut _in_io_statistics = false;
        let mut in_io_task_exec = false;
        
        for line in text.lines() {
            let trimmed = line.trim();
            
            if trimmed == "IOStatistics:" || trimmed.starts_with("- IOStatistics:") {
                _in_io_statistics = true;
                in_io_task_exec = false;
                continue;
            } else if trimmed == "IOTaskExecTime:" || trimmed.starts_with("- IOTaskExecTime:") {
                in_io_task_exec = true;
                _in_io_statistics = false;
                continue;
            } else if trimmed.ends_with("Metrics:") {
                _in_io_statistics = false;
                in_io_task_exec = false;
            }
            
            if let Some((key, value)) = Self::parse_kv_line(trimmed) {
                match key {
                    "DataSourceType" => data_source_type = value.to_string(),
                    "Table" => table = value.to_string(),
                    "Rollup" => rollup = value.to_string(),
                    "MorselQueueType" => morsel_queue_type = value.to_string(),
                    "ScanTime" => scan_time = ValueParser::parse_duration(value).ok(),
                    "IOTime" => {
                        if in_io_task_exec {
                            io_task_exec_time = ValueParser::parse_duration(value).ok();
                        } else {
                            io_time = ValueParser::parse_duration(value).ok();
                        }
                    }
                    "BytesRead" => bytes_read = ValueParser::parse_bytes(value).ok(),
                    "UncompressedBytesRead" => uncompressed_bytes_read = ValueParser::parse_bytes(value).ok(),
                    "RowsRead" => rows_read = ValueParser::parse_number(value).ok(),
                    "RawRowsRead" => raw_rows_read = ValueParser::parse_number(value).ok(),
                    "CompressedBytesReadLocalDisk" => {
                        compressed_bytes_read_local_disk = ValueParser::parse_bytes(value).ok();
                    }
                    "CompressedBytesReadRemote" => {
                        compressed_bytes_read_remote = ValueParser::parse_bytes(value).ok();
                    }
                    "CompressedBytesReadRequest" => {
                        compressed_bytes_read_request = ValueParser::parse_bytes(value).ok();
                    }
                    "IOCountLocalDisk" => io_count_local_disk = ValueParser::parse_number(value).ok(),
                    "IOCountRemote" => io_count_remote = ValueParser::parse_number(value).ok(),
                    "IOTimeLocalDisk" => io_time_local_disk = ValueParser::parse_duration(value).ok(),
                    "IOTimeRemote" => io_time_remote = ValueParser::parse_duration(value).ok(),
                    "SegmentInit" => segment_init = ValueParser::parse_duration(value).ok(),
                    "SegmentRead" => segment_read = ValueParser::parse_duration(value).ok(),
                    "SegmentsReadCount" => segment_read_count = ValueParser::parse_number(value).ok(),
                    _ => {}
                }
            }
        }
        
        ConnectorScanSpecializedMetrics {
            data_source_type,
            table,
            rollup,
            shared_scan: false,
            morsel_queue_type,
            io_time,
            io_task_exec_time,
            scan_time,
            bytes_read,
            uncompressed_bytes_read,
            rows_read,
            raw_rows_read,
            compressed_bytes_read_local_disk,
            compressed_bytes_read_remote,
            compressed_bytes_read_request,
            io_count_local_disk,
            io_count_remote,
            io_time_local_disk,
            io_time_remote,
            segment_init,
            segment_read,
            segment_read_count,
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

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_olap_scan() {
        let text = r#"
OLAP_SCAN (plan_node_id=0):
  UniqueMetrics:
     - Table: test_table
     - Rollup: test_rollup
     - ScanTime: 1h30m
     - IOTime: 5s500ms
     - BytesRead: 2.167 KB
     - RowsRead: 1
"#;
        
        let strategy = ScanStrategy;
        if let OperatorSpecializedMetrics::OlapScan(metrics) = strategy.parse(text) {
            assert_eq!(metrics.table, "test_table");
            assert!(metrics.scan_time.is_some());
        } else {
            panic!("Expected OlapScan metrics");
        }
    }
}
