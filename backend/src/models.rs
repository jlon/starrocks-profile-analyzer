use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub summary: ProfileSummary,
    pub planner: PlannerInfo,
    pub execution: ExecutionInfo,
    pub fragments: Vec<Fragment>,
    pub execution_tree: Option<ExecutionTree>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileSummary {
    pub query_id: String,
    pub start_time: String, // 保持为字符串格式
    pub end_time: String,
    pub total_time: String,
    pub query_state: String,
    pub starrocks_version: String,
    pub sql_statement: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_db: Option<String>,
    pub variables: HashMap<String, String>,
    // Additional metrics for frontend display
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_allocated_memory: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_peak_memory: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub push_total_time: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pull_total_time: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_time_ms: Option<f64>, // Total time in milliseconds for calculations
    // QueryCumulativeOperatorTime: 用于计算operator时间百分比的分母
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_cumulative_operator_time: Option<String>, // 原始字符串格式
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_cumulative_operator_time_ms: Option<f64>, // 毫秒格式（支持小数），用于计算
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_execution_wall_time: Option<String>, // 原始字符串格式
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_execution_wall_time_ms: Option<f64>, // 毫秒格式（支持小数），用于百分比计算
    
    /// Top N最耗时的节点（对齐StarRocks官方逻辑）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_time_consuming_nodes: Option<Vec<TopNode>>,
}

/// Top N最耗时节点信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopNode {
    pub rank: u32,
    pub operator_name: String,
    pub plan_node_id: i32,
    pub total_time: String,
    pub time_percentage: f64,
    pub is_most_consuming: bool,
    pub is_second_most_consuming: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerInfo {
    pub details: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionInfo {
    pub topology: String,
    pub metrics: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fragment {
    pub id: String,
    pub backend_addresses: Vec<String>,
    pub instance_ids: Vec<String>,
    pub pipelines: Vec<Pipeline>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub id: String,
    pub metrics: HashMap<String, String>,
    pub operators: Vec<Operator>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionTree {
    pub root: ExecutionTreeNode,
    pub nodes: Vec<ExecutionTreeNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionTreeNode {
    pub id: String,
    pub operator_name: String,
    pub node_type: NodeType,
    pub plan_node_id: Option<i32>,
    pub parent_plan_node_id: Option<i32>,
    pub metrics: OperatorMetrics,
    pub children: Vec<String>, // 孩子的ID列表
    pub depth: usize,
    pub is_hotspot: bool,
    pub hotspot_severity: HotSeverity,
    // 新增：用于前端按 Fragment/Pipeline 归一化百分比
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fragment_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pipeline_id: Option<String>,
    // 新增：执行时间百分比
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_percentage: Option<f64>,
    
    /// 时间消耗超过30%的节点（红色高亮）
    #[serde(default)]
    pub is_most_consuming: bool,
    
    /// 时间消耗在15%-30%之间的节点（粉色/珊瑚色高亮）
    #[serde(default)]
    pub is_second_most_consuming: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum NodeType {
    OlapScan,
    ConnectorScan,
    HashJoin,
    Aggregate,
    Limit,
    ExchangeSink,
    ExchangeSource,
    ResultSink,
    ChunkAccumulate,
    Sort,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorMetrics {
    // 通用指标 (所有操作符都有的)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_total_time: Option<u64>, // nanoseconds (changed from milliseconds for precision)
    // 保留原始字符串格式，用于调试
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_total_time_raw: Option<String>,
    pub push_chunk_num: Option<u64>,
    pub push_row_num: Option<u64>,
    pub pull_chunk_num: Option<u64>,
    pub pull_row_num: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub push_total_time: Option<u64>, // nanoseconds (changed from milliseconds for precision)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pull_total_time: Option<u64>, // nanoseconds (changed from milliseconds for precision)

    // 内存相关
    pub memory_usage: Option<u64>,

    // 输出数据量
    pub output_chunk_bytes: Option<u64>,

    // 专用指标 (根据操作符类型决定)
    pub specialized: OperatorSpecializedMetrics,
}

impl Default for OperatorMetrics {
    fn default() -> Self {
        Self {
            operator_total_time: None,
            operator_total_time_raw: None,
            push_chunk_num: None,
            push_row_num: None,
            pull_chunk_num: None,
            pull_row_num: None,
            push_total_time: None,
            pull_total_time: None,
            memory_usage: None,
            output_chunk_bytes: None,
            specialized: OperatorSpecializedMetrics::None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperatorSpecializedMetrics {
    None,
    ConnectorScan(ConnectorScanSpecializedMetrics),
    OlapScan(OlapScanSpecializedMetrics),
    ExchangeSink(ExchangeSinkSpecializedMetrics),
    Join(JoinSpecializedMetrics),
    Aggregate(AggregateSpecializedMetrics),
    ResultSink(ResultSinkSpecializedMetrics),
    // 可以继续扩展其他操作符的专用指标
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorScanSpecializedMetrics {
    pub data_source_type: String,
    pub table: String,
    pub rollup: String,
    pub shared_scan: bool,
    pub morsel_queue_type: String,

    // IO统计
    pub io_time: Option<Duration>,
    pub io_task_exec_time: Option<Duration>,
    pub scan_time: Option<Duration>,
    pub bytes_read: Option<u64>,
    pub uncompressed_bytes_read: Option<u64>,
    pub rows_read: Option<u64>,
    pub raw_rows_read: Option<u64>,

    // IO统计细节
    pub compressed_bytes_read_local_disk: Option<u64>,
    pub compressed_bytes_read_remote: Option<u64>,
    pub compressed_bytes_read_request: Option<u64>,
    pub io_count_local_disk: Option<u64>,
    pub io_count_remote: Option<u64>,
    pub io_time_local_disk: Option<Duration>,
    pub io_time_remote: Option<Duration>,

    // 分段读取统计
    pub segment_init: Option<Duration>,
    pub segment_read: Option<Duration>,
    pub segment_read_count: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OlapScanSpecializedMetrics {
    pub table: String,
    pub rollup: String,
    pub shared_scan: bool,
    pub scan_time: Option<Duration>,
    pub io_time: Option<Duration>,
    pub bytes_read: Option<u64>,
    pub rows_read: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeSinkSpecializedMetrics {
    pub dest_fragment_ids: Vec<String>,
    pub dest_be_addresses: Vec<String>,
    pub part_type: String, // "UNPARTITIONED"
    pub bytes_sent: Option<u64>,
    pub bytes_pass_through: Option<u64>,
    pub request_sent: Option<u64>,
    pub network_time: Option<Duration>,
    pub overall_time: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinSpecializedMetrics {
    pub join_type: String,
    pub build_rows: Option<u64>,
    pub probe_rows: Option<u64>,
    pub runtime_filter_num: Option<u64>,
    pub runtime_filter_evaluate: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateSpecializedMetrics {
    pub agg_mode: String,
    pub chunk_by_chunk: bool,
    pub input_rows: Option<u64>,
    pub agg_function_time: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultSinkSpecializedMetrics {
    pub sink_type: String,
    pub operator_total_time: Option<Duration>,
    pub max_operator_total_time: Option<Duration>,
    pub append_chunk_time: Option<Duration>,
    pub result_rend_time: Option<Duration>,
    pub tuple_convert_time: Option<Duration>,
}

// 兼容旧接口的简化的Operator接口
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operator {
    pub name: String,
    pub plan_node_id: Option<String>,
    pub operator_id: Option<String>,
    pub common_metrics: HashMap<String, String>,
    pub unique_metrics: HashMap<String, String>,
    pub children: Vec<Operator>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HotSeverity {
    Normal,
    Mild,
    Moderate,
    Severe,
    Critical,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotSpot {
    pub node_path: String,
    pub severity: HotSeverity,
    pub issue_type: String,
    pub description: String,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub hotspots: Vec<HotSpot>,
    pub conclusion: String,
    pub suggestions: Vec<String>,
    pub performance_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileAnalysisResponse {
    pub hotspots: Vec<HotSpot>,
    pub conclusion: String,
    pub suggestions: Vec<String>,
    pub performance_score: f64,
    pub execution_tree: Option<ExecutionTree>,
    pub summary: Option<ProfileSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub fragment_id: String,
    pub pipeline_id: String,
    pub operator_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorSummary {
    pub operator_name: String,
    pub fragment_id: String,
    pub pipeline_id: String,
    pub total_time: Duration,
    pub time_percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorContext {
    pub operator_name: String,
    pub fragment_id: String,
    pub pipeline_id: String,
    pub metrics: HashMap<String, String>,
    pub execution_metrics: HashMap<String, String>,
}
