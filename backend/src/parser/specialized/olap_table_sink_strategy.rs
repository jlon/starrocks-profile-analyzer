use crate::models::OperatorSpecializedMetrics;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct OlapTableSinkStrategy;

impl OlapTableSinkStrategy {
    pub fn parse(&self, text: &str) -> OperatorSpecializedMetrics {
        println!("DEBUG: OlapTableSinkStrategy.parse called with text length: {}", text.len());
        
        let mut metrics = HashMap::new();
        
        // 解析unique_metrics中的关键指标
        for line in text.lines() {
            if let Some((key, value)) = self.parse_kv_line(line) {
                // 只保留重要的时间指标，过滤掉__MAX_OF_和__MIN_OF_前缀的指标
                if !key.starts_with("__MAX_OF_") && !key.starts_with("__MIN_OF_") {
                    match key.as_str() {
                        "PrepareDataTime" | "RpcClientSideTime" | "RpcServerSideTime" | 
                        "SendDataTime" | "PackChunkTime" | "ConvertChunkTime" | 
                        "ValidateDataTime" | "SerializeChunkTime" | "SendRpcTime" |
                        "WaitResponseTime" | "CloseWaitTime" | "AllocAutoIncrementTime" |
                        "UpdateLoadChannelProfileTime" => {
                            metrics.insert(key, value);
                        }
                        _ => {
                            // 其他指标暂时不处理
                        }
                    }
                }
            }
        }
        
        println!("DEBUG: OlapTableSinkStrategy parsed {} metrics", metrics.len());
        
        OperatorSpecializedMetrics::None // 暂时返回None，因为我们还没有定义OlapTableSinkSpecializedMetrics
    }
    
    fn parse_kv_line(&self, line: &str) -> Option<(String, String)> {
        let trimmed = line.trim();
        if trimmed.starts_with("- ") {
            let content = &trimmed[2..];
            if let Some(colon_pos) = content.find(':') {
                let key = content[..colon_pos].trim().to_string();
                let value = content[colon_pos + 1..].trim().to_string();
                return Some((key, value));
            }
        }
        None
    }
}
