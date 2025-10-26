/**
 * NodeType 枚举
 * 对应后端: backend/src/models.rs::NodeType
 *
 * ⚠️ 必须与后端完全同步！
 */
export const NodeType = Object.freeze({
  OLAP_SCAN: "OlapScan",
  CONNECTOR_SCAN: "ConnectorScan",
  HASH_JOIN: "HashJoin",
  AGGREGATE: "Aggregate",
  LIMIT: "Limit",
  EXCHANGE_SINK: "ExchangeSink",
  EXCHANGE_SOURCE: "ExchangeSource",
  RESULT_SINK: "ResultSink",
  CHUNK_ACCUMULATE: "ChunkAccumulate",
  SORT: "Sort",
  UNKNOWN: "Unknown",
});

/**
 * NodeType 显示标签（中文）
 */
export const NodeTypeLabels = {
  [NodeType.OLAP_SCAN]: "OLAP扫描",
  [NodeType.CONNECTOR_SCAN]: "连接器扫描",
  [NodeType.HASH_JOIN]: "哈希连接",
  [NodeType.AGGREGATE]: "聚合",
  [NodeType.LIMIT]: "限制",
  [NodeType.EXCHANGE_SINK]: "交换汇",
  [NodeType.EXCHANGE_SOURCE]: "交换源",
  [NodeType.RESULT_SINK]: "结果汇聚",
  [NodeType.CHUNK_ACCUMULATE]: "数据块累积",
  [NodeType.SORT]: "排序",
  [NodeType.UNKNOWN]: "未知",
};

/**
 * 获取 NodeType 的显示标签
 * @param {string} nodeType - NodeType 枚举值
 * @returns {string} 显示标签
 */
export function getNodeTypeLabel(nodeType) {
  return NodeTypeLabels[nodeType] || nodeType;
}

/**
 * 获取 NodeType 的图标
 * @param {string} nodeType - NodeType 枚举值
 * @returns {string} FontAwesome 图标类名
 */
export function getNodeTypeIcon(nodeType) {
  const iconMap = {
    [NodeType.OLAP_SCAN]: "fas fa-database",
    [NodeType.CONNECTOR_SCAN]: "fas fa-plug",
    [NodeType.HASH_JOIN]: "fas fa-code-branch",
    [NodeType.AGGREGATE]: "fas fa-layer-group",
    [NodeType.LIMIT]: "fas fa-hand-paper",
    [NodeType.EXCHANGE_SINK]: "fas fa-arrow-down",
    [NodeType.EXCHANGE_SOURCE]: "fas fa-arrow-up",
    [NodeType.RESULT_SINK]: "fas fa-flag-checkered",
    [NodeType.CHUNK_ACCUMULATE]: "fas fa-inbox",
    [NodeType.SORT]: "fas fa-sort-amount-down",
    [NodeType.UNKNOWN]: "fas fa-question-circle",
  };

  return iconMap[nodeType] || "fas fa-cube";
}

/**
 * 获取 NodeType 的颜色
 * @param {string} nodeType - NodeType 枚举值
 * @returns {string} 颜色值
 */
export function getNodeTypeColor(nodeType) {
  const colorMap = {
    [NodeType.OLAP_SCAN]: "#1890ff",
    [NodeType.CONNECTOR_SCAN]: "#13c2c2",
    [NodeType.HASH_JOIN]: "#722ed1",
    [NodeType.AGGREGATE]: "#52c41a",
    [NodeType.LIMIT]: "#faad14",
    [NodeType.EXCHANGE_SINK]: "#fa8c16",
    [NodeType.EXCHANGE_SOURCE]: "#fa541c",
    [NodeType.RESULT_SINK]: "#eb2f96",
    [NodeType.CHUNK_ACCUMULATE]: "#2f54eb",
    [NodeType.SORT]: "#fadb14",
    [NodeType.UNKNOWN]: "#8c8c8c",
  };

  return colorMap[nodeType] || "#8c8c8c";
}
