<template>
  <div
    class="tree-node"
    :style="{ marginLeft: `${depth * 40}px` }"
  >
    <div
      class="node-container"
      :class="{ 'node-hotspot': highlightHotspots && node.is_hotspot }"
      @click="handleNodeClick"
    >
      <div class="node-header">
        <div class="node-icon">
          <i
            :class="getNodeIcon(node.node_type)"
            :style="{ color: getNodeColor() }"
          ></i>
        </div>

        <div class="node-info">
          <div class="node-name">{{ node.operator_name }}</div>
          <div class="node-type">{{ getNodeTypeLabel(node.node_type) }}</div>
        </div>

        <div v-if="node.is_hotspot" class="hotspot-badge">
          <span
            class="severity-dot"
            :class="`hotspot-${node.hotspot_severity.toLowerCase()}`"
          >
            {{ getSeverityAbbrev(node.hotspot_severity) }}
          </span>
        </div>
      </div>

      <div v-if="showMetrics" class="node-metrics">
        <div class="metric-row">
          <span class="metric-label">耗时:</span>
          <span class="metric-value">{{ formatDuration(node.metrics.operator_total_time) }}</span>
        </div>
        <div class="metric-row">
          <span class="metric-label">行数:</span>
          <span class="metric-value">{{ node.metrics.push_row_num || 'N/A' }}</span>
        </div>
        <div class="metric-row">
          <span class="metric-label">内存:</span>
          <span class="metric-value">{{ formatBytes(node.metrics.memory_usage) }}</span>
        </div>
      </div>

      <div v-if="expanded && node.children && node.children.length > 0" class="node-children">
        <!-- Recursively render child nodes -->
        <TreeNode
          v-for="child in node.children"
          :key="child.id"
          :node="child"
          :depth="child.depth"
          :show-metrics="showMetrics"
          :highlight-hotspots="highlightHotspots"
          :expanded="true"
          @node-click="handleChildClick"
        />
      </div>
    </div>
  </div>
</template>

<script>
export default {
  name: 'TreeNode',

  props: {
    node: {
      type: Object,
      required: true
    },
    showMetrics: {
      type: Boolean,
      default: true
    },
    highlightHotspots: {
      type: Boolean,
      default: true
    },
    expanded: {
      type: Boolean,
      default: true
    },
    depth: {
      type: Number,
      default: 0
    }
  },

  emits: ['node-click'],

  methods: {
    handleNodeClick() {
      this.$emit('node-click', this.node)
    },

    handleChildClick(node) {
      // Forward child node clicks to parent
      this.$emit('node-click', node)
    },

    getNodeIcon(nodeType) {
      const icons = {
        OlapScan: 'fas fa-database',
        ConnectorScan: 'fas fa-plug',
        HashJoin: 'fas fa-link',
        Aggregate: 'fas fa-calculator',
        Limit: 'fas fa-filter',
        ExchangeSink: 'fas fa-sign-out-alt',
        ExchangeSource: 'fas fa-sign-in-alt',
        ResultSink: 'fas fa-flag-checkered',
        ChunkAccumulate: 'fas fa-layer-group',
        Sort: 'fas fa-sort',
        Unknown: 'fas fa-question-circle'
      }
      return icons[nodeType] || 'fas fa-cog'
    },

    getNodeColor() {
      if (this.highlightHotspots && this.node.is_hotspot) {
        const severity = this.node.hotspot_severity.toLowerCase()
        const colors = {
          normal: '#52c41a',
          mild: '#faad14',
          moderate: '#fa8c16',
          severe: '#fa541a',
          critical: '#f5222d',
          high: '#722ed1'
        }
        return colors[severity] || '#1890ff'
      }

      const typeColors = {
        OlapScan: '#36cfc9',
        ConnectorScan: '#95de64',
        HashJoin: '#69c0ff',
        Aggregate: '#b37feb',
        Limit: '#f759ab',
        ExchangeSink: '#ff9c6e',
        ExchangeSource: '#ff7875',
        ResultSink: '#d3adf7',
        Unknown: '#d9d9d9'
      }

      return typeColors[this.node.node_type] || '#1890ff'
    },

    getNodeTypeLabel(nodeType) {
      const labels = {
        OlapScan: 'OLAP 扫描',
        ConnectorScan: '连接器扫描',
        HashJoin: '哈希连接',
        Aggregate: '聚合',
        Limit: '限制',
        ExchangeSink: '交换接收',
        ExchangeSource: '交换源',
        ResultSink: '结果接收',
        ChunkAccumulate: '数据块累积',
        Sort: '排序',
        Unknown: '未知'
      }
      return labels[nodeType] || nodeType
    },

    getSeverityAbbrev(severity) {
      const abbrevs = {
        Normal: '正',
        Mild: '轻',
        Moderate: '中',
        Severe: '重',
        Critical: '严',
        High: '高'
      }
      return abbrevs[severity] || '热'
    },

    formatDuration(duration) {
      if (!duration) return 'N/A'
      // Handle both Duration objects (with as_secs_f64 method) and numeric values
      let seconds
      if (typeof duration === 'object' && duration.as_secs_f64) {
        seconds = duration.as_secs_f64()
      } else if (typeof duration === 'number') {
        // If it's already a number, assume it's in milliseconds
        seconds = duration / 1000
      } else if (typeof duration === 'string') {
        return duration
      } else {
        return 'N/A'
      }
      if (seconds < 1) return `${(seconds * 1000).toFixed(1)}ms`
      return `${seconds.toFixed(2)}s`
    },

    formatBytes(bytes) {
      if (!bytes) return 'N/A'
      const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
      if (bytes === 0) return '0 B'
      const i = Math.floor(Math.log(bytes) / Math.log(1024))
      return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${sizes[i]}`
    }
  }
}
</script>

<style scoped>
.tree-node {
  margin-bottom: 8px;
}

.node-container {
  border: 2px solid #e4e7ed;
  border-radius: 8px;
  background: white;
  cursor: pointer;
  transition: all 0.2s ease;
}

.node-container:hover {
  border-color: #409eff;
  box-shadow: 0 2px 12px rgba(64, 158, 255, 0.1);
}

.node-hotspot {
  border-color: #f5222d;
  background: linear-gradient(135deg, #fff1f0 0%, #ffe7e7 100%);
}

.node-hotspot:hover {
  border-color: #ff7875;
  box-shadow: 0 2px 12px rgba(245, 34, 45, 0.2);
}

.node-header {
  display: flex;
  align-items: center;
  padding: 12px 16px;
  gap: 12px;
}

.node-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  background: #f5f7fa;
  border-radius: 6px;
}

.node-icon i {
  font-size: 20px;
}

.node-info {
  flex: 1;
}

.node-name {
  font-weight: 600;
  font-size: 14px;
  color: #303133;
  margin-bottom: 2px;
}

.node-type {
  font-size: 12px;
  color: #909399;
}

.hotspot-badge {
  flex-shrink: 0;
}

.severity-dot {
  display: inline-block;
  width: 24px;
  height: 24px;
  border-radius: 50%;
  text-align: center;
  line-height: 24px;
  font-weight: bold;
  color: white;
  font-size: 10px;
}

.node-metrics {
  padding: 8px 16px 12px;
  border-top: 1px solid #f0f2f5;
  background: #fafbfc;
}

.metric-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 2px 0;
}

.metric-row:not(:last-child) {
  margin-bottom: 4px;
}

.metric-label {
  font-size: 12px;
  color: #606266;
  font-weight: 500;
}

.metric-value {
  font-size: 12px;
  color: #303133;
  font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
}
</style>
