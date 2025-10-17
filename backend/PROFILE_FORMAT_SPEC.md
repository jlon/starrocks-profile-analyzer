# StarRocks Profile 格式规范

> 基于 StarRocks 源码分析得出的 Profile 文本格式规范
> 
> 参考源码:
> - `fe/fe-core/src/main/java/com/starrocks/common/util/RuntimeProfile.java`
> - `fe/fe-core/src/main/java/com/starrocks/common/util/DebugUtil.java`

## 1. 整体结构

Profile 由 **递归的节点树** 组成，每个节点包含：
1. **节点名称** (Profile Name)
2. **信息字符串** (Info Strings)
3. **计数器** (Counters) - 可递归嵌套
4. **子节点** (Children) - 递归结构

## 2. 节点格式

### 2.1 节点名称行

```
{prefix}{profile_name}:(Active: {total_time}, % non-child: {percent}%)
```

**示例**:
```
Fragment 0:
Pipeline (id=0):(Active: 1h30m, % non-child: 12.34%)
  CONNECTOR_SCAN (plan_node_id=0):
```

**规则**:
- 节点名称以 `:` 结尾
- Active 时间和百分比是**可选的**
- `prefix` 是当前的缩进前缀

### 2.2 Info Strings (信息字符串)

```
{prefix}   - {key}: {value}
```

**关键规则**:
- 固定前缀: `"   - "` (3个空格 + 短横线 + 1个空格)
- 如果 `value` 为空白，则**不打印冒号和值**
- Info Strings 紧跟在节点名称之后

**示例**:
```
  Summary:
     - Query ID: b1f9a935-a967-11f0-b3d8-f69e292b7593
     - Start Time: 2025-10-15 09:38:48
     - IsFinalSink
```

### 2.3 Counters (计数器)

```
{prefix}   - {counter_name}: {formatted_value}
```

**关键规则**:
- 格式与 Info Strings 相同
- **支持递归嵌套**: 子计数器会增加 `"  "` (2个空格) 缩进
- `__MIN_OF_` 和 `__MAX_OF_` 前缀的计数器会**排在前面**

**示例**:
```
  CommonMetrics:
     - OperatorTotalTime: 7s854ms
       - __MAX_OF_OperatorTotalTime: 1h30m
       - __MIN_OF_OperatorTotalTime: 5.540us
     - PullChunkNum: 1
```

### 2.4 子节点缩进

- 每个子节点根据 `indent` 标志决定是否增加 `"  "` (2个空格) 的缩进
- Fragment 下的 Pipeline 会缩进
- Pipeline 下的 Operator 会缩进

## 3. 值格式化规则

### 3.1 时间格式 (TIME_NS)

**源码**: `RuntimeProfile.printCounter()` + `DebugUtil.printTimeMs()`

时间值原始单位是**纳秒 (ns)**，根据大小转换：

#### 规则 1: ≥ 1秒 (≥ 1,000,000,000 ns)
转换为毫秒后使用 `printTimeMs` 格式化:

```
{hours}h{minutes}m{seconds}s{milliseconds}ms
```

**特殊规则**:
- 如果有小时 (h)，则**忽略毫秒精度**
- 如果有小时或分钟，则忽略毫秒
- 零值部分不显示

**示例**:
```
1h30m        // 1小时30分钟 (忽略秒和毫秒)
45m30s       // 45分钟30秒 (忽略毫秒)
30s500ms     // 30秒500毫秒
```

#### 规则 2: ≥ 1毫秒 且 < 1秒 (≥ 1,000,000 ns)
```
{integer}ms           // 整数毫秒
{integer}.{frac}ms    // 带小数的毫秒 (3位小数)
```

**示例**:
```
123ms
123.456ms
7.854ms
```

#### 规则 3: ≥ 1微秒 且 < 1毫秒 (≥ 1,000 ns)
```
{integer}us           // 整数微秒
{integer}.{frac}us    // 带小数的微秒 (3位小数)
```

**示例**:
```
500us
123.456us
5.540us
```

#### 规则 4: < 1微秒 (< 1,000 ns)
```
{value}ns
```

**示例**:
```
390ns
0ns
```

### 3.2 字节格式 (BYTES)

**源码**: `DebugUtil.getByteUint()`

```
{number} {unit}
```

**单位转换** (基于 1024):
- `B` (bytes): < 1024
- `KB` (kilobytes): ≥ 1024
- `MB` (megabytes): ≥ 1024 * 1024
- `GB` (gigabytes): ≥ 1024 * 1024 * 1024
- `TB` (terabytes): ≥ 1024 * 1024 * 1024 * 1024

**数字格式**: `%.3f` (3位小数)

**示例**:
```
2.167 KB
259.547 GB
1.768 MB
0.000 B
```

### 3.3 大数字单位格式 (UNIT)

**源码**: `DebugUtil.getUint()`

```
{formatted} ({raw_value})
```

**单位转换** (基于 1000):
- `K` (千): ≥ 1,000
- `M` (百万): ≥ 1,000,000
- `B` (十亿): ≥ 1,000,000,000

**数字格式**: `%.3f` (3位小数)

**特殊规则**:
- 如果值 < 1000，直接显示原始数字，**不带括号**
- 如果值 ≥ 1000，显示格式化值 + 括号中的原始值

**示例**:
```
2.174K (2174)          // 大于1000
334                    // 小于1000，直接显示
1.234M (1234567)
```

### 3.4 双精度浮点数 (DOUBLE_VALUE)

```
{number}
```

**数字格式**: `%.3f` (3位小数)

### 3.5 普通整数 (无单位)

```
{value}
```

直接显示原始整数值

## 4. 解析策略

### 4.1 逐行解析

Profile 是**严格的逐行格式**，应该：
1. 按行分割文本
2. 根据缩进级别判断层级关系
3. 根据行模式识别类型

### 4.2 缩进规则

- 使用**空格**缩进（不使用 Tab）
- 每级缩进通常是 **2个空格** 或 **3个空格**
- Info Strings 和 Counters 固定使用 `"   - "` 前缀

### 4.3 节点识别模式

```regex
// 节点名称行 (以冒号结尾)
^(\s*)([\w\s\(\)=,-]+):

// Info String / Counter 行
^\s*-\s+([A-Za-z][A-Za-z0-9_]*(?:\s+[A-Za-z][A-Za-z0-9_]*)*)(?::\s*(.+))?$

// Operator 头部
^([A-Z_]+)\s*\(plan_node_id=(-?\d+)(?:\s*\(operator\s+id=(\d+)\))?\):?

// Fragment 头部  
^\s*Fragment\s+(\d+):

// Pipeline 头部
^\s*Pipeline\s+\(id=(\d+)\):
```

### 4.4 值解析优先级

解析数值时应该按照以下优先级：

1. **检查括号内的原始值**: 如 `2.174K (2174)` → 使用 `2174`
2. **提取第一个数字和单位**: 解析格式化的值
3. **处理逗号分隔符**: `1,234` → `1234`

## 5. 特殊标记

### 5.1 聚合标记

- `__MIN_OF_{counter_name}`: 最小值
- `__MAX_OF_{counter_name}`: 最大值

这些计数器在合并多个 BE 实例的 Profile 时生成。

### 5.2 可选标记

某些 Info Strings 可能没有值，例如：
```
- IsFinalSink
- IsSubordinate
```

这些行**没有冒号和值**。

## 6. 典型结构示例

```
Query:
  Summary:
     - Query ID: xxx
     - Start Time: 2025-10-15 09:38:48
  Planner:
     - -- Parser[1] 0
     - -- Total[1] 13ms
  Execution:
     - Topology: {"rootId":1,...}
     - QueryAllocatedMemoryUsage: 259.547 GB
    Fragment 0:
       - BackendAddresses: 192.168.1.1:9060
      Pipeline (id=0):
         - DegreeOfParallelism: 32
         - DriverTotalTime: 1h30m
        CONNECTOR_SCAN (plan_node_id=0):
          CommonMetrics:
             - OperatorTotalTime: 7s854ms
               - __MAX_OF_OperatorTotalTime: 1h30m
             - PullChunkNum: 1
          UniqueMetrics:
             - Table: test_table
             - BytesRead: 2.167 KB
```

## 7. 解析器设计建议

基于以上规范，解析器应该：

1. **状态机解析**: 根据当前章节状态解析不同格式
2. **递归解析 Counters**: 正确处理嵌套的计数器
3. **智能值提取**: 
   - 优先使用括号内的原始值
   - 正确识别和解析各种单位格式
4. **容错性**: 
   - 处理可选字段
   - 处理格式变化
5. **通用性**: 
   - 不要硬编码特定的键名
   - 使用正则表达式匹配模式而非精确字符串

## 8. 测试要点

解析器必须正确处理：

- [x] 多级缩进的嵌套结构
- [x] 带括号的数字格式: `2.174K (2174)`
- [x] 复杂时间格式: `1h30m`, `7s854ms`, `5.540us`, `390ns`
- [x] 字节单位: `2.167 KB`, `259.547 GB`
- [x] `__MIN_OF_` 和 `__MAX_OF_` 前缀
- [x] 没有值的 Info String: `- IsFinalSink`
- [x] 递归的 Counter 结构
- [x] 多个 Fragment 和 Pipeline
- [x] 各种 Operator 类型

---

**版本**: v1.0  
**更新日期**: 2025-10-17  
**基于 StarRocks 版本**: 3.5.2

