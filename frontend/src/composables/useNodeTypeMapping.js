/**
 * NodeType 映射组合式函数
 * 提供通用的 NodeType 映射逻辑，移除硬编码
 */

import { NodeType, getNodeTypeLabel, getNodeTypeIcon, getNodeTypeColor } from '@/models/NodeType';

export function useNodeTypeMapping() {
  /**
   * 判断是否为 Scan 类型
   */
  const isScanNode = (nodeType) => {
    return [NodeType.OLAP_SCAN, NodeType.CONNECTOR_SCAN].includes(nodeType);
  };

  /**
   * 判断是否为 Join 类型
   */
  const isJoinNode = (nodeType) => {
    return nodeType === NodeType.HASH_JOIN;
  };

  /**
   * 判断是否为 Exchange 类型
   */
  const isExchangeNode = (nodeType) => {
    return [NodeType.EXCHANGE_SINK, NodeType.EXCHANGE_SOURCE].includes(nodeType);
  };

  /**
   * 判断是否为 Sink 类型
   */
  const isSinkNode = (nodeType) => {
    return [NodeType.EXCHANGE_SINK, NodeType.RESULT_SINK].includes(nodeType);
  };

  return {
    NodeType,
    getNodeTypeLabel,
    getNodeTypeIcon,
    getNodeTypeColor,
    isScanNode,
    isJoinNode,
    isExchangeNode,
    isSinkNode,
  };
}

