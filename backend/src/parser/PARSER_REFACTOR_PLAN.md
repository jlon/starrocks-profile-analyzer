# StarRocks Profile 解析器重构方案

## 当前代码审查

### 存在的问题

1. **单文件过长**: `advanced_parser.rs` 超过 1200 行，违反了单一职责原则
2. **职责混乱**: 一个结构体承担了太多职责（解析、构建树、检测热点等）
3. **重复代码**: 多处出现相似的解析逻辑（时间、字节、数字等）
4. **硬编码**: 大量硬编码的字符串匹配和正则表达式
5. **可扩展性差**: 添加新的 Operator 类型需要修改多处代码
6. **测试困难**: 函数耦合度高，单元测试困难
7. **错误处理不完善**: 很多地方使用 `unwrap_or()` 静默处理错误
8. **没有抽象层**: 缺少通用的解析接口和特质

### 代码优点

1. 功能完整：支持 Topology JSON 和 fallback 解析
2. 指标解析全面：涵盖了多种 Operator 的特定指标
3. 深度计算和热点检测逻辑合理

## 十个通用解析器设计

基于单一职责原则和领域驱动设计，将解析功能拆分为 10 个模块化解析器：

### 1. ValueParser（值解析器）
**职责**: 解析各种类型的值（数字、时间、字节、百分比等）
```rust
pub struct ValueParser;

impl ValueParser {
    fn parse_duration(s: &str) -> Result<Duration, ParseError>
    fn parse_bytes(s: &str) -> Result<u64, ParseError>
    fn parse_number(s: &str) -> Result<u64, ParseError>
    fn parse_percentage(s: &str) -> Result<f64, ParseError>
    fn parse_time_string_to_ms(s: &str) -> Result<u64, ParseError>
}
```

### 2. SectionParser（章节解析器）
**职责**: 识别和提取 Profile 的各个章节（Summary, Planner, Execution, Fragment）
```rust
pub struct SectionParser;

impl SectionParser {
    fn parse_summary_section(text: &str) -> Result<ProfileSummary, ParseError>
    fn parse_planner_section(text: &str) -> Result<PlannerInfo, ParseError>
    fn parse_execution_section(text: &str) -> Result<ExecutionInfo, ParseError>
    fn extract_fragment_blocks(text: &str) -> Vec<&str>
}
```

### 3. TopologyParser（拓扑解析器）
**职责**: 解析 Topology JSON 并构建基础节点关系
```rust
pub struct TopologyParser;

impl TopologyParser {
    fn parse_topology_json(json_str: &str) -> Result<TopologyGraph, ParseError>
    fn build_node_relationships(topology: &TopologyGraph) -> Vec<NodeRelation>
    fn extract_root_id(topology: &TopologyGraph) -> Result<i32, ParseError>
}
```

### 4. OperatorParser（操作符解析器）
**职责**: 解析 Operator 名称和基本信息
```rust
pub struct OperatorParser;

impl OperatorParser {
    fn parse_operator_header(line: &str) -> Result<OperatorHeader, ParseError>
    fn extract_operator_block(text: &str, op_name: &str, plan_id: Option<i32>) -> String
    fn determine_operator_type(op_name: &str) -> NodeType
    fn normalize_operator_name(name: &str) -> String
}

pub struct OperatorHeader {
    pub name: String,
    pub plan_node_id: Option<i32>,
    pub operator_id: Option<String>,
}
```

### 5. MetricsParser（指标解析器）
**职责**: 解析通用的 Operator 指标（CommonMetrics）
```rust
pub struct MetricsParser;

impl MetricsParser {
    fn parse_common_metrics(text: &str) -> OperatorMetrics
    fn parse_metric_line(line: &str) -> Option<(String, String)>
    fn extract_common_metrics_block(text: &str) -> &str
    fn extract_unique_metrics_block(text: &str) -> &str
}
```

### 6. SpecializedMetricsParser（专用指标解析器）
**职责**: 使用策略模式解析不同 Operator 的专用指标
```rust
pub trait SpecializedMetricsStrategy {
    fn parse(&self, text: &str) -> OperatorSpecializedMetrics;
}

pub struct SpecializedMetricsParser {
    strategies: HashMap<String, Box<dyn SpecializedMetricsStrategy>>,
}

impl SpecializedMetricsParser {
    fn new() -> Self { /* 注册所有策略 */ }
    fn parse(& self, op_name: &str, text: &str) -> OperatorSpecializedMetrics
}

// 具体策略实现
pub struct ScanMetricsStrategy;
pub struct ExchangeMetricsStrategy;
pub struct JoinMetricsStrategy;
pub struct AggregateMetricsStrategy;
pub struct ResultSinkMetricsStrategy;
```

### 7. TreeBuilder（树构建器）
**职责**: 基于解析后的节点构建执行树
```rust
pub struct TreeBuilder;

impl TreeBuilder {
    fn build_from_topology(nodes: Vec<TreeNode>, root_id: i32) -> ExecutionTree
    fn build_from_fragments(fragments: &[Fragment]) -> ExecutionTree
    fn link_parent_child(nodes: &mut [TreeNode])
    fn calculate_depths(nodes: &mut [TreeNode])
    fn validate_tree(tree: &ExecutionTree) -> Result<(), TreeError>
}
```

### 8. FragmentParser（Fragment 解析器）
**职责**: 解析 Fragment 和 Pipeline 结构
```rust
pub struct FragmentParser;

impl FragmentParser {
    fn parse_fragment(text: &str, id: &str) -> Result<Fragment, ParseError>
    fn parse_pipelines(text: &str) -> Vec<Pipeline>
    fn extract_backend_addresses(text: &str) -> Vec<String>
    fn extract_instance_ids(text: &str) -> Vec<String>
}
```

### 9. HotspotDetector（热点检测器）
**职责**: 分析和标记性能热点
```rust
pub struct HotspotDetector;

pub struct HotspotConfig {
    pub moderate_threshold: f64,  // 20%
    pub severe_threshold: f64,    // 50%
    pub critical_threshold: f64,  // 80%
}

impl HotspotDetector {
    fn detect(nodes: &mut [TreeNode], config: HotspotConfig)
    fn calculate_severity(time: f64, max_time: f64) -> HotSeverity
    fn find_bottlenecks(tree: &ExecutionTree) -> Vec<Bottleneck>
}
```

### 10. ProfileComposer（Profile 组合器）
**职责**: 协调所有解析器，组装最终的 Profile 对象
```rust
pub struct ProfileComposer {
    value_parser: ValueParser,
    section_parser: SectionParser,
    topology_parser: TopologyParser,
    operator_parser: OperatorParser,
    metrics_parser: MetricsParser,
    specialized_parser: SpecializedMetricsParser,
    tree_builder: TreeBuilder,
    fragment_parser: FragmentParser,
    hotspot_detector: HotspotDetector,
}

impl ProfileComposer {
    pub fn parse(text: &str) -> Result<Profile, ParseError> {
        // 1. 解析章节
        let summary = self.section_parser.parse_summary_section(text)?;
        let planner = self.section_parser.parse_planner_section(text)?;
        let execution = self.section_parser.parse_execution_section(text)?;
        
        // 2. 构建执行树（优先使用 Topology）
        let tree = if let Ok(topology) = self.topology_parser.parse_topology_json(&execution.topology) {
            self.build_tree_from_topology(text, topology)?
        } else {
            self.build_tree_from_fragments(text)?
        };
        
        // 3. 检测热点
        let mut tree = tree;
        self.hotspot_detector.detect(&mut tree.nodes, HotspotConfig::default());
        
        // 4. 组装 Profile
        Ok(Profile {
            summary,
            planner,
            execution,
            fragments: vec![], // 从树中提取
            execution_tree: Some(tree),
        })
    }
}
```

## 模块目录结构

```
backend/src/parser/
├── mod.rs                          // 模块导出
├── error.rs                        // 错误类型定义
├── value_parser.rs                 // 1. 值解析器
├── section_parser.rs               // 2. 章节解析器
├── topology_parser.rs              // 3. 拓扑解析器
├── operator_parser.rs              // 4. 操作符解析器
├── metrics_parser.rs               // 5. 指标解析器
├── specialized_metrics/            // 6. 专用指标解析器（目录）
│   ├── mod.rs
│   ├── strategy.rs                 // 策略特质定义
│   ├── scan_strategy.rs            // Scan 指标解析策略
│   ├── exchange_strategy.rs        // Exchange 指标解析策略
│   ├── join_strategy.rs            // Join 指标解析策略
│   ├── aggregate_strategy.rs       // Aggregate 指标解析策略
│   └── result_sink_strategy.rs     // ResultSink 指标解析策略
├── tree_builder.rs                 // 7. 树构建器
├── fragment_parser.rs              // 8. Fragment 解析器
├── hotspot_detector.rs             // 9. 热点检测器
├── composer.rs                     // 10. Profile 组合器
├── legacy/                         // 旧代码（保留兼容）
│   ├── advanced_parser.rs
│   └── starrocks.rs
└── tests/                          // 单元测试
    ├── value_parser_tests.rs
    ├── topology_parser_tests.rs
    └── ...
```

## 重构路线图

### 阶段 1: 基础解析器（Week 1）
- [ ] 创建错误类型 `error.rs`
- [ ] 实现 ValueParser
- [ ] 实现 SectionParser
- [ ] 编写单元测试

### 阶段 2: 核心解析器（Week 2）
- [ ] 实现 TopologyParser
- [ ] 实现 OperatorParser
- [ ] 实现 MetricsParser
- [ ] 编写集成测试

### 阶段 3: 专用解析器（Week 3）
- [ ] 设计策略接口
- [ ] 实现 5 种 SpecializedMetricsStrategy
- [ ] 实现 SpecializedMetricsParser
- [ ] 编写策略测试

### 阶段 4: 树构建和检测（Week 4）
- [ ] 实现 TreeBuilder
- [ ] 实现 FragmentParser
- [ ] 实现 HotspotDetector
- [ ] 编写性能测试

### 阶段 5: 组合和迁移（Week 5）
- [ ] 实现 ProfileComposer
- [ ] 迁移现有代码到新架构
- [ ] 保留 legacy 代码用于兼容
- [ ] 完整回归测试

### 阶段 6: 优化和文档（Week 6）
- [ ] 性能优化（使用 `&str` 而非 `String`）
- [ ] 添加详细文档和示例
- [ ] 创建贡献指南
- [ ] 发布新版本

## 设计原则

1. **单一职责**: 每个解析器只负责一个明确的功能
2. **开闭原则**: 通过策略模式支持扩展新的 Operator 类型
3. **依赖倒置**: 依赖抽象接口而非具体实现
4. **接口隔离**: 小而专注的接口，不强制实现不需要的方法
5. **里氏替换**: 所有策略可互换使用
6. **组合优于继承**: 使用组合模式构建 ProfileComposer

## 性能考虑

1. **零拷贝**: 尽可能使用 `&str` 而非 `String::from()`
2. **懒加载**: 使用 `once_cell::Lazy` 缓存正则表达式
3. **并行解析**: 对独立的 Fragment 可以并行解析
4. **内存池**: 使用 Arena 分配器减少内存分配开销
5. **缓存**: 缓存常用的解析结果

## 错误处理

```rust
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Invalid profile format: {0}")]
    InvalidFormat(String),
    
    #[error("Section not found: {0}")]
    SectionNotFound(String),
    
    #[error("Failed to parse value: {0}")]
    ValueError(String),
    
    #[error("Invalid topology JSON: {0}")]
    TopologyError(String),
    
    #[error("Operator parse error: {0}")]
    OperatorError(String),
    
    #[error("Tree build error: {0}")]
    TreeError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
```

## 测试策略

1. **单元测试**: 每个解析器独立测试
2. **集成测试**: 测试解析器组合
3. **回归测试**: 使用真实 Profile 文件
4. **性能测试**: Benchmark 关键路径
5. **模糊测试**: 使用 cargo-fuzz 测试鲁棒性

## API 稳定性

- 新架构放在 `parser::v2` 模块
- 保留旧 API 在 `parser::legacy`
- 提供迁移指南
- 至少支持 2 个版本周期

---

**作者**: AI Assistant  
**日期**: 2025-10-17  
**版本**: v1.0

