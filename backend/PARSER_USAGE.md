# StarRocks Profile Parser 使用指南

> 基于 StarRocks Profile 格式规范的通用解析器
> 
> 参考: `PROFILE_FORMAT_SPEC.md`

## 📦 解析器架构

### 模块组织

```
parser/
├── error.rs                    # 统一错误处理
├── value_parser.rs             # 值解析器（时间、字节、数字）
├── topology_parser.rs          # Topology JSON 解析器
├── operator_parser.rs          # Operator 头部解析器
├── metrics_parser.rs           # 通用指标解析器
├── fragment_parser.rs          # Fragment 解析器
├── tree_builder.rs             # 执行树构建器
├── hotspot_detector.rs         # 热点检测器
├── composer.rs                 # 解析器组合器（主入口）
└── specialized_metrics/        # 专用指标解析策略
    ├── mod.rs                  # 策略注册
    ├── strategy.rs             # 策略 trait
    ├── scan_strategy.rs        # Scan 类型策略
    ├── exchange_strategy.rs    # Exchange 策略
    ├── result_sink_strategy.rs # ResultSink 策略
    ├── join_strategy.rs        # Join 策略
    └── aggregate_strategy.rs   # Aggregate 策略
```

## 🚀 快速开始

### 基本用法

```rust
use starrocks_profile_analyzer::parser::ProfileComposer;

// 读取 Profile 文本
let profile_text = std::fs::read_to_string("profile.txt")?;

// 创建解析器
let composer = ProfileComposer::new();

// 解析 Profile
let profile = composer.parse(&profile_text)?;

// 访问解析结果
println!("Query ID: {}", profile.summary.query_id);
println!("Total Time: {}", profile.summary.total_time);
println!("Query State: {}", profile.summary.query_state);

// 执行树
if let Some(tree) = profile.execution_tree {
    println!("Root Operator: {}", tree.root.operator_name);
    println!("Total Nodes: {}", tree.nodes.len());
}

// 性能瓶颈
if let Some(bottlenecks) = profile.bottlenecks {
    for bn in bottlenecks {
        println!("Hotspot: {} - {:.1}% time", bn.operator_name, bn.time_percentage);
    }
}
```

### 自定义热点检测配置

```rust
use starrocks_profile_analyzer::parser::{ProfileComposer, HotspotConfig};

let hotspot_config = HotspotConfig {
    moderate_threshold: 20.0,   // 20% 为轻度热点
    severe_threshold: 50.0,     // 50% 为中度热点
    critical_threshold: 80.0,   // 80% 为严重热点
};

let composer = ProfileComposer::new().with_hotspot_config(hotspot_config);
let profile = composer.parse(&profile_text)?;
```

## 🔧 模块详解

### 1. ValueParser - 值解析器

**职责**: 解析 Profile 中的各种值类型

**支持的格式**:
- 时间: `1h30m`, `7s854ms`, `5.540us`, `390ns`
- 字节: `2.167 KB`, `12.768 GB`, `2.174K (2174)`
- 数字: `334`, `2.174K (2174)`, `1,234,567`
- 百分比: `85.5%`, `12.34`

**使用示例**:

```rust
use starrocks_profile_analyzer::parser::ValueParser;

// 解析时间
let duration = ValueParser::parse_duration("7s854ms")?;
println!("Milliseconds: {}", duration.as_millis());

let ms = ValueParser::parse_time_to_ms("1h30m")?;
println!("Total ms: {}", ms);

// 解析字节
let bytes = ValueParser::parse_bytes("2.167 KB")?;
println!("Bytes: {}", bytes);

// 优先使用括号内的原始值
let bytes = ValueParser::parse_bytes("2.174K (2174)")?;
assert_eq!(bytes, 2174);

// 解析数字
let num: u64 = ValueParser::parse_number("1,234,567")?;
let num2: u64 = ValueParser::parse_number("2.174K (2174)")?;
assert_eq!(num2, 2174);

// 解析百分比
let pct = ValueParser::parse_percentage("85.5%")?;
```

### 2. TopologyParser - 拓扑解析器

**职责**: 解析 Execution 章节中的 Topology JSON

**Topology 结构**:
```json
{
  "rootId": 1,
  "nodes": [
    {
      "id": 1,
      "name": "EXCHANGE",
      "properties": {"sinkIds": [], "displayMem": true},
      "children": [0]
    },
    {
      "id": 0,
      "name": "OLAP_SCAN",
      "properties": {"sinkIds": [1], "displayMem": false},
      "children": []
    }
  ]
}
```

**使用示例**:

```rust
use starrocks_profile_analyzer::parser::TopologyParser;

// 解析 Topology JSON
let topology = TopologyParser::parse(json_str)?;

// 验证拓扑图
TopologyParser::validate(&topology)?;

// 构建关系映射
let relationships = TopologyParser::build_relationships(&topology);

// 获取叶子节点
let leaves = TopologyParser::get_leaf_nodes(&topology);
```

### 3. OperatorParser - 操作符解析器

**职责**: 解析 Operator 头部信息

**Operator 头部格式**:
```
CONNECTOR_SCAN (plan_node_id=0):
HASH_JOIN (plan_node_id=1) (operator id=2):
RESULT_SINK (plan_node_id=-1):
```

**使用示例**:

```rust
use starrocks_profile_analyzer::parser::OperatorParser;

// 解析 Operator 头部
let header = OperatorParser::parse_header("CONNECTOR_SCAN (plan_node_id=0):")?;
println!("Name: {}", header.name);
println!("Plan Node ID: {}", header.plan_node_id);

// 提取 Operator 块
let block = OperatorParser::extract_operator_block(profile_text, "CONNECTOR_SCAN", Some(0));

// 确定节点类型
let node_type = OperatorParser::determine_node_type("CONNECTOR_SCAN");

// 标准化名称
let normalized = OperatorParser::normalize_name("es_scan");
assert_eq!(normalized, "CONNECTOR_SCAN");
```

### 4. MetricsParser - 指标解析器

**职责**: 解析 Operator 的 CommonMetrics

**CommonMetrics 结构**:
```
CommonMetrics:
   - OperatorTotalTime: 7s854ms
   - PullChunkNum: 1
   - PullRowNum: 1
```

**使用示例**:

```rust
use starrocks_profile_analyzer::parser::MetricsParser;

// 解析通用指标
let metrics = MetricsParser::parse_common_metrics(operator_text);

println!("Operator Total Time: {:?}", metrics.operator_total_time);
println!("Pull Chunk Num: {:?}", metrics.pull_chunk_num);

// 提取 UniqueMetrics 块
let unique_block = MetricsParser::extract_unique_metrics_block(operator_text);
```

### 5. SpecializedMetricsParser - 专用指标解析器

**职责**: 使用策略模式解析不同 Operator 的专用指标

**支持的 Operator 类型**:
- `CONNECTOR_SCAN` / `OLAP_SCAN` -> `ScanStrategy`
- `EXCHANGE_SINK` / `EXCHANGE_SOURCE` -> `ExchangeSinkStrategy`
- `RESULT_SINK` -> `ResultSinkStrategy`
- `HASH_JOIN` -> `JoinStrategy`
- `AGGREGATE` -> `AggregateStrategy`

**使用示例**:

```rust
use starrocks_profile_analyzer::parser::specialized_metrics::SpecializedMetricsParser;

let parser = SpecializedMetricsParser::new();

// 解析专用指标
let specialized = parser.parse("CONNECTOR_SCAN", unique_metrics_text);

match specialized {
    OperatorSpecializedMetrics::ConnectorScan(metrics) => {
        println!("Table: {}", metrics.table);
        println!("Bytes Read: {:?}", metrics.bytes_read);
    },
    OperatorSpecializedMetrics::OlapScan(metrics) => {
        println!("Scan Time: {:?}", metrics.scan_time);
    },
    _ => {}
}
```

### 6. TreeBuilder - 执行树构建器

**职责**: 根据 Topology 或 Fragment 信息构建执行树

**使用示例**:

```rust
use starrocks_profile_analyzer::parser::TreeBuilder;

// 从 Topology 构建执行树
let tree = TreeBuilder::build_from_topology(&topology, nodes)?;

// 验证树的有效性
TreeBuilder::validate(&tree)?;

// 链接 Exchange 连接
TreeBuilder::link_exchange_operators(&mut nodes);
```

### 7. HotspotDetector - 热点检测器

**职责**: 分析执行树，识别性能热点和瓶颈

**使用示例**:

```rust
use starrocks_profile_analyzer::parser::{HotspotDetector, HotspotConfig};

let config = HotspotConfig::default();

// 检测热点
HotspotDetector::detect(&mut nodes, config.clone());

// 查找瓶颈
let bottlenecks = HotspotDetector::find_bottlenecks(&tree, &config);

for bn in bottlenecks {
    println!("Bottleneck: {} ({:.1}%)", bn.operator_name, bn.time_percentage);
    println!("Severity: {:?}", bn.severity);
    println!("Reason: {}", bn.reason);
}
```

### 8. ProfileComposer - 解析器组合器

**职责**: 协调所有解析器，完成完整的 Profile 解析

**解析流程**:
1. 提取 Summary（Query、Planner、Execution）
2. 解析 Topology
3. 解析 Fragments 和 Operators
4. 解析指标（CommonMetrics + UniqueMetrics）
5. 构建执行树
6. 检测热点
7. 组装最终的 Profile 数据模型

**使用示例**:

```rust
use starrocks_profile_analyzer::parser::ProfileComposer;

let composer = ProfileComposer::new();
let profile = composer.parse(&profile_text)?;

// Profile 包含完整的解析结果
println!("Summary: {:?}", profile.summary);
println!("Planner Info: {:?}", profile.planner_info);
println!("Execution Info: {:?}", profile.execution_info);

if let Some(tree) = profile.execution_tree {
    // 遍历执行树
    for node in &tree.nodes {
        println!("Operator: {}", node.operator_name);
        if node.is_hotspot {
            println!("  -> HOT SPOT! Severity: {:?}", node.hotspot_severity);
        }
    }
}
```

## 🧪 测试

### 运行单元测试

```bash
cd backend
cargo test --package starrocks-profile-analyzer --lib parser
```

### 测试特定模块

```bash
# 测试值解析器
cargo test --package starrocks-profile-analyzer --lib parser::value_parser

# 测试拓扑解析器
cargo test --package starrocks-profile-analyzer --lib parser::topology_parser
```

### 集成测试

```bash
# 使用真实的 Profile 文件测试
cargo test --package starrocks-profile-analyzer --test integration_test
```

## 📊 性能优化

### 正则表达式缓存

所有正则表达式使用 `once_cell::Lazy` 预编译，避免重复编译：

```rust
static OPERATOR_HEADER_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^([A-Z_]+)\s*\(plan_node_id=(-?\d+)").unwrap()
});
```

### 策略模式

使用策略模式避免大量的 `match` 语句，提高可维护性和性能：

```rust
// 注册策略
let mut strategies: HashMap<String, Box<dyn SpecializedMetricsStrategy>> = HashMap::new();
strategies.insert("OLAP_SCAN".to_string(), Box::new(ScanStrategy));
strategies.insert("HASH_JOIN".to_string(), Box::new(JoinStrategy));

// 动态分发
if let Some(strategy) = strategies.get(operator_name) {
    let metrics = strategy.parse(text);
}
```

## 🔍 错误处理

所有解析器使用统一的错误类型 `ParseError`：

```rust
pub enum ParseError {
    InvalidFormat(String),
    SectionNotFound(String),
    ValueError { value: String, reason: String },
    TopologyError(String),
    OperatorError(String),
    TreeError(String),
    // ...
}
```

**错误处理示例**:

```rust
use starrocks_profile_analyzer::parser::{ProfileComposer, ParseError};

match composer.parse(&profile_text) {
    Ok(profile) => {
        println!("Parse success!");
    },
    Err(ParseError::InvalidFormat(msg)) => {
        eprintln!("Invalid profile format: {}", msg);
    },
    Err(ParseError::TopologyError(msg)) => {
        eprintln!("Topology parsing error: {}", msg);
    },
    Err(e) => {
        eprintln!("Parse error: {}", e);
    }
}
```

## 📚 扩展解析器

### 添加新的 Operator 策略

1. 在 `models.rs` 中定义专用指标结构：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyOperatorSpecializedMetrics {
    pub custom_field: String,
    pub custom_time: Option<Duration>,
}
```

2. 更新 `OperatorSpecializedMetrics` 枚举：

```rust
pub enum OperatorSpecializedMetrics {
    // ...
    MyOperator(MyOperatorSpecializedMetrics),
}
```

3. 实现策略：

```rust
// specialized_metrics/my_operator_strategy.rs
use crate::parser::specialized_metrics::strategy::SpecializedMetricsStrategy;

pub struct MyOperatorStrategy;

impl SpecializedMetricsStrategy for MyOperatorStrategy {
    fn parse(&self, text: &str) -> OperatorSpecializedMetrics {
        // 实现解析逻辑
        OperatorSpecializedMetrics::MyOperator(MyOperatorSpecializedMetrics {
            custom_field: "...".to_string(),
            custom_time: None,
        })
    }
}
```

4. 注册策略：

```rust
// specialized_metrics/mod.rs
impl SpecializedMetricsParser {
    pub fn new() -> Self {
        let mut strategies = HashMap::new();
        // ...
        strategies.insert("MY_OPERATOR".to_string(), Box::new(MyOperatorStrategy));
        Self { strategies }
    }
}
```

## 🎯 最佳实践

1. **优先使用组合器**: 使用 `ProfileComposer` 而不是直接调用子解析器
2. **错误处理**: 总是处理 `ParseResult` 的错误情况
3. **值提取**: 使用 `ValueParser` 而不是手动解析字符串
4. **类型安全**: 利用 Rust 的类型系统确保解析正确性
5. **性能**: 对于大量 Profile，考虑使用并行解析

## 📖 参考文档

- [Profile 格式规范](./PROFILE_FORMAT_SPEC.md)
- [重构指南](./REFACTOR_GUIDE.md)
- [StarRocks 官方文档](https://docs.starrocks.io/)

---

**版本**: v1.0  
**更新日期**: 2025-10-17

