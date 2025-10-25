/// 性能分析常量定义
/// 
/// 这个模块定义了所有用于性能分析的常量，避免魔法数字

/// 时间消耗阈值
pub mod time_thresholds {
    /// 严重时间消耗阈值（红色高亮）: >30%
    pub const MOST_CONSUMING_THRESHOLD: f64 = 30.0;
    
    /// 中等时间消耗阈值（珊瑚色高亮）: >15%
    pub const SECOND_CONSUMING_THRESHOLD: f64 = 15.0;
    
    /// 指标级别时间消耗阈值: >30%
    pub const METRIC_CONSUMING_THRESHOLD: f64 = 0.3;
    
    /// 最大合理基准时间（毫秒）: 100秒
    pub const MAX_REASONABLE_BASE_TIME_MS: f64 = 100_000.0;
}

/// Top N配置
pub mod top_n {
    /// Top Most Time-consuming Nodes数量
    pub const TOP_NODES_LIMIT: usize = 3;
}

/// 文件大小限制
pub mod file_limits {
    /// 文件上传最大大小: 50MB
    pub const MAX_UPLOAD_SIZE: u64 = 50 * 1024 * 1024;
}

/// StarRocks特定常量
pub mod starrocks {
    /// MERGED_INFO_PREFIX_MAX: "__MAX_OF_"
    pub const MERGED_INFO_PREFIX_MAX: &str = "__MAX_OF_";
    
    /// MERGED_INFO_PREFIX_MIN: "__MIN_OF_"
    pub const MERGED_INFO_PREFIX_MIN: &str = "__MIN_OF_";
    
    /// FINAL_SINK_PSEUDO_PLAN_NODE_ID
    pub const FINAL_SINK_PSEUDO_PLAN_NODE_ID: i32 = -1;
}

/// 性能优化配置
pub mod performance {
    /// NodeInfo缓存容量
    pub const NODE_INFO_CACHE_CAPACITY: usize = 1000;
    
    /// 指标聚合缓存容量
    pub const METRIC_CACHE_CAPACITY: usize = 5000;
}

