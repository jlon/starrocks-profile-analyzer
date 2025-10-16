# StarRocks Profile智能诊断分析器 - 系统设计文档

## 🎯 总体目标

构建一款专门用于StarRocks OLAP引擎查询Profile的智能分析工具，实现：

1. **准确定位性能瓶颈**：精准识别执行计划中的热点节点，支持多Fragment场景
2. **智能诊断建议**：基于官方tuning recipes的自动化诊断，覆盖Summary/Planner/Fragment/Operator四个层次
3. **可视化执行分析**：交互式DAG图展示执行计划，支持节点点击查看详情
4. **全方位性能洞察**：从概览统计到具体优化建议的完整分析体验

## 🏗️ 系统架构

### 后端架构 (Rust)

```
starrocks-profile-analyzer/
├── src/
│   ├── lib.rs                 # 主入口：analyze_profile()
│   ├── models.rs              # 数据模型定义
│   │   ├── Profile            # Profile原始数据结构
│   │   ├── ProfileAnalysisResponse # 分析结果
│   │   ├── HotSpot            # 热点问题定义
│   │   └── ExecutionTree      # 执行树结构
│   ├── parser/                # 解析模块
│   │   ├── advanced_parser.rs # 高级Profile解析器
│   │   ├── starrocks.rs       # StarRocks格式处理器
│   │   └── models.rs          # 解析专用模型
│   ├── analyzer/              # 分析引擎
│   │   ├── hotspot_detector.rs # 热点检测引擎
│   │   ├── suggestion_engine.rs # 智能建议引擎
│   │   └── rule_engine.rs     # 规则引擎 (待实现)
│   ├── api/                   # REST API
│   │   └── mod.rs            # Warp HTTP服务
│   └── main.rs               # 服务启动器
```

### 前端架构 (Vue.js 3 + Element Plus)

```
frontend/
├── src/
│   ├── main.js               # Vue应用入口
│   ├── App.vue               # 根组件
│   ├── router/               # 路由配置
│   │   └── index.js
│   ├── store/                # Vuex状态管理
│   │   └── index.js
│   ├── components/           # 组件
│   │   ├── DAGVisualization.vue       # DAG图可视化
│   │   ├── ExecutionPlanVisualization.vue # 执行计划
│   │   ├── HotSpotsPanel.vue          # 热点面板
│   │   ├── AnalysisSummary.vue        # 分析概览
│   │   ├── TreeNode.vue               # 树节点
│   │   └── FileUploader.vue           # 文件上传
│   ├── views/                # 页面视图
│   │   └── ProfileAnalyzer.vue # 主分析器页面
│   └── styles/               # 样式
└── package.json             # 依赖配置
```

## 🎯 智能诊断建议模块设计

### 1. 诊断数据模型

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotSpot {
    pub node_path: String,        // 节点路径：Fragment1.Pipeline0.CONNECTOR_SCAN
    pub severity: HotSeverity,    // 严重程度：Critical/High/Severe/Moderate/Mild
    pub issue_type: String,       // 问题类型：fragmented_rowsets/cold_storage/...
    pub description: String,      // 详细描述
    pub suggestions: Vec<String>, // 优化建议列表
    pub metrics: HashMap<String, String>, // 相关指标数据
    pub confidence_score: f64,    // 可信度分数（0.0-1.0）
}
```

### 2. 规则引擎设计

#### 规则配置数据结构

```rust
#[derive(Debug)]
pub struct TuningRule {
    pub id: String,
    pub name: String,
    pub target_operators: Vec<String>,      // 适用的操作符类型
    pub condition: DiagnosticCondition,     // 诊断条件
    pub priority: i32,                      // 规则优先级
    pub severity: HotSeverity,              // 诊断严重程度
    pub strategies: Vec<DiagnosticStrategy>, // 诊断策略
}

#[derive(Debug)]
pub enum DiagnosticCondition {
    MetricThreshold {               // 指标阈值条件
        metric_name: String,
        operator: ComparisonOperator,
        threshold: f64,
        unit: String,
    },
    MetricRatio {                  // 指标比例条件
        numerator: String,
        denominator: String,
        operator: ComparisonOperator,
        ratio_threshold: f64,
    },
    BooleanLogic {                 // 逻辑组合条件
        left: Box<DiagnosticCondition>,
        operator: LogicOperator,    // AND, OR, NOT
        right: Option<Box<DiagnosticCondition>>,
    },
    TimeBased {                    // 时间相关条件
        time_threshold: Duration,
        context_metric: String,
    },
}

#[derive(Debug)]
pub enum DiagnosticStrategy {
    FragmentationAnalysis,          // Segment碎片化分析
    MemoryPressureAnalysis,         // 内存压力分析
    IOEffficiencyAnalysis,          // IO效率分析
    NetworkBottleneckAnalysis,      // 网络瓶颈分析
    ConcurrentOperationAnalysis,    // 并发操作分析
    DataSkewDetection,              // 数据倾斜检测
    PresetSuggestions(Vec<String>), // 预设建议
    ExternalToolRecommendation,     // 外部工具推荐
}
```

#### 逐层诊断规则体系

## 1. SQL语句层面诊断规则

#### Query Profile Summary层规则
```rust
pub static SUMMARY_RULES: &[TuningRule] = &[
    TuningRule {
        id: "summary.collect_profile_time_high",
        name: "Profile收集时间过长检测",
        target_operators: vec!["ProfileSummary"],
        condition: DiagnosticCondition::MetricThreshold {
            metric_name: "CollectProfileTime",
            operator: ComparisonOperator::GreaterThan,
            threshold: 5000.0, // 5秒
            unit: "ms",
        },
        priority: 12,
        severity: HotSeverity::Moderate,
        strategies: vec![
            DiagnosticStrategy::PresetSuggestions(vec![
                "检查Profile收集配置是否过于详细".to_string(),
                "考虑是否需要调整_profile_enable_collect_hook参数".to_string(),
                "监控Profile收集对查询性能的影响".to_string(),
            ]),
        ],
    },
    TuningRule {
        id: "summary.execution_skew_fragment",
        name: "Fragment执行时间倾斜检测",
        target_operators: vec!["ProfileSummary"],
        condition: DiagnosticCondition::TimeBased {
            time_threshold: Duration::from_secs(10),
            context_metric: "FragmentExecutionTimeVariance",
        },
        priority: 15,
        severity: HotSeverity::Severe,
        strategies: vec![
            DiagnosticStrategy::DataSkewDetection,
        ],
    },
];
```

#### Planner层规则
```rust
pub static PLANNER_RULES: &[TuningRule] = &[
    TuningRule {
        id: "planner.coord_deliver_exec_abnormal",
        name: "Coordinator执行分发异常检测",
        target_operators: vec!["Planner"],
        condition: DiagnosticCondition::MetricThreshold {
            metric_name: "CoordDeliverExec",
            operator: ComparisonOperator::GreaterThan,
            threshold: 3000.0, // 3秒
            unit: "ms",
        },
        priority: 13,
        severity: HotSeverity::High,
        strategies: vec![
            DiagnosticStrategy::PresetSuggestions(vec![
                "检查Coordinator到BE节点网络连接".to_string(),
                "验证BE节点健康状态和响应性".to_string(),
                "调整query_delivery_timeout参数".to_string(),
            ]),
        ],
    },
];
```

## 2. 算子层面诊断规则

#### 通用算子规则
```rust
pub static OPERATOR_COMMON_RULES: &[TuningRule] = &[
    TuningRule {
        id: "operator.execution_time_high_ratio",
        name: "算子执行时间占比过高",
        target_operators: vec!["ALL"], // 适用于所有算子
        condition: DiagnosticCondition::MetricRatio {
            numerator: "OperatorWallTime",
            denominator: "QueryWallTime",
            operator: ComparisonOperator::GreaterThan,
            ratio_threshold: 0.6, // 占总时间的60%以上
        },
        priority: 20,
        severity: HotSeverity::Critical,
        strategies: vec![
            DiagnosticStrategy::PresetSuggestions(vec![
                "优化执行时间占比过高的算子".to_string(),
                "检查是否存在数据热点导致的执行倾斜".to_string(),
                "考虑调整并行度或分片策略".to_string(),
                "监控算子在多个实例上的执行时间分布".to_string(),
            ]),
        ],
    },
];
```

#### Scan算子规则集
```rust
pub static SCAN_OPERATOR_RULES: &[TuningRule] = &[
    TuningRule {
        id: "scan.data_skew",
        name: "数据倾斜",
        target_operators: vec!["OLAP_SCAN", "CONNECTOR_SCAN"],
        condition: DiagnosticCondition::BooleanLogic {
            left: Box::new(DiagnosticCondition::MetricThreshold {
                metric_name: "ScanDataSkewRatio",
                operator: ComparisonOperator::GreaterThan,
                threshold: 3.0, // 数据量差异3倍以上
                unit: "倍",
            }),
            operator: LogicOperator::And,
            right: Some(Box::new(DiagnosticCondition::MetricThreshold {
                metric_name: "IOVariance",
                operator: ComparisonOperator::GreaterThan,
                threshold: 5.0, // IO时间差异5倍以上
                unit: "倍",
            })),
        },
        priority: 16,
        severity: HotSeverity::Severe,
        strategies: vec![
            DiagnosticStrategy::PresetSuggestions(vec![
                "检查并优化分桶键设置，确保数据更均匀分布".to_string(),
                "调整表的分桶数量以分散数据".to_string(),
                "使用更合适的分桶策略避免热点".to_string(),
            ]),
        ],
    },

    TuningRule {
        id: "scan.io_skew",
        name: "IO倾斜",
        target_operators: vec!["OLAP_SCAN", "CONNECTOR_SCAN"],
        condition: DiagnosticCondition::MetricThreshold {
            metric_name: "IOTimeVariance",
            operator: ComparisonOperator::GreaterThan,
            threshold: 10.0, // IO时间差异10倍以上
            unit: "倍",
        },
        priority: 14,
        severity: HotSeverity::High,
        strategies: vec![
            DiagnosticStrategy::PresetSuggestions(vec![
                "检查是否存在节点的IO使用率明显高于其它节点".to_string(),
                "验证IO线程池是否打满".to_string(),
                "检查数据在节点上的分布是否均匀".to_string(),
            ]),
        ],
    },

    TuningRule {
        id: "scan.ineffective_filtering",
        name: "数据扫描未有效过滤",
        target_operators: vec!["OLAP_SCAN", "CONNECTOR_SCAN"],
        condition: DiagnosticCondition::BooleanLogic {
            left: Box::new(DiagnosticCondition::MetricThreshold {
                metric_name: "RawRowsRead",
                operator: ComparisonOperator::GreaterThan,
                threshold: 1000000.0, // 读取100万行以上
                unit: "行",
            }),
            operator: LogicOperator::And,
            right: Some(Box::new(DiagnosticCondition::MetricRatio {
                numerator: "RowsRead",
                denominator: "RawRowsRead",
                operator: ComparisonOperator::GreaterThan,
                ratio_threshold: 0.8, // 80%以上未过滤
            })),
        },
        priority: 15,
        severity: HotSeverity::Moderate,
        strategies: vec![
            DiagnosticStrategy::PresetSuggestions(vec![
                "检查表结构是否合理".to_string(),
                "检查排序键是否合适".to_string(),
                "检查是否需要添加索引".to_string(),
                "检查查询条件是否包含函数，导致无法用于过滤数据".to_string(),
            ]),
        ],
    },
];
```

#### Join算子规则集
```rust
pub static JOIN_OPERATOR_RULES: &[TuningRule] = &[
    TuningRule {
        id: "join.result_inflation",
        name: "Join结果膨胀",
        target_operators: vec!["HASH_JOIN", "NESTED_LOOP_JOIN"],
        condition: DiagnosticCondition::MetricRatio {
            numerator: "OutputRowCount",
            denominator: "InputRowCount",
            operator: ComparisonOperator::GreaterThan,
            ratio_threshold: 2.0, // 输出行数是输入行数的2倍以上
        },
        priority: 18,
        severity: HotSeverity::Severe,
        strategies: vec![
            DiagnosticStrategy::PresetSuggestions(vec![
                "检查Join条件是否缺失".to_string(),
                "验证Join条件是否符合业务逻辑".to_string(),
                "增添必要条件以精确匹配，避免结果膨胀".to_string(),
                "优化多表Join顺序，可设置disable_join_reorder=true".to_string(),
                "检查统计信息是否收集或过期".to_string(),
            ]),
        ],
    },

    TuningRule {
        id: "join.build_table_inappropriate",
        name: "Join build表选择不合理",
        target_operators: vec!["HASH_JOIN", "NESTED_LOOP_JOIN"],
        condition: DiagnosticCondition::BooleanLogic {
            left: Box::new(DiagnosticCondition::MetricRatio {
                numerator: "BuildTableSize",
                denominator: "TotalMemory",
                operator: ComparisonOperator::GreaterThan,
                ratio_threshold: 0.3, // Build表占总内存30%以上
            }),
            operator: LogicOperator::And,
            right: Some(Box::new(DiagnosticCondition::MetricThreshold {
                metric_name: "MemoryUsageSpike",
                operator: ComparisonOperator::GreaterThan,
                threshold: 1.0, // 内存使用峰值异常
                unit: "",
            })),
        },
        priority: 17,
        severity: HotSeverity::High,
        strategies: vec![
            DiagnosticStrategy::PresetSuggestions(vec![
                "检查统计信息是否收集或过期".to_string(),
                "通过设置disable_join_reorder=true手动控制Join顺序".to_string(),
                "使用hints强制指定Build表".to_string(),
                "考虑使用Broadcast Join".to_string(),
            ]),
        ],
    },

    TuningRule {
        id: "join.incorrect_broadcast",
        name: "Join不应该使用Broadcast",
        target_operators: vec!["HASH_JOIN", "NESTED_LOOP_JOIN"],
        condition: DiagnosticCondition::BooleanLogic {
            left: Box::new(DiagnosticCondition::MetricThreshold {
                metric_name: "BroadcastTableSize",
                operator: ComparisonOperator::GreaterThan,
                threshold: 100000000.0, // 广播表超过1亿行
                unit: "行",
            }),
            operator: LogicOperator::And,
            right: Some(Box::new(DiagnosticCondition::MetricThreshold {
                metric_name: "NetworkCostRatio",
                operator: ComparisonOperator::GreaterThan,
                threshold: 0.5, // 网络开销占总开销50%以上
                unit: "",
            })),
        },
        priority: 16,
        severity: HotSeverity::Moderate,
        strategies: vec![
            DiagnosticStrategy::PresetSuggestions(vec![
                "检查统计信息是否准确".to_string(),
                "考虑使用Shuffle Join替代Broadcast Join".to_string(),
                "调整broadcast_row_limit参数".to_string(),
                "优化表大小估算".to_string(),
            ]),
        ],
    },
];
```

#### Aggregate算子规则集
```rust
pub static AGGREGATE_OPERATOR_RULES: &[TuningRule] = &[
    TuningRule {
        id: "aggregate.low_local_aggregation",
        name: "Aggregate本地聚合度低",
        target_operators: vec!["AGGREGATE"],
        condition: DiagnosticCondition::BooleanLogic {
            left: Box::new(DiagnosticCondition::MetricRatio {
                numerator: "InputRowCount",
                denominator: "AfterLocalAggRowCount",
                operator: ComparisonOperator::LessThan,
                ratio_threshold: 2.0, // 本地聚合后行数减少不明显
            }),
            operator: LogicOperator::And,
            right: Some(Box::new(DiagnosticCondition::MetricThreshold {
                metric_name: "NetworkTransferBytes",
                operator: ComparisonOperator::GreaterThan,
                threshold: 1000000000.0, // 传输1GB以上数据
                unit: "字节",
            })),
        },
        priority: 14,
        severity: HotSeverity::Moderate,
        strategies: vec![
            DiagnosticStrategy::PresetSuggestions(vec![
                "设置new_planner_agg_stage=1关闭二阶段聚合".to_string(),
                "考虑使用单阶段聚合优化".to_string(),
                "调整streaming_preaggregation_enable参数".to_string(),
                "检查GROUP BY键的选择是否合适".to_string(),
            ]),
        ],
    },
];
```

### 3. 诊断引擎设计

```rust
pub struct IntelligentDiagnoser {
    rule_registry: RuleRegistry,
    context_builder: ContextBuilder,
    strategy_executor: StrategyExecutor,
}

impl IntelligentDiagnoser {
    pub fn diagnose(&self, profile: &Profile) -> Vec<DetailedDiagnosis> {
        let mut diagnoses = Vec::new();

        // 1. 构建诊断上下文
        let global_context = self.context_builder.build_global_context(profile);
        let operator_contexts = self.context_builder.build_operator_contexts(profile);

        // 2. 执行全局诊断
        diagnoses.extend(self.diagnose_global_issues(&global_context));

        // 3. 执行算子级别诊断
        for operator_ctx in operator_contexts {
            diagnoses.extend(self.diagnose_operator(&operator_ctx));
        }

        // 4. 执行关联诊断
        diagnoses.extend(self.diagnose_correlations(&diagnoses, &global_context));

        diagnoses
    }

    fn diagnose_operator(&self, context: &OperatorContext) -> Vec<DetailedDiagnosis> {
        let applicable_rules = self.rule_registry
            .find_applicable_rules(&context.operator_name);

        applicable_rules.iter()
            .filter(|rule| rule.condition.evaluate(context))
            .map(|rule| {
                let diagnosis = DetailedDiagnosis {
                    rule_id: rule.id.clone(),
                    operator_context: context.clone(),
                    severity: rule.severity.clone(),
                    confidence_score: self.calculate_confidence(rule, context),
                    strategies: rule.strategies.clone(),
                };
                // 执行诊断策略
                self.strategy_executor.execute_strategies(&diagnosis)
            })
            .collect()
    }
}
```

### 4. 智能建议生成

#### 建议优先级引擎

```rust
pub struct SuggestionPrioritizer {
    suggestion_templates: HashMap<String, SuggestionTemplate>,
    context_analyzer: ContextAnalyzer,
}

impl SuggestionPrioritizer {
    pub fn generate_suggestions(&self, diagnosis: &DetailedDiagnosis) -> Vec<PrioritizedSuggestion> {
        diagnosis.strategies.iter()
            .flat_map(|strategy| self.strategy_executor.generate_suggestions(strategy, diagnosis))
            .map(|suggestion| PrioritizedSuggestion {
                content: suggestion,
                priority: self.calculate_priority(diagnosis, &suggestion),
                context_relevance: self.analyze_context_relevance(diagnosis, &suggestion),
            })
            .sorted_by(|a, b| a.priority.cmp(&b.priority).reverse())
            .take(5) // 返回前5个最优先的建议
            .collect()
    }

    fn calculate_priority(&self, diagnosis: &DetailedDiagnosis, suggestion: &str) -> i32 {
        let base_priority = match diagnosis.severity {
            HotSeverity::Critical => 100,
            HotSeverity::High => 80,
            HotSeverity::Severe => 60,
            HotSeverity::Moderate => 40,
            HotSeverity::Mild => 20,
            HotSeverity::Normal => 10,
        };

        // 根据操作符类型调整优先级
        let operator_modifier = match diagnosis.operator_context.operator_name.as_str() {
            "CONNECTOR_SCAN" => 10,
            "OLAP_SCAN" => 8,
            "HASH_JOIN" => 12,
            "AGGREGATE" => 6,
            _ => 0,
        };

        // 根据建议类型调整优先级
        let suggestion_modifier = if suggestion.contains("compaction") { 15 }
                                else if suggestion.contains("memory") { 12 }
                                else if suggestion.contains("index") { 10 }
                                else { 0 };

        base_priority + operator_modifier + suggestion_modifier
    }
}
```

## 🎨 前端可视化设计

### 1. 主界面布局

```
┌─────────────────────────────────────────────────────────────┐
│                         Header                              │
│        StarRocks Profile 智能分析器                          │
├─────────────────────────┬───────────────────────────────────┤
│        上传区域           │        分析概览                   │
├─────────────────────────┴───────────────────────────────────┤
│                                                             │
│                 执行计划可视化区域                          │
│                                                             │
│    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    │
│    │  Fragment   │ -> │  Pipeline  │ -> │  Operator  │    │
│    │   (绿)      │    │   (黄)      │    │   (红)      │    │
│    └─────────────┘    └─────────────┘    └─────────────┘    │
│                                                             │
├─────────────────────────┬───────────────────────────────────┤
│        热点问题面板      │        诊断建议面板               │
│    ┌─────────────────┐   │   ┌───────────────────────┐    │
│    │ ⚠️  CRITICAL    │   │   │ 💡 最优先优化建议     │    │
│    │ 内存使用过高    │   │   │ • 增加BE内存配置      │    │
│    └─────────────────┘   │   │ • 启动可以溢出运算符 │    │
│                         │   │                       │    │
├─────────────────────────┴───────────────────────────────────┤
│                          统计信息面板                         │
└─────────────────────────────────────────────────────────────┘
```

### 2. 执行计划可视化组件

#### DAG可视化引擎

```typescript
// frontend/src/components/DAGVisualization.vue
export default {
  name: 'DAGVisualization',

  props: {
    executionTree: {
      type: Object,
      required: true
    },
    hotspots: {
      type: Array,
      default: () => []
    }
  },

  data() {
    return {
      nodes: [],
      edges: [],
      selectedNode: null
    }
  },

  mounted() {
    this.initializeDAG()
  },

  methods: {
    initializeDAG() {
      // 构建D3力导向图布局
      this.simulation = d3.forceSimulation(this.nodes)
        .force('link', d3.forceLink(this.edges).id(d => d.id))
        .force('charge', d3.forceManyBody())
        .force('center', d3.forceCenter(this.width / 2, this.height / 2))

      this.renderDAG()
    },

    renderDAG() {
      const svg = d3.select(this.$refs.svg)

      // 绘制边
      svg.selectAll('line')
        .data(this.edges)
        .enter()
        .append('line')
        .attr('stroke', this.getEdgeColor)
        .attr('stroke-width', 2)

      // 绘制节点
      const nodeElements = svg.selectAll('circle')
        .data(this.nodes)
        .enter()
        .append('circle')
        .attr('r', d => this.getNodeRadius(d))
        .attr('fill', d => this.getNodeColor(d))
        .attr('stroke', d => this.getNodeStrokeColor(d))
        .attr('stroke-width', d => this.getNodeStrokeWidth(d))

      // 添加交互
      nodeElements
        .on('click', (event, d) => this.onNodeClick(d))
        .on('mouseover', (event, d) => this.onNodeHover(d))
        .on('mouseout', () => this.onNodeOut())

      // 添加节点标签
      svg.selectAll('text')
        .data(this.nodes)
        .enter()
        .append('text')
        .text(d => d.label)
        .attr('font-size', '10px')
        .attr('text-anchor', 'middle')
        .attr('dy', '.35em')
    },

    getNodeColor(node) {
      const hotspotSeverity = this.getHotspotSeverity(node.id)
      switch (hotspotSeverity) {
        case 'critical': return '#f5222d'
        case 'high': return '#722ed1'
        case 'severe': return '#fa541a'
        case 'moderate': return '#fa8c16'
        case 'mild': return '#faad14'
        default: return '#52c41a'  // normal
      }
    },

    getNodeRadius(node) {
      // 根据执行时间动态调整节点大小
      const minRadius = 20
      const maxRadius = 50
      const timeRatio = node.executionTime / this.maxExecutionTime
      return minRadius + (maxRadius - minRadius) * timeRatio
    }
  }
}
```

#### 动态热点标识

```typescript
computed: {
  nodesWithHotspots() {
    return this.nodes.map(node => ({
      ...node,
      hotspotLevel: this.calculateHotspotLevel(node.id),
      isBottleneck: this.isBottleneckNode(node.id)
    }))
  }
},

methods: {
  calculateHotspotLevel(nodeId: string) {
    const hotspots = this.hotspots.filter(h => h.nodePath.includes(nodeId))
    if (hotspots.length === 0) return 0

    const maxSeverity = Math.max(...hotspots.map(h => this.severityToNumber(h.severity)))
    return maxSeverity
  },

  isBottleneckNode(nodeId: string) {
    const hotspots = this.hotspots.filter(h => h.nodePath.includes(nodeId))
    return hotspots.some(h => h.issueType.includes('bottleneck'))
  }
}
```

### 3. 诊断建议面板

#### 智能建议排序和分组

```typescript
export default {
  name: 'IntelligentSuggestions',

  props: {
    diagnosisResult: {
      type: Object,
      required: true
    }
  },

  computed: {
    categorizedSuggestions() {
      const categories = {
        critical: [],
        immediate: [],
        medium: [],
        low: []
      }

      this.diagnosisResult.suggestions.forEach(suggestion => {
        const priority = suggestion.priority

        if (priority >= 80) categories.critical.push(suggestion)
        else if (priority >= 60) categories.immediate.push(suggestion)
        else if (priority >= 40) categories.medium.push(suggestion)
        else categories.low.push(suggestion)
      })

      return categories
    },

    confidenceIndicators() {
      return this.diagnosisResult.suggestions.map(suggestion => ({
        confidence: suggestion.confidenceScore,
        confidenceLabel: this.getConfidenceLabel(suggestion.confidenceScore),
        confidenceColor: this.getConfidenceColor(suggestion.confidenceScore)
      }))
    }
  },

  methods: {
    getConfidenceLabel(score) {
      if (score >= 0.8) return '高度可信'
      if (score >= 0.6) return '可信'
      if (score >= 0.4) return '中度可信'
      return '低度可信'
    },

    getConfidenceColor(score) {
      if (score >= 0.8) return '#52c41a'    // green
      if (score >= 0.6) return '#1890ff'    // blue
      if (score >= 0.4) return '#faad14'    // yellow
      return '#f5222d'                      // red
    },

    getImplementationDifficulty(suggestion) {
      // 基于建议内容分析实施难度
      const easyKeywords = ['配置参数', '调整设置']
      const mediumKeywords = ['添加索引', '优化查询']
      const hardKeywords = ['重构表', '数据迁移']

      if (easyKeywords.some(k => suggestion.content.includes(k))) return 'easy'
      if (mediumKeywords.some(k => suggestion.content.includes(k))) return 'medium'
      if (hardKeywords.some(k => suggestion.content.includes(k))) return 'hard'
      return 'unknown'
    }
  }
}
```

## 🔧 技术实现细节

### 1. 规则引擎实现

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct RuleEngine {
    rules: Vec<Arc<TuningRule>>,
    context_cache: RwLock<HashMap<String, ContextCacheEntry>>,
}

impl RuleEngine {
    pub async fn evaluate_rules(&self, context: &OperatorContext) -> Vec<RuleResult> {
        let mut results = Vec::new();

        // 并行评估规则
        let rule_futures: Vec<_> = self.rules.iter()
            .filter(|rule| self.rule_applies_to_context(rule, context))
            .map(|rule| {
                let rule = rule.clone();
                let context = context.clone();
                tokio::spawn(async move {
                    self.evaluate_single_rule(&rule, &context).await
                })
            })
            .collect();

        // 等待所有规则评估完成
        let rule_results = futures::future::join_all(rule_futures).await;

        for result in rule_results {
            if let Ok(Some(rule_result)) = result {
                results.push(rule_result);
            }
        }

        // 按优先级排序
        results.sort_by(|a, b| a.priority.cmp(&b.priority));

        results
    }

    fn rule_applies_to_context(&self, rule: &TuningRule, context: &OperatorContext) -> bool {
        // 检查操作符类型匹配
        if !rule.target_operators.is_empty() &&
           !rule.target_operators.contains(&context.operator_name) {
            return false;
        }

        // 检查其他预过滤条件
        rule.pre_filter.as_ref()
            .map_or(true, |filter| filter.evaluate(context))
    }

    async fn evaluate_single_rule(&self, rule: &TuningRule, context: &OperatorContext) -> Option<RuleResult> {
        // 1. 获取指标值
        let metric_values = self.collect_metric_values(rule, context).await;

        // 2. 评估条件
        let condition_result = rule.condition.evaluate_with_context(context, &metric_values)?;

        if !condition_result.matched {
            return None;
        }

        // 3. 计算置信度
        let confidence_score = self.calculate_confidence(rule, &condition_result, context);

        // 4. 生成结果
        Some(RuleResult {
            rule_id: rule.id.clone(),
            severity: rule.severity.clone(),
            condition_result,
            confidence_score,
            priority: self.calculate_priority(rule, confidence_score),
            suggested_actions: vec![], // 由建议引擎填充
        })
    }
}
```

### 2. 上下文自适应

#### 动态阈值调整

```rust
#[derive(Debug)]
pub struct AdaptiveThresholdManager {
    historical_data: HistoricalDataStore,
    cluster_info: ClusterInformation,
    workload_patterns: WorkloadPatternAnalyzer,
}

impl AdaptiveThresholdManager {
    pub fn adjust_threshold(&self, rule: &TuningRule, cluster_context: &ClusterContext) -> f64 {
        let base_threshold = rule.threshold;

        // 根据集群规模调整
        let scale_modifier = self.calculate_scale_modifier(cluster_context);

        // 根据历史负载调整
        let load_modifier = self.calculate_load_modifier();

        // 根据数据模式调整
        let pattern_modifier = self.calculate_pattern_modifier(cluster_context);

        base_threshold * scale_modifier * load_modifier * pattern_modifier
    }

    fn calculate_scale_modifier(&self, context: &ClusterContext) -> f64 {
        // 小集群相对宽松，大集群相对严格
        match context.be_count {
            1..=3 => 1.2,    // 小集群放宽20%
            4..=10 => 1.0,   // 中等集群标准值
            11..=50 => 0.9,  // 大集群收紧10%
            _ => 0.8,         // 超大集群收紧20%
        }
    }
}
```

### 3. 多维度关联诊断

```rust
pub struct CorrelationAnalyzer {
    correlation_patterns: Vec<CorrelationPattern>,
    temporal_analyzer: TemporalPatternAnalyzer,
}

impl CorrelationAnalyzer {
    pub fn analyze_correlations(&self, diagnoses: &[DetailedDiagnosis]) -> Vec<CorrelationResult> {
        let mut correlations = Vec::new();

        // 内存压力相关的多节点关联
        correlations.extend(self.analyze_memory_correlations(diagnoses));

        // IO压力的上游传播分析
        correlations.extend(self.analyze_io_correlations(diagnoses));

        // 网络瓶颈的级联效应分析
        correlations.extend(self.analyze_network_correlations(diagnoses));

        correlations
    }

    fn analyze_memory_correlations(&self, diagnoses: &[DetailedDiagnosis]) -> Vec<CorrelationResult> {
        // 寻找内存热点节点，分析是否形成瓶颈链
        let memory_hotspots: Vec<_> = diagnoses.iter()
            .filter(|d| d.severity >= HotSeverity::Severe)
            .filter(|d| d.primary_metric.as_ref().map_or(false, |m| m.category == "memory"))
            .collect();

        if memory_hotspots.len() >= 2 {
            // 检查是否有直接的数据流关系
            let memory_chain = self.find_memory_chain(&memory_hotspots);

            if !memory_chain.is_empty() {
                return vec![CorrelationResult {
                    correlation_type: CorrelationType::MemoryBottleneckChain,
                    involved_diagnoses: memory_chain,
                    description: "检测到内存瓶颈传播链",
                    recommended_actions: vec![
                        "考虑增加整体内存分配".to_string(),
                        "优化内存密集型操作的分布".to_string(),
                    ],
                }];
            }
        }

        Vec::new()
    }
}
```

## 📊 性能监控和优化

### 1. 诊断性能监控

```rust
#[derive(Debug)]
pub struct DiagnosticsPerformanceMonitor {
    metrics_collector: MetricsCollector,
    performance_alerts: Vec<PerformanceAlert>,
}

impl DiagnosticsPerformanceMonitor {
    pub async fn monitor_diagnostic_performance(&self, duration: Duration, rules_count: usize) {
        // 记录诊断耗时
        self.metrics_collector.record_duration("diagnostic_processing_time", duration);

        // 检查性能阈值
        if duration > Duration::from_millis(500) {
            self.performance_alerts.push(PerformanceAlert {
                alert_type: AlertType::SlowDiagnosticProcessing,
                message: format!("诊断处理耗时过长: {:?}", duration),
                suggestions: vec![
                    "优化规则计算逻辑".to_string(),
                    "考虑缓存常用指标".to_string(),
                    "减少并发规则评估数量".to_string(),
                ],
            });
        }

        // 计算平均处理时间
        self.metrics_collector
            .histogram("diagnostics_processing_per_rule")
            .record(duration.as_millis_f64() / rules_count as f64);
    }
}
```

### 2. 智能缓存机制

```rust
pub struct IntelligentCache {
    profile_cache: LruCache<String, CachedProfileData>,
    metrics_cache: MetricsCache,
    rules_cache: RuleExecutionCache,
}

impl IntelligentCache {
    pub async fn get_or_compute_profile_data(&self, profile_hash: &str, compute_fn: impl Future) -> Result<CachedProfileData> {
        if let Some(cached) = self.profile_cache.get(profile_hash) {
            // 检查缓存是否新鲜
            if self.is_cache_fresh(cached.timestamp) {
                self.metrics_collector.increment("cache_hit");
                return Ok(cached.data.clone());
            }
        }

        // 重新计算
        self.metrics_collector.increment("cache_miss");
        let data = compute_fn.await?;
        self.profile_cache.put(profile_hash.to_string(), CachedProfileData {
            data: data.clone(),
            timestamp: SystemTime::now(),
        });

        Ok(data)
    }
}
```

## 🚀 部署和运维

### 1. 容器化部署

```dockerfile
# Dockerfile
FROM rust:1.70-slim AS backend-build
WORKDIR /app
COPY backend/Cargo.toml backend/Cargo.lock ./
COPY backend/src ./src
RUN cargo build --release

FROM node:18-alpine AS frontend-build
WORKDIR /app
COPY frontend/package*.json ./
RUN npm ci
COPY frontend .
RUN npm run build

FROM debian:11-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=backend-build /app/target/release/starrocks-profile-analyzer /usr/local/bin/
COPY --from=frontend-build /app/dist /var/www/html
EXPOSE 3030
CMD ["starrocks-profile-analyzer"]
```

### 2. 配置管理

```yaml
# config/application.yaml
diagnostics:
  rules:
    enabled_rules: ["connector_scan.*", "memory_pressure.*", "io_efficiency.*"]
    min_confidence_score: 0.6

  thresholds:
    adaptive_enabled: true
    adaptation_window: "1h"
    min_samples_for_adaptation: 10

  caching:
    enabled: true
    profile_cache_ttl: "30m"
    metrics_cache_ttl: "10m"

performance:
  max_concurrent_analyses: 10
  analysis_timeout: "30s"
  enable_metrics_collection: true

visualization:
  max_nodes_in_graph: 200
  dag_layout_algorithm: "force_directed"
  color_scheme: "severity_based"
```

### 3. 监控和告警

```rust
#[derive(Debug)]
pub struct SystemMonitor {
    diagnostics_performance: DiagnosticsPerformanceMonitor,
    alert_manager: AlertManager,
}

impl SystemMonitor {
    pub async fn run_monitoring_loop(&self) {
        loop {
            // 检查系统健康状态
            self.check_system_health().await;

            // 检查诊断性能指标
            self.check_diagnostic_performance().await;

            // 检查缓存命中率
            self.check_cache_effectiveness().await;

            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    }
}
```

## 🎯 总结

这个设计方案提供了一个完整的、基于规则引擎的智能诊断建议系统，具有以下特点：

1. **精确诊断**：基于StarRocks官方tuning recipes的规则引擎
2. **智能建议**：多维度关联分析和上下文自适应
3. **高性能**：并行处理和智能缓存机制
4. **可视化驱动**：直观的DAG图和热点标识
5. **可扩展架构**：模块化设计便于扩展新规则

系统从策略执行层的"规则引擎"到前端可视化的"智能建议面板"构建了一个完整的诊断闭环，能够为用户提供精准、实用的StarRocks查询性能优化建议。
