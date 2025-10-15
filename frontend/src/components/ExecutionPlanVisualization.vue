<template>
  <div class="execution-plan-viz">
    <div class="viz-controls">
      <el-button-group>
        <el-button
          size="small"
          :type="viewMode === 'tree' ? 'primary' : ''"
          @click="viewMode = 'tree'"
        >
          <i class="fas fa-project-diagram"></i> 树形视图
        </el-button>
        <el-button
          size="small"
          :type="viewMode === 'graph' ? 'primary' : ''"
          @click="viewMode = 'graph'"
        >
          <i class="fas fa-share-alt"></i> 图表视图
        </el-button>
      </el-button-group>

      <div class="control-actions">
        <el-button size="small" @click="resetZoom">
          <i class="fas fa-search"></i> 重置缩放
        </el-button>
        <el-checkbox v-model="showHotspots" @change="toggleHotspots">
          显示热点
        </el-checkbox>
        <el-checkbox v-model="showMetrics" @change="toggleMetrics">
          显示指标
        </el-checkbox>
      </div>
    </div>

    <div class="viz-container" ref="vizContainer">
      <svg
        v-if="viewMode === 'graph'"
        class="plan-graph"
        :width="containerWidth"
        :height="containerHeight"
      >
        <g class="zoom-container" ref="zoomContainer">
          <g class="links"></g>
          <g class="nodes"></g>
        </g>
      </svg>

      <div v-if="viewMode === 'tree'" class="plan-tree">
        <TreeNode
          v-for="node in treeNodes"
          :key="node.id"
          :node="node"
          :show-metrics="showMetrics"
          :highlight-hotspots="showHotspots"
          @node-click="onNodeClick"
        />
      </div>
    </div>

    <!-- Node Detail Modal -->
    <el-dialog
      v-model="detailVisible"
      :title="`Operator 详情: ${selectedNode?.operator_name || ''}`"
      width="70%"
      :before-close="closeDetailModal"
    >
      <div v-if="selectedNode">
        <el-descriptions :column="2" border>
          <el-descriptions-item label="节点ID">
            {{ selectedNode.id }}
          </el-descriptions-item>
          <el-descriptions-item label="操作符类型">
            {{ selectedNode.operator_name }}
          </el-descriptions-item>
          <el-descriptions-item label="节点类型">
            {{ getNodeTypeLabel(selectedNode.node_type) }}
          </el-descriptions-item>
          <el-descriptions-item label="深度">
            {{ selectedNode.depth }}
          </el-descriptions-item>
          <el-descriptions-item label="是否热点">
            <el-tag :type="selectedNode.is_hotspot ? 'danger' : 'success'">
              {{ selectedNode.is_hotspot ? '是' : '否' }}
            </el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="热点严重程度" v-if="selectedNode.is_hotspot">
            <span :class="`hotspot-${selectedNode.hotspot_severity.toLowerCase()}`">
              {{ getSeverityLabel(selectedNode.hotspot_severity) }}
            </span>
          </el-descriptions-item>
        </el-descriptions>

        <div class="metrics-section">
          <h4>性能指标</h4>
          <el-row :gutter="20">
            <el-col :span="12">
              <el-card size="small" shadow="hover">
                <template #header>通用指标</template>
                <div class="metric-grid">
                  <div class="metric-item">
                    <span class="metric-label">总耗时:</span>
                    <span class="metric-value">{{ formatDuration(selectedNode.metrics.operator_total_time) }}</span>
                  </div>
                  <div class="metric-item">
                    <span class="metric-label">推入数据块:</span>
                    <span class="metric-value">{{ selectedNode.metrics.push_chunk_num || 'N/A' }}</span>
                  </div>
                  <div class="metric-item">
                    <span class="metric-label">推入行数:</span>
                    <span class="metric-value">{{ selectedNode.metrics.push_row_num || 'N/A' }}</span>
                  </div>
                  <div class="metric-item">
                    <span class="metric-label">内存使用:</span>
                    <span class="metric-value">{{ formatBytes(selectedNode.metrics.memory_usage) }}</span>
                  </div>
                </div>
              </el-card>
            </el-col>

            <el-col :span="12" v-if="selectedNode.metrics.specialized">
              <el-card size="small" shadow="hover">
                <template #header>专业指标</template>
                <div class="metric-grid">
                  <template v-if="selectedNode.metrics.specialized.ConnectorScan">
                    <div class="metric-item">
                      <span class="metric-label">读取字节:</span>
                      <span class="metric-value">{{ formatBytes(selectedNode.metrics.specialized.ConnectorScan.bytes_read) }}</span>
                    </div>
                    <div class="metric-item">
                      <span class="metric-label">读取行数:</span>
                      <span class="metric-value">{{ selectedNode.metrics.specialized.ConnectorScan.rows_read || 'N/A' }}</span>
                    </div>
                    <div class="metric-item">
                      <span class="metric-label">IO时间:</span>
                      <span class="metric-value">{{ formatDuration(selectedNode.metrics.specialized.ConnectorScan.io_time) }}</span>
                    </div>
                  </template>
                </div>
              </el-card>
            </el-col>
          </el-row>
        </div>

        <div class="children-section" v-if="selectedNode.children && selectedNode.children.length > 0">
          <h4>子节点</h4>
          <el-tag
            v-for="childId in selectedNode.children"
            :key="childId"
            size="small"
            @click="focusOnNode(childId)"
          >
            {{ childId }}
          </el-tag>
        </div>
      </div>
    </el-dialog>
  </div>
</template>

<script>
import * as d3 from 'd3'
import TreeNode from './TreeNode.vue'

export default {
  name: 'ExecutionPlanVisualization',

  components: {
    TreeNode
  },

  props: {
    result: {
      type: Object,
      required: true
    }
  },

  data() {
    return {
      viewMode: 'tree', // 'tree' or 'graph'
      showHotspots: true,
      showMetrics: true,
      detailVisible: false,
      selectedNode: null,
      containerWidth: 800,
      containerHeight: 600,
      zoom: null,
      svg: null,
      simulation: null
    }
  },

  computed: {
    executionTree() {
      return this.result.execution_tree
    },

    treeNodes() {
      if (!this.executionTree) return []
      return this.buildTreeStructure(this.executionTree.root, this.executionTree.nodes)
    }
  },

  mounted() {
    this.updateContainerSize()
    this.initGraphView()
    window.addEventListener('resize', this.updateContainerSize)
  },

  beforeUnmount() {
    window.removeEventListener('resize', this.updateContainerSize)
    this.destroyGraph()
  },

  watch: {
    viewMode() {
      this.$nextTick(() => {
        if (this.viewMode === 'graph') {
          this.renderGraph()
        }
      })
    },

    showHotspots() {
      this.updateGraph()
    },

    showMetrics() {
      this.updateGraph()
    }
  },

  methods: {
    updateContainerSize() {
      if (this.$refs.vizContainer) {
        const rect = this.$refs.vizContainer.getBoundingClientRect()
        this.containerWidth = rect.width
        this.containerHeight = rect.height
      }
    },

    initGraphView() {
      if (this.viewMode === 'graph') {
        this.renderGraph()
      }
    },

    renderGraph() {
      if (!this.executionTree) return

      this.destroyGraph()

      const svg = d3.select('.plan-graph')
      this.svg = svg

      const zoomContainer = svg.select('.zoom-container')
      this.zoom = d3.zoom()
        .scaleExtent([0.1, 4])
        .on('zoom', (event) => {
          zoomContainer.attr('transform', event.transform)
        })

      svg.call(this.zoom)

      this.simulation = d3.forceSimulation(this.executionTree.nodes)
        .force('link', d3.forceLink(this.buildLinks()).id(d => d.id).distance(120))
        .force('charge', d3.forceManyBody().strength(-400))
        .force('center', d3.forceCenter(this.containerWidth / 2, this.containerHeight / 2))

      this.updateGraph()
    },

    buildLinks() {
      const links = []
      if (!this.executionTree) return links

      this.executionTree.nodes.forEach(node => {
        node.children.forEach(childId => {
          links.push({
            source: node.id,
            target: childId
          })
        })
      })

      return links
    },

    updateGraph() {
      if (!this.simulation || !this.svg) return

      const zoomContainer = this.svg.select('.zoom-container')

      // Update links
      const link = zoomContainer.select('.links')
        .selectAll('.link')
        .data(this.buildLinks())

      link.enter()
        .append('line')
        .attr('class', 'link')
        .merge(link)
        .attr('stroke', '#999')
        .attr('stroke-opacity', 0.6)
        .attr('stroke-width', 2)

      link.exit().remove()

      // Update nodes
      const node = zoomContainer.select('.nodes')
        .selectAll('.node')
        .data(this.executionTree.nodes, d => d.id)

      const nodeEnter = node.enter()
        .append('g')
        .attr('class', 'node')
        .call(d3.drag()
          .on('start', (event, d) => {
            if (!event.active) this.simulation.alphaTarget(0.3).restart()
            d.fx = d.x
            d.fy = d.y
          })
          .on('drag', (event, d) => {
            d.fx = event.x
            d.fy = event.y
          })
          .on('end', (event, d) => {
            if (!event.active) this.simulation.alphaTarget(0)
            d.fx = null
            d.fy = null
          })
        )
        .on('click', (event, d) => this.onNodeClick(d))

      // Add circles
      nodeEnter.append('circle')
        .attr('r', d => this.getNodeRadius(d))
        .attr('fill', d => this.getNodeColor(d))
        .attr('stroke', d => d.is_hotspot ? '#f5222d' : '#1890ff')
        .attr('stroke-width', d => d.is_hotspot ? 3 : 1)

      // Add labels
      nodeEnter.append('text')
        .attr('dy', '.35em')
        .attr('text-anchor', 'middle')
        .style('font-size', '12px')
        .style('font-weight', 'bold')
        .text(d => d.operator_name)

      // Add metrics labels if enabled
      if (this.showMetrics) {
        nodeEnter.append('text')
          .attr('dy', '1.5em')
          .attr('text-anchor', 'middle')
          .style('font-size', '10px')
          .style('fill', '#666')
          .text(d => this.getNodeMetricText(d))
      }

      const nodeMerge = nodeEnter.merge(node)

      // Update node positions
      this.simulation.on('tick', () => {
        zoomContainer.selectAll('.link')
          .attr('x1', d => d.source.x)
          .attr('y1', d => d.source.y)
          .attr('x2', d => d.target.x)
          .attr('y2', d => d.target.y)

        zoomContainer.selectAll('.node')
          .attr('transform', d => `translate(${d.x},${d.y})`)
      })

      this.simulation.restart()
    },

    buildTreeStructure(root, nodes) {
      // Build tree structure for tree view
      return this.buildTree(root, nodes, 0)
    },

    buildTree(nodeId, nodes, depth) {
      const node = nodes.find(n => n.id === nodeId)
      if (!node) return []

      const treeNode = {
        ...node,
        depth,
        children: []
      }

      node.children.forEach(childId => {
        treeNode.children.push(...this.buildTree(childId, nodes, depth + 1))
      })

      return [treeNode]
    },

    getNodeRadius(node) {
      const baseRadius = 25
      if (!this.showMetrics || !node.metrics.operator_total_time) return baseRadius

      // Scale radius by timing (larger = slower)
      const scale = Math.min(2, Math.max(0.5, node.metrics.operator_total_time.as_secs_f64() / 100.0))
      return baseRadius * scale
    },

    getNodeColor(node) {
      if (this.showHotspots && node.is_hotspot) {
        const severity = node.hotspot_severity.toLowerCase()
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

      return typeColors[node.node_type] || '#1890ff'
    },

    getNodeMetricText(node) {
      if (!node.metrics.operator_total_time) return ''
      return `${node.metrics.operator_total_time.as_secs_f64().toFixed(2)}s`
    },

    onNodeClick(node) {
      this.selectedNode = node
      this.detailVisible = true
    },

    closeDetailModal() {
      this.detailVisible = false
      this.selectedNode = null
    },

    focusOnNode(nodeId) {
      // Focus graph on specific node
      const node = this.executionTree.nodes.find(n => n.id === nodeId)
      if (node && this.zoom && this.svg) {
        this.svg.transition().duration(750).call(
          this.zoom.transform,
          d3.zoomIdentity.translate(this.containerWidth / 2 - node.x, this.containerHeight / 2 - node.y)
        )
      }
    },

    toggleHotspots() {
      this.updateGraph()
    },

    toggleMetrics() {
      this.updateGraph()
    },

    resetZoom() {
      if (this.zoom && this.svg) {
        this.svg.transition().duration(750).call(
          this.zoom.transform,
          d3.zoomIdentity
        )
      }
    },

    destroyGraph() {
      if (this.simulation) {
        this.simulation.stop()
        this.simulation = null
      }
      if (this.svg) {
        this.svg.selectAll('*').remove()
      }
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

    getSeverityLabel(severity) {
      const labels = {
        Normal: '正常',
        Mild: '轻微',
        Moderate: '中等',
        Severe: '严重',
        Critical: '严重',
        High: '高'
      }
      return labels[severity] || severity
    },

    formatDuration(duration) {
      if (!duration) return 'N/A'
      const secs = duration.as_secs_f64()
      if (secs < 1) return `${(secs * 1000).toFixed(1)}ms`
      return `${secs.toFixed(2)}s`
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
.execution-plan-viz {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.viz-controls {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
  padding: 12px 16px;
  background: #fafafa;
  border-radius: 6px;
}

.control-actions {
  display: flex;
  gap: 12px;
}

.viz-container {
  flex: 1;
  min-height: 400px;
  border: 1px solid #e4e7ed;
  border-radius: 6px;
  overflow: hidden;
}

.plan-graph {
  width: 100%;
  height: 100%;
  background: #fafafa;
}

.plan-tree {
  padding: 20px;
  height: 100%;
  overflow: auto;
}

:deep(.el-dialog__body) {
  padding: 20px;
}

.metrics-section h4,
.children-section h4 {
  margin-bottom: 16px;
  font-weight: 600;
  color: #303133;
}

.metric-grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 8px;
}

.metric-item {
  display: flex;
  justify-content: space-between;
  padding: 4px 0;
  border-bottom: 1px solid #f0f0f0;
}

.metric-label {
  font-weight: 500;
  color: #606266;
}

.metric-value {
  color: #303133;
}

.children-section .el-tag {
  margin: 4px 4px 4px 0;
  cursor: pointer;
}

.children-section .el-tag:hover {
  opacity: 0.8;
}
</style>
