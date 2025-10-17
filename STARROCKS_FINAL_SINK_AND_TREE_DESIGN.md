# StarRocks Profile 解析：Final Sink 与执行树生成设计

## 目标
- 基于 Profile 的 `Execution` 章节中的 Topology JSON 严格生成执行树。
- 在 Topology 的 `nodes` 中补齐最终的 `_SINK` 节点（final sink），并将其作为最终树根。
- 提供通用、可扩展的 SINK 选择与树构建策略，适配不同版本与形态的 Profile，避免写死解析逻辑。

## 输入
- Topology（来自 `Execution`）：包含 `rootId` 与 `nodes`（`id`、`name`、`properties`、`children`）。
- Fragments（来自 `Fragments`）：包含跨多个 Fragment 的 `pipelines` 与 `operators`，每个 operator 可能具备 `plan_node_id`、`name`、`metrics` 等。
- Profile 文本原文：作为兜底信息源（例如查找 `_SINK` 操作符名）。

## Final Sink 的通用定义（来自 StarRocks 语义）
- Final Sink 是产生最终结果或持久化外部存储的 DataSink：如 `RESULT_SINK`、`OLAP_TABLE_SINK`、其他外部 `*_SINK`。
- 非 Final Sink：数据转发类 `EXCHANGE_SINK`、`LOCAL_EXCHANGE_SINK`，以及 `MULTI_CAST` 类 Sink（用于分发/广播）。
- 判定规则（通用且可扩展）：
  - 候选必须满足：操作符名或类型以 `_SINK` 结尾。
  - 排除包含：`EXCHANGE_SINK`、`LOCAL_EXCHANGE_SINK`、`MULTI_CAST`。
  - 优先级：`RESULT_SINK`(1) > `OLAP_TABLE_SINK`(2) > 其它 `TABLE_SINK`(3) > 其它 SINK(6)。
  - 扩展钩子：若 Profile/Topology 提供标志（例如 `IsFinalSink` 或属性位），优先使用该标志以提升鲁棒性。

## SINK 选择算法（通用策略）
1. 在所有 Fragments 的 `pipelines`/`operators` 中收集 `_SINK` 候选。
2. 对每个候选应用 `is_final_sink`（排除 EXCHANGE/LOCAL_EXCHANGE/MULTI_CAST）。
3. 按 `get_sink_priority` 排序并选择优先级最高的 Final Sink。
4. Fallback：若不存在 Final Sink，则不添加 `_SINK` 节点，树根回退为 Topology 的 `rootId`。
5. 扩展：若出现 `IsFinalSink` 标记（或类似属性），该标记直接提升为最高优先级用于确定最终 SINK。

## 在 Topology 中补齐 SINK 节点
- 若最终选定的 SINK 不在 Topology 的 `nodes` 中：
  - 使用其 `plan_node_id` 作为 `id`（若不可得，则用 `-1` 占位），`name` 使用候选名（如 `OLAP_TABLE_SINK`）。
  - `children` 为空（SINK 为叶子，不再向下传递）。
  - `properties` 初始化为空或保留必要属性（例如后续可加入 `displayMem`、`isFinalSink: true`）。
- 保持原始 Topology 的结构不变（不直接修改现有边），树构建阶段再进行附加连接。

## 树生成逻辑（严格遵循 Topology）
- 核心策略：以 Topology 的结构为主，跨 Fragment 收集实际 Operator 信息进行匹配和充实。
- 步骤：
  1. 从 Topology 的 `nodes` 建立基本节点图与 `id -> index` 映射。
  2. 跨所有 Fragments 收集 Operator，采用名称归一化（`extract_operator_name`）进行智能匹配：
     - 以 Topology 节点名为锚，找到最匹配的 Operator（优先同名、其次同类）。
     - 将 Operator 的 `plan_node_id`、`metrics` 合并回对应的树节点。
  3. 若选定了 Final SINK：
     - 将 SINK 节点提升为新树根，并将“原 Topology 根”作为其唯一子节点（新增一条边，仅在树层级中体现）。
     - 清理误连关系（例如某些 Profile 会把 SINK 错误地作为原根的子节点）。
  4. 若未选定 SINK：树根维持为 Topology 的 `rootId` 对应节点。
  5. 从最终树根（通常是 SINK）开始 BFS 重新计算深度。

- 说明：
  - “严格遵循 Topology”的含义：所有非 SINK 的父子关系严格以 Topology 的 `children` 为准；仅在树层级上附加一条由 SINK 指向原根的边，以表达最终输出语义。
  - 该附加边为逻辑上的树根连线，不修改 Topology 原始 JSON，保证 Topology 的可核对性与可移植性。

## 指标与属性合并（跨 Fragments）
- 合并原则：
  - 优先使用匹配到的 Operator 的 `plan_node_id` 作为树节点的 `plan_node_id`。
  - 指标合并遵循“缺省不覆盖”的原则：Topology 保留结构，Fragments 提供度量（如时间、行数、吞吐）。
  - 若一个 Topology 节点对应多个物理 Operator（常见于 SINK/SOURCE），按优先策略合并代表性的 Operator 指标；保留其余为附加信息或调试输出。

## 适配性与扩展
- 适配不同 Profile：
  - 名称差异：使用 `_SINK` 后缀和排除关键字的“模式匹配”，而非枚举具体类型，保证向后兼容。
  - 结构差异：无 SINK 的 Topology 能正常回退；多 SINK 时根据优先级选取最合适根。
  - 语义差异：可通过属性位（如 `IsFinalSink`）增强判断，便于未来 StarRocks 的格式更新。
- 可扩展点：
  - `is_final_sink` 与 `get_sink_priority` 支持策略化（未来可根据版本或租户配置调整）。
  - 允许在 Topology `properties` 注入解析标记（如 `isFinalSink: true`），但不影响原始结构。

## 校验与测试
- 单测：`backend/tests/test_dag_generation.rs::test_profile5_sink_nodes`
  - 校验树根为 Final SINK，且指向原 Topology 根。
  - 校验存在 `TABLE_SINK` 节点。


## 非写死实现的保证
- 使用后缀/关键字模式而非完整枚举，减少对具体类型的强耦合。
- 优先级与排除列表基于 StarRocks 通用语义（RESULT/TABLE vs EXCHANGE/LOCAL/MULTI_CAST），并提供扩展钩子。
- 结构上仅增加一条逻辑连接（SINK→原根）用于树根表达，不改变原始 Topology 的节点与边集合。

## 代码位置（便于审阅）
- `backend/src/parser/core/topology_parser.rs`：补齐并注入 SINK 节点（`extract_and_add_sink_nodes`、`select_sink_node`）。
- `backend/src/parser/core/tree_builder.rs`：严格按 Topology 构建树，并将最终 SINK 提升为树根，重新计算深度。
- `backend/src/parser/composer.rs`：主编排，解析各章节，整合 Topology + Fragments，执行树生成。
- `backend/tests/test_dag_generation.rs`：profile5 的 SINK 根校验用例。

## 后续可选增强
- 解析 `IsFinalSink` 或类似提示位（若未来 Profile 提供），进一步减少字符串匹配的不确定性。
- 将优先级与排除策略抽象为策略接口，支持通过配置文件按租户/版本进行定制。