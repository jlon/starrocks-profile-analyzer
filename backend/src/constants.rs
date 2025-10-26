/// Performance analysis constants

pub mod time_thresholds {
    pub const MOST_CONSUMING_THRESHOLD: f64 = 30.0;
    
    pub const SECOND_CONSUMING_THRESHOLD: f64 = 15.0;
    
    pub const METRIC_CONSUMING_THRESHOLD: f64 = 0.3;
    
    pub const MAX_REASONABLE_BASE_TIME_MS: f64 = 100_000.0;
}

pub mod top_n {
    pub const TOP_NODES_LIMIT: usize = 3;
}

pub mod file_limits {
    pub const MAX_UPLOAD_SIZE: u64 = 50 * 1024 * 1024;
}

pub mod starrocks {
    pub const MERGED_INFO_PREFIX_MAX: &str = "__MAX_OF_";
    
    pub const MERGED_INFO_PREFIX_MIN: &str = "__MIN_OF_";
    
    pub const FINAL_SINK_PSEUDO_PLAN_NODE_ID: i32 = -1;
}

pub mod performance {
    pub const NODE_INFO_CACHE_CAPACITY: usize = 1000;
    
    pub const METRIC_CACHE_CAPACITY: usize = 5000;
}
