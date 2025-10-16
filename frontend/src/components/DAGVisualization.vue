<template>
  <div class="dag-wrapper">
    <!-- 工具栏 -->
    <div class="dag-toolbar">
      <div class="toolbar-left">
        <el-button-group>
          <el-button @click="resetView" size="small">
            <i class="fas fa-home"></i> 重置
          </el-button>
          <el-button @click="zoomIn" size="small">
            <i class="fas fa-plus"></i>
          </el-button>
          <el-button @click="zoomOut" size="small">
            <i class="fas fa-minus"></i>
          </el-button>
          <el-button @click="fitToScreen" size="small">
            <i class="fas fa-compress"></i>
          </el-button>
        </el-button-group>
      </div>
      <div class="zoom-level">{{ (zoom * 100).toFixed(0) }}%</div>
    </div>

    <!-- 主容器 -->
    <div class="dag-main">
      <!-- SVG 画布 -->
      <div class="dag-canvas-wrapper">
        <svg 
          ref="dagSvg"
          class="dag-svg"
          :width="svgWidth"
          :height="svgHeight"
          @wheel.prevent="handleWheel"
          @mousedown="startPan"
          @mousemove="doPan"
          @mouseup="endPan"
          @mouseleave="endPan"
        >
          <defs>
            <marker
              id="arrow"
              markerWidth="10"
              markerHeight="10"
              refX="9"
              refY="3"
              orient="auto"
            >
              <polygon points="0 0, 10 3, 0 6" fill="#999" />
            </marker>
            <marker
              id="arrow-red"
              markerWidth="10"
              markerHeight="10"
              refX="9"
              refY="3"
              orient="auto"
            >
              <polygon points="0 0, 10 3, 0 6" fill="#ff6b6b" />
            </marker>
          </defs>

          <!-- 背景网格 -->
          <defs>
            <pattern id="grid" width="20" height="20" patternUnits="userSpaceOnUse">
              <path d="M 20 0 L 0 0 0 20" fill="none" stroke="#f0f0f0" stroke-width="0.5"/>
            </pattern>
          </defs>
          <rect width="100%" height="100%" fill="url(#grid)" />

          <!-- 缩放组 -->
          <g :transform="`translate(${panX}, ${panY}) scale(${zoom})`" class="zoom-group">
            <!-- 连接线 -->
            <g class="lines">
              <path
                v-for="link in links"
                :key="`line-${link.id}`"
                :d="link.path"
                class="connection-line"
                :class="{ 'connection-hotspot': link.isHotspot }"
                :marker-end="`url(#${link.isHotspot ? 'arrow-red' : 'arrow'})`"
              />
              
              <!-- 行数标签 -->
              <text
                v-for="link in links"
                :key="`label-${link.id}`"
                :x="link.labelX"
                :y="link.labelY"
                class="row-count-label"
              >
                {{ link.label }}
              </text>
            </g>

            <!-- 节点 -->
            <g class="nodes">
              <g
                v-for="node in nodes"
                :key="node.id"
                :transform="`translate(${node.x}, ${node.y})`"
                class="node-group"
                :class="{ selected: selectedNodeId === node.id }"
                @click="selectNode(node)"
              >
                <!-- 节点背景 -->
                <rect
                  class="node-rect"
                  :class="{ 'node-hotspot': node.is_hotspot }"
                  :width="NODE_WIDTH"
                  :height="NODE_HEIGHT"
                  rx="4"
                  ry="4"
                />

                <!-- 热点指示 -->
                <circle
                  v-if="node.is_hotspot"
                  class="hotspot-badge"
                  cx="8"
                  cy="8"
                  r="4"
                />

                <!-- 操作符名称 -->
                <text class="node-title" x="10" y="22">
                  {{ node.operator_name }}
                </text>

                <!-- plan_node_id -->
                <text class="node-info" x="10" y="38">
                  plan_node_id={{ node.plan_node_id }}
                </text>

                <!-- 执行时间 -->
                <text class="node-info" x="10" y="54">
                  耗时: {{ node.metrics ? formatDuration(node.metrics.operator_total_time) : 'N/A' }}
                </text>

                <!-- 性能百分比 -->
                <text class="node-percentage" :x="NODE_WIDTH - 10" y="32">
                  {{ getPercentage(node) }}%
                </text>
              </g>
            </g>
          </g>
        </svg>
      </div>

      <!-- 右侧详情面板 -->
      <div class="detail-panel" v-if="selectedNode">
        <!-- 标签页 -->
        <el-tabs class="detail-tabs" value="overview">
          <el-tab-pane :label="selectedNode.is_summary ? '查询概览' : '节点详情'" name="overview">
            <div class="detail-content">
              <!-- 查询摘要 -->
              <template v-if="selectedNode.is_summary">
                <div class="info-section">
                  <h4>执行统计</h4>
                  <div class="info-grid">
                    <div class="info-item" v-if="selectedNode.summary_data.total_time">
                      <span class="label">执行总时间:</span>
                      <span class="value">{{ selectedNode.summary_data.total_time }}</span>
                    </div>
                    <div class="info-item" v-if="selectedNode.summary_data.query_state">
                      <span class="label">查询状态:</span>
                      <span class="value">{{ selectedNode.summary_data.query_state }}</span>
                    </div>
                    <div class="info-item" v-if="selectedNode.summary_data.starrocks_version">
                      <span class="label">StarRocks版本:</span>
                      <span class="value">{{ selectedNode.summary_data.starrocks_version }}</span>
                    </div>
                    <div class="info-item" v-if="selectedNode.summary_data.query_id">
                      <span class="label">查询ID:</span>
                      <span class="value" style="font-size: 12px; word-break: break-all;">{{ selectedNode.summary_data.query_id }}</span>
                    </div>
                  </div>
                </div>
              </template>
              <!-- 节点详情 -->
              <template v-else>
                <div class="info-section">
                  <h4>{{ selectedNode.operator_name }}</h4>
                  <div class="info-grid">
                    <div class="info-item">
                      <span class="label">Plan Node ID:</span>
                      <span class="value">{{ selectedNode.plan_node_id }}</span>
                    </div>
                    <div class="info-item">
                      <span class="label">Node Type:</span>
                      <span class="value">{{ selectedNode.node_type }}</span>
                    </div>
                    <div class="info-item">
                      <span class="label">执行耗时:</span>
                      <span class="value" style="color: #f5222d; font-weight: bold;">
                        {{ formatDuration(selectedNode.metrics.operator_total_time) }}
                      </span>
                    </div>
                    <div class="info-item">
                      <span class="label">性能占比:</span>
                      <span class="value">{{ getPercentage(selectedNode) }}%</span>
                    </div>
                  </div>
                </div>

                <!-- 数据流指标 -->
                <div class="metrics-section">
                  <h5>数据流</h5>
                  <div class="metrics-grid">
                    <div class="metric">
                      <span class="metric-label">推入数据块</span>
                      <span class="metric-value">{{ selectedNode.metrics.push_chunk_num || 'N/A' }}</span>
                    </div>
                    <div class="metric">
                      <span class="metric-label">推入行数</span>
                      <span class="metric-value">{{ selectedNode.metrics.push_row_num || 'N/A' }}</span>
                    </div>
                    <div class="metric">
                      <span class="metric-label">拉取数据块</span>
                      <span class="metric-value">{{ selectedNode.metrics.pull_chunk_num || 'N/A' }}</span>
                    </div>
                    <div class="metric">
                      <span class="metric-label">拉取行数</span>
                      <span class="metric-value">{{ selectedNode.metrics.pull_row_num || 'N/A' }}</span>
                    </div>
                    <div class="metric">
                      <span class="metric-label">输出字节</span>
                      <span class="metric-value">{{ formatBytes(selectedNode.metrics.output_chunk_bytes) }}</span>
                    </div>
                  </div>
                </div>

                <!-- 性能指标 -->
                <div class="metrics-section">
                  <h5>性能指标</h5>
                  <div class="metrics-grid">
                    <div class="metric">
                      <span class="metric-label">内存使用</span>
                      <span class="metric-value">{{ formatBytes(selectedNode.metrics.memory_usage) }}</span>
                    </div>
                    <div class="metric">
                      <span class="metric-label">推送总时间</span>
                      <span class="metric-value">{{ formatDuration(selectedNode.metrics.push_total_time) }}</span>
                    </div>
                    <div class="metric">
                      <span class="metric-label">拉取总时间</span>
                      <span class="metric-value">{{ formatDuration(selectedNode.metrics.pull_total_time) }}</span>
                    </div>
                  </div>
                </div>

                <!-- 热点提示 -->
                <div v-if="selectedNode.is_hotspot" class="hotspot-alert">
                  <i class="fas fa-exclamation-circle"></i>
                  <span>热点检测: {{ selectedNode.hotspot_severity }}</span>
                </div>
              </template>
            </div>
          </el-tab-pane>

          <el-tab-pane label="分析建议" name="suggestions">
            <div class="detail-content">
              <div v-if="selectedNode.is_hotspot" class="suggestions">
                <el-alert
                  title="性能瓶颈检测"
                  type="warning"
                  :closable="false"
                  show-icon
                />
              </div>
              <div v-else class="suggestions">
                <el-alert
                  title="该算子执行正常"
                  type="success"
                  :closable="false"
                  show-icon
                />
              </div>
            </div>
          </el-tab-pane>
        </el-tabs>
      </div>
    </div>
  </div>
</template>

<script>
export default {
  name: 'DAGVisualization',
  
  props: {
    executionTree: {
      type: Object,
      required: true
    },
    summary: {
      type: Object,
      required: false,
      default: null
    }
  },

  data() {
    return {
      NODE_WIDTH: 140,
      NODE_HEIGHT: 60,
      
      nodes: [],
      links: [],
      
      zoom: 1,
      panX: 20,
      panY: 20,
      
      isPanning: false,
      panStartX: 0,
      panStartY: 0,
      
      svgWidth: window.innerWidth - 400,
      svgHeight: 600,
      
      selectedNodeId: null,
      selectedNode: null,
      
      maxTime: 0
    }
  },

  watch: {
    executionTree: {
      handler() {
        this.renderDAG()
      },
      immediate: true
    }
  },

  mounted() {
    window.addEventListener('resize', this.onWindowResize)
    // 初始化显示summary作为detail panel的内容
    if (this.summary) {
      this.selectedNode = {
        operator_name: 'Query Overview',
        is_summary: true,
        summary_data: this.summary
      }
    }
  },

  beforeUnmount() {
    window.removeEventListener('resize', this.onWindowResize)
  },

  methods: {
    renderDAG() {
      if (!this.executionTree || !this.executionTree.nodes || !this.executionTree.nodes.length) return

      const nodeMap = new Map()
      const nodesByDepth = new Map()

      // 按深度分组
      this.executionTree.nodes.forEach(node => {
        if (!node || !node.metrics) return
        nodeMap.set(node.id, node)
        const depth = node.depth || 0
        if (!nodesByDepth.has(depth)) {
          nodesByDepth.set(depth, [])
        }
        nodesByDepth.get(depth).push(node)
      })

      // 计算最大执行时间
      let maxTime = 0
      this.executionTree.nodes.forEach(node => {
        if (!node || !node.metrics) return
        const time = this.getDurationMs(node.metrics.operator_total_time)
        maxTime = Math.max(maxTime, time)
      })
      this.maxTime = maxTime || 1

      // 计算节点位置 - 垂直布局（从上到下）
      const levelHeight = 150  // 垂直间距
      const levelWidth = 200   // 水平间距
      
      // 计算最大深度以确定SVG高度
      let maxDepth = 0
      this.executionTree.nodes.forEach(node => {
        maxDepth = Math.max(maxDepth, node.depth || 0)
      })
      
      // 动态调整SVG高度
      this.svgHeight = Math.max(600, (maxDepth + 1) * levelHeight + 150)
      
      this.nodes = this.executionTree.nodes.map(node => {
        const depth = node.depth || 0
        const levelNodes = nodesByDepth.get(depth) || []
        const indexInLevel = levelNodes.indexOf(node)
        const nodesCountInLevel = levelNodes.length
        
        // 垂直布局：y根据depth增加，x根据同深度内的位置计算（水平居中）
        const y = depth * levelHeight + 50
        const totalWidth = (nodesCountInLevel - 1) * levelWidth
        const centerX = this.svgWidth / 2
        const x = centerX - totalWidth / 2 + indexInLevel * levelWidth
        
        return {
          ...node,
          x,
          y
        }
      })

      // 构建连接线
      this.links = []
      this.executionTree.nodes.forEach((sourceNode, sourceIdx) => {
        if (!sourceNode || !sourceNode.children) return
        sourceNode.children.forEach((childId) => {
          const targetNode = nodeMap.get(childId)
          if (targetNode && targetNode.metrics) {
            const source = this.nodes[sourceIdx]
            const target = this.nodes.find(n => n.id === targetNode.id)
            
            if (source && target && source.metrics && target.metrics) {
              // 反向箭头：从target指向source（从下往上）
              const startX = target.x + this.NODE_WIDTH / 2
              const startY = target.y + this.NODE_HEIGHT
              const endX = source.x + this.NODE_WIDTH / 2
              const endY = source.y
              
              const controlY = (startY + endY) / 2
              const path = `M ${startX} ${startY} C ${startX} ${controlY}, ${endX} ${controlY}, ${endX} ${endY}`
              
              this.links.push({
                id: `${source.id}-${target.id}`,
                path,
                labelX: (startX + endX) / 2,
                labelY: controlY - 5,
                label: this.formatRows(targetNode.metrics.push_row_num || 0),
                isHotspot: (source.is_hotspot || false) || (target.is_hotspot || false)
              })
            }
          }
        })
      })
    },

    formatDuration(duration) {
      if (!duration) return 'N/A'
      const ms = this.getDurationMs(duration)
      if (ms < 1000) return `${ms.toFixed(1)}ms`
      return `${(ms / 1000).toFixed(2)}s`
    },

    getDurationMs(duration) {
      if (!duration) return 0
      if (typeof duration === 'number') return duration
      if (typeof duration === 'object' && duration.as_secs_f64 && typeof duration.as_secs_f64 === 'function') {
        return duration.as_secs_f64() * 1000
      }
      return 0
    },

    formatBytes(bytes) {
      if (!bytes) return 'N/A'
      const units = ['B', 'KB', 'MB', 'GB', 'TB']
      const index = Math.floor(Math.log(bytes) / Math.log(1024))
      return `${(bytes / Math.pow(1024, index)).toFixed(2)} ${units[index]}`
    },

    formatRows(rows) {
      if (!rows) return 'Rows: 0'
      if (rows > 1000000) return `Rows: ${(rows / 1000000).toFixed(1)}M`
      if (rows > 1000) return `Rows: ${(rows / 1000).toFixed(1)}K`
      return `Rows: ${rows}`
    },

    getPercentage(node) {
      if (!node || !node.metrics) return 0
      const nodeTime = this.getDurationMs(node.metrics.operator_total_time)
      if (nodeTime === 0 || this.maxTime === 0) return 0
      return ((nodeTime / this.maxTime) * 100).toFixed(2)
    },

    selectNode(node) {
      this.selectedNodeId = node.id
      this.selectedNode = node
    },

    resetView() {
      this.zoom = 1
      this.panX = 20
      this.panY = 20
    },

    zoomIn() {
      this.zoom = Math.min(this.zoom + 0.2, 3)
    },

    zoomOut() {
      this.zoom = Math.max(this.zoom - 0.2, 0.2)
    },

    fitToScreen() {
      this.zoom = 1
      this.resetView()
    },

    handleWheel(e) {
      if (e.deltaY < 0) {
        this.zoomIn()
      } else {
        this.zoomOut()
      }
    },

    startPan(e) {
      if (e.button !== 0) return
      this.isPanning = true
      this.panStartX = e.clientX - this.panX
      this.panStartY = e.clientY - this.panY
    },

    doPan(e) {
      if (!this.isPanning) return
      this.panX = e.clientX - this.panStartX
      this.panY = e.clientY - this.panStartY
    },

    endPan() {
      this.isPanning = false
    },

    onWindowResize() {
      this.svgWidth = window.innerWidth - 400
    }
  }
}
</script>

<style scoped>
.dag-wrapper {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: #fafafa;
}

.dag-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  background: white;
  border-bottom: 1px solid #e8e8e8;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.05);
}

.toolbar-left {
  display: flex;
  gap: 8px;
}

.zoom-level {
  font-size: 12px;
  color: #999;
}

.dag-main {
  display: flex;
  flex: 1;
  gap: 0;
}

.dag-canvas-wrapper {
  flex: 1;
  background: white;
  border-right: 1px solid #e8e8e8;
  overflow: hidden;
  position: relative;
}

.dag-svg {
  width: 100%;
  height: 100%;
  cursor: grab;
  background: white;
}

.dag-svg:active {
  cursor: grabbing;
}

.zoom-group {
  pointer-events: all;
}

/* 连接线 */
.connection-line {
  stroke: #bfbfbf;
  stroke-width: 1.5;
  fill: none;
  stroke-linecap: round;
  transition: stroke 0.2s;
}

.connection-line:hover {
  stroke: #595959;
  stroke-width: 2;
}

.connection-hotspot {
  stroke: #ff6b6b !important;
}

.row-count-label {
  font-size: 11px;
  fill: #666;
  pointer-events: none;
  text-anchor: middle;
}

/* 节点 */
.node-group {
  cursor: pointer;
  transition: all 0.2s;
}

.node-group:hover .node-rect {
  filter: drop-shadow(0 2px 4px rgba(0, 0, 0, 0.1));
}

.node-group.selected .node-rect {
  stroke: #1890ff !important;
  stroke-width: 2 !important;
  filter: drop-shadow(0 2px 8px rgba(24, 144, 255, 0.2));
}

.node-rect {
  fill: #f5f5f5;
  stroke: #d9d9d9;
  stroke-width: 1;
  transition: all 0.2s;
}

.node-hotspot {
  fill: #ffe7e6 !important;
  stroke: #ff6b6b !important;
  stroke-width: 2 !important;
}

.hotspot-badge {
  fill: #ff6b6b;
  animation: pulse-hot 1.5s infinite;
}

@keyframes pulse-hot {
  0%, 100% { opacity: 1; r: 4; }
  50% { opacity: 0.6; r: 5; }
}

.node-title {
  font-size: 12px;
  font-weight: 600;
  fill: #333;
  pointer-events: none;
}

.node-info {
  font-size: 10px;
  fill: #666;
  pointer-events: none;
}

.node-percentage {
  font-size: 13px;
  font-weight: bold;
  fill: #f5222d;
  pointer-events: none;
  text-anchor: end;
}

/* 右侧详情面板 */
.detail-panel {
  width: 380px;
  background: white;
  border-left: 1px solid #e8e8e8;
  overflow-y: auto;
  box-shadow: -1px 0 2px rgba(0, 0, 0, 0.05);
}

.detail-tabs {
  height: 100%;
  display: flex;
  flex-direction: column;
}

:deep(.el-tabs__header) {
  margin: 0;
  border-bottom: 1px solid #e8e8e8;
}

:deep(.el-tabs__content) {
  flex: 1;
  overflow-y: auto;
}

.detail-content {
  padding: 16px;
}

.info-section {
  margin-bottom: 20px;
}

.info-section h4 {
  margin: 0 0 12px 0;
  font-size: 14px;
  font-weight: 600;
  color: #333;
}

.info-grid {
  display: grid;
  gap: 8px;
}

.info-item {
  display: flex;
  justify-content: space-between;
  font-size: 12px;
}

.info-item .label {
  color: #666;
}

.info-item .value {
  color: #333;
  font-weight: 500;
}

.metrics-section {
  margin-bottom: 20px;
}

.metrics-section h5 {
  margin: 0 0 12px 0;
  font-size: 12px;
  font-weight: 600;
  color: #333;
  text-transform: uppercase;
}

.metrics-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
}

.metric {
  padding: 8px;
  background: #f9f9f9;
  border-radius: 4px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.metric-label {
  font-size: 11px;
  color: #999;
}

.metric-value {
  font-size: 12px;
  font-weight: 600;
  color: #333;
}

.hotspot-alert {
  padding: 12px;
  background: #ffe7e6;
  border: 1px solid #ff6b6b;
  border-radius: 4px;
  display: flex;
  align-items: center;
  gap: 8px;
  color: #ff6b6b;
  font-size: 12px;
  margin-top: 12px;
}

.suggestions {
  padding: 16px 0;
}
</style>
