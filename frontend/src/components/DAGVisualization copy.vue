<template>
  <div class="dag-wrapper">
    <!-- 主容器 -->
    <div class="dag-main">
      <!-- SVG 画布 -->
      <div class="dag-canvas-wrapper">
        <!-- 右上角工具栏 -->
        <div class="dag-toolbar-right">
          <button @click="zoomIn" class="toolbar-icon-btn" title="放大">
            <i class="fas fa-search-plus"></i>
          </button>
          <button @click="zoomOut" class="toolbar-icon-btn" title="缩小">
            <i class="fas fa-search-minus"></i>
          </button>
          <button
            @click="fitToScreen"
            class="toolbar-icon-btn"
            title="适应屏幕"
          >
            <i class="fas fa-expand"></i>
          </button>
          <button @click="resetView" class="toolbar-icon-btn" title="重置视图">
            <i class="fas fa-redo"></i>
          </button>
          <button
            @click="downloadAsImage"
            class="toolbar-icon-btn"
            title="下载图片"
          >
            <i class="fas fa-download"></i>
          </button>
          <button
            @click="copyToClipboard"
            class="toolbar-icon-btn"
            title="复制"
          >
            <i class="fas fa-copy"></i>
          </button>
        </div>

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
            <!-- 箭头标记（精致小巧版） -->
            <marker
              id="arrow"
              markerWidth="6"
              markerHeight="6"
              refX="4"
              refY="3"
              orient="auto"
            >
              <polygon points="0 0, 5 3, 0 6" fill="#BDBDBD" />
            </marker>
            <marker
              id="arrow-red"
              markerWidth="6"
              markerHeight="6"
              refX="4"
              refY="3"
              orient="auto"
            >
              <polygon points="0 0, 5 3, 0 6" fill="#E57373" />
            </marker>

            <!-- 圆形标记（起点） -->
            <marker
              id="circle-normal"
              markerWidth="8"
              markerHeight="8"
              refX="4"
              refY="4"
              orient="auto"
            >
              <circle cx="4" cy="4" r="2" fill="#999" />
            </marker>
            <marker
              id="circle-red"
              markerWidth="8"
              markerHeight="8"
              refX="4"
              refY="4"
              orient="auto"
            >
              <circle cx="4" cy="4" r="2" fill="#ff6b6b" />
            </marker>
          </defs>

          <!-- 背景网格 -->
          <defs>
            <pattern
              id="grid"
              width="20"
              height="20"
              patternUnits="userSpaceOnUse"
            >
              <path
                d="M 20 0 L 0 0 0 20"
                fill="none"
                stroke="#f0f0f0"
                stroke-width="0.5"
              />
            </pattern>
          </defs>
          <rect
            width="100%"
            height="100%"
            fill="url(#grid)"
            @click="clearSelection"
          />

          <!-- 缩放组 -->
          <g
            :transform="`translate(${panX}, ${panY}) scale(${zoom})`"
            class="zoom-group"
          >
            <!-- 连接线 -->
            <g class="lines">
              <path
                v-for="link in links"
                :key="`line-${link.id}`"
                :d="link.path"
                class="connection-line"
                :class="{ 'connection-hotspot': link.isHotspot }"
                :marker-end="`url(#${link.isHotspot ? 'arrow-red' : 'arrow'})`"
                :style="{ strokeWidth: link.strokeWidth + 'px' }"
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
                @click.stop="selectNode(node)"
              >
                <!-- 节点头部（深灰色背景，上圆角） -->
                <rect
                  class="node-header"
                  :class="`node-header-${getNodeColorClass(node)}`"
                  :width="NODE_WIDTH"
                  :height="NODE_HEADER_HEIGHT"
                  rx="2"
                  ry="2"
                />

                <!-- 节点主体（白色背景，无圆角） -->
                <rect
                  class="node-body"
                  :class="`node-body-${getNodeColorClass(node)}`"
                  :width="NODE_WIDTH"
                  :y="NODE_HEADER_HEIGHT"
                  :height="NODE_BODY_HEIGHT"
                />

                <!-- 进度条背景（浅灰色，下圆角） -->
                <rect
                  class="progress-bg"
                  :y="NODE_HEADER_HEIGHT + NODE_BODY_HEIGHT"
                  :width="NODE_WIDTH"
                  :height="NODE_PROGRESS_HEIGHT"
                />

                <!-- 进度条填充（彩色，下圆角） -->
                <rect
                  class="progress-fill"
                  :y="NODE_HEADER_HEIGHT + NODE_BODY_HEIGHT"
                  :width="getProgressWidth(node)"
                  :height="NODE_PROGRESS_HEIGHT"
                  :fill="getProgressColor(node)"
                />

                <!-- 节点整体边框（圆角边框） -->
                <rect
                  class="node-border"
                  :width="NODE_WIDTH"
                  :height="NODE_HEIGHT"
                  rx="2"
                  ry="2"
                  fill="none"
                  stroke="#E0E0E0"
                  stroke-width="1"
                />

                <!-- 头部操作符名称（黑色文字） -->
                <text class="node-title-header" x="10" y="19">
                  {{ node.operator_name }}
                </text>

                <!-- 主体：plan_node_id -->
                <text
                  class="node-info-detail"
                  x="10"
                  :y="NODE_HEADER_HEIGHT + 15"
                >
                  plan_node_id={{ node.plan_node_id }}
                </text>

                <!-- 主体：执行时间 -->
                <text
                  class="node-info-detail"
                  x="10"
                  :y="NODE_HEADER_HEIGHT + 30"
                >
                  耗时:
                  {{
                    formatDuration(
                      node.metrics.operator_total_time_raw ||
                        node.metrics.operator_total_time,
                    )
                  }}
                </text>

                <!-- 主体：性能百分比（右对齐） -->
                <text
                  class="node-percentage-value"
                  :x="NODE_WIDTH - 10"
                  :y="NODE_HEADER_HEIGHT + 30"
                  text-anchor="end"
                >
                  {{ getPercentage(node) }}%
                </text>
              </g>
            </g>
          </g>
        </svg>
      </div>

      <!-- 右侧详情面板 -->
      <div class="detail-panel">
        <!-- Top 10 & 总览（未选中节点时显示） -->
        <div v-if="!selectedNodeId" class="top-panel">
          <div class="overview-header">
            <h3>执行概览</h3>
          </div>

          <div class="overview-content">
            <div
              v-if="summary && summary.total_time_ms"
              class="overview-metrics"
            >
              <!-- 执行时间 -->
              <div class="metric-group">
                <h5>Execution Wall time</h5>
                <div class="time-bar">
                  <div class="time-value">
                    {{ formatDuration(getTotalTime()) }}
                  </div>
                </div>

                <div class="time-breakdown">
                  <div
                    v-for="(item, idx) in getTimeBreakdown()"
                    :key="idx"
                    class="time-item"
                  >
                    <span
                      class="time-label"
                      :style="{ color: getTimeColors()[idx] }"
                      >●</span
                    >
                    <span class="time-name">{{ item.name }}</span>
                    <span class="time-duration">{{ item.duration }}</span>
                    <span class="time-percent">{{ item.percent }}%</span>
                  </div>
                </div>
              </div>

              <!-- Top Most Time-consuming Nodes -->
              <div
                v-if="
                  summary.top_time_consuming_nodes &&
                  summary.top_time_consuming_nodes.length > 0
                "
                class="metric-group"
                style="margin-top: 20px"
              >
                <h5>Top Most Time-consuming Nodes</h5>
                <div class="top-nodes-list">
                  <div
                    v-for="node in summary.top_time_consuming_nodes"
                    :key="node.rank"
                    class="top-node-item"
                    :class="{
                      'top-node-most-consuming': node.is_most_consuming,
                      'top-node-second-consuming':
                        node.is_second_most_consuming,
                    }"
                  >
                    <span class="top-node-rank">{{ node.rank }}.</span>
                    <span class="top-node-name">{{ node.operator_name }}</span>
                    <span
                      v-if="node.total_time && node.total_time !== 'N/A'"
                      class="top-node-time"
                      >{{ node.total_time }}</span
                    >
                    <span class="top-node-percentage"
                      >{{ node.time_percentage.toFixed(2) }}%</span
                    >
                  </div>
                </div>
              </div>

              <!-- 内存 -->
              <div
                v-if="
                  summary.query_allocated_memory || summary.query_peak_memory
                "
                class="metric-group"
                style="margin-top: 20px"
              >
                <h5>Memory</h5>
                <div class="memory-metrics">
                  <div
                    v-if="summary.query_allocated_memory"
                    class="memory-item"
                  >
                    <span class="memory-label">AllocatedMemoryUsage</span>
                    <span class="memory-value">{{
                      formatBytes(summary.query_allocated_memory)
                    }}</span>
                  </div>
                  <div v-if="summary.query_peak_memory" class="memory-item">
                    <span class="memory-label">PeakMemoryUsage</span>
                    <span class="memory-value">{{
                      formatBytes(summary.query_peak_memory)
                    }}</span>
                  </div>
                </div>
              </div>

              <!-- Spill Warning -->
              <div
                v-if="
                  summary.query_spill_bytes &&
                  summary.query_spill_bytes !== '0.000 B'
                "
                class="metric-group"
                style="margin-top: 20px"
              >
                <h5>⚠️ Spill Warning</h5>
                <div class="spill-warning">
                  <div class="spill-item">
                    <span class="spill-label">Spill Bytes</span>
                    <span class="spill-value">{{
                      summary.query_spill_bytes
                    }}</span>
                  </div>
                </div>
              </div>
            </div>
            <div v-else class="empty-state">
              <p>暂无执行概览数据</p>
            </div>
          </div>
        </div>

        <!-- 节点详情（选中节点时显示） -->
        <div v-else class="node-detail-panel">
          <el-tabs
            v-model="activeTab"
            tab-position="top"
            @tab-click="handleTabClick"
          >
            <!-- 节点Tab -->
            <el-tab-pane label="节点" name="node">
              <div class="detail-content">
                <div class="info-grid">
                  <div class="info-item">
                    <span class="info-label">节点ID</span>
                    <span class="info-value">{{ selectedNode.id }}</span>
                  </div>
                  <div class="info-item">
                    <span class="info-label">操作符</span>
                    <span class="info-value">{{
                      selectedNode.operator_name
                    }}</span>
                  </div>
                  <div class="info-item">
                    <span class="info-label">Plan Node ID</span>
                    <span class="info-value">{{
                      selectedNode.plan_node_id
                    }}</span>
                  </div>
                  <div class="info-item">
                    <span class="info-label">深度</span>
                    <span class="info-value">{{ selectedNode.depth }}</span>
                  </div>
                </div>

                <!-- 执行时间 -->
                <div class="metrics-section">
                  <h5>Execution Time</h5>
                  <div class="execution-time-breakdown">
                    <!-- 基础执行时间 -->
                    <div
                      v-if="selectedNode.metrics.operator_total_time"
                      class="time-metric"
                    >
                      <span class="metric-name">Total Time</span>
                      <span class="metric-value">{{
                        formatDuration(selectedNode.metrics.operator_total_time)
                      }}</span>
                    </div>

                    <!-- Push/Pull 时间 -->
                    <div
                      v-if="selectedNode.metrics.push_total_time"
                      class="time-metric"
                    >
                      <span class="metric-name">Push Time</span>
                      <span class="metric-value">{{
                        formatDuration(selectedNode.metrics.push_total_time)
                      }}</span>
                    </div>

                    <div
                      v-if="selectedNode.metrics.pull_total_time"
                      class="time-metric"
                    >
                      <span class="metric-name">Pull Time</span>
                      <span class="metric-value">{{
                        formatDuration(selectedNode.metrics.pull_total_time)
                      }}</span>
                    </div>
                  </div>
                </div>

                <!-- 数据量指标 -->
                <div class="metrics-section">
                  <h5>Data Metrics</h5>
                  <div class="data-metrics-grid">
                    <div
                      v-if="hasMetric(selectedNode.metrics, 'push_chunk_num')"
                      class="metric"
                    >
                      <span class="metric-label">Push Chunks</span>
                      <span class="metric-value">{{
                        selectedNode.metrics.push_chunk_num
                      }}</span>
                    </div>
                    <div
                      v-if="hasMetric(selectedNode.metrics, 'push_row_num')"
                      class="metric"
                    >
                      <span class="metric-label">Push Rows</span>
                      <span class="metric-value">{{
                        formatNumber(selectedNode.metrics.push_row_num)
                      }}</span>
                    </div>
                    <div
                      v-if="hasMetric(selectedNode.metrics, 'pull_chunk_num')"
                      class="metric"
                    >
                      <span class="metric-label">Pull Chunks</span>
                      <span class="metric-value">{{
                        selectedNode.metrics.pull_chunk_num
                      }}</span>
                    </div>
                    <div
                      v-if="hasMetric(selectedNode.metrics, 'pull_row_num')"
                      class="metric"
                    >
                      <span class="metric-label">Pull Rows</span>
                      <span class="metric-value">{{
                        formatNumber(selectedNode.metrics.pull_row_num)
                      }}</span>
                    </div>
                    <div
                      v-if="
                        hasMetric(selectedNode.metrics, 'output_chunk_bytes')
                      "
                      class="metric"
                    >
                      <span class="metric-label">Output Bytes</span>
                      <span class="metric-value">{{
                        formatBytes(selectedNode.metrics.output_chunk_bytes)
                      }}</span>
                    </div>
                    <div
                      v-if="hasMetric(selectedNode.metrics, 'memory_usage')"
                      class="metric"
                    >
                      <span class="metric-label">Memory Usage</span>
                      <span class="metric-value">{{
                        formatBytes(selectedNode.metrics.memory_usage)
                      }}</span>
                    </div>
                  </div>
                </div>

                <!-- Unique Metrics (按照StarRocks官方逻辑直接显示) -->
                <div
                  v-if="hasUniqueMetrics(selectedNode)"
                  class="metrics-section"
                  style="margin-top: 20px"
                >
                  <h5>Over Consuming Metrics</h5>
                  <div class="unique-metrics">
                    <div
                      v-for="(value, key) in getUniqueMetrics(selectedNode)"
                      :key="key"
                      v-show="!isMetricNA(value)"
                      class="metric-item"
                    >
                      <span class="label">{{ formatMetricKey(key) }}:</span>
                      <span class="value">{{ formatMetricValue(value) }}</span>
                      <!-- 显示min/max值，如果存在的话 -->
                      <span
                        v-if="hasMinMaxValues(selectedNode, key)"
                        class="min-max-values"
                      >
                        [max={{
                          formatMetricValue(getMaxValue(selectedNode, key))
                        }}, min={{
                          formatMetricValue(getMinValue(selectedNode, key))
                        }}]
                      </span>
                    </div>
                  </div>
                </div>

                <!-- 操作符特定指标 -->
                <div
                  v-if="hasSpecializedMetrics(selectedNode)"
                  class="metrics-section"
                  style="margin-top: 20px"
                >
                  <h5>{{ selectedNode.operator_name }} 专用指标</h5>
                  <div class="specialized-metrics">
                    <!-- 动态渲染所有专用指标，过滤N/A值 -->
                    <template
                      v-for="(specObj, specKey) in getValidSpecializedMetrics(
                        selectedNode,
                      )"
                      :key="specKey"
                    >
                      <div
                        v-for="(value, key) in specObj"
                        :key="`${specKey}-${key}`"
                        v-show="
                          value !== null &&
                          value !== undefined &&
                          !isMetricNA(value)
                        "
                        class="metric-item"
                      >
                        <span class="label">{{ formatMetricKey(key) }}:</span>
                        <span class="value">{{
                          formatMetricValue(value)
                        }}</span>
                      </div>
                    </template>
                  </div>
                </div>
              </div>
            </el-tab-pane>

            <!-- 节点信息Tab -->
            <el-tab-pane label="节点信息" name="node-info">
              <div class="detail-content">
                <div class="profile-text">
                  <pre>{{ formatNodeProfile(selectedNode) }}</pre>
                </div>
              </div>
            </el-tab-pane>

            <!-- Pipeline Tab -->
            <el-tab-pane label="Pipeline" name="pipeline">
              <div class="detail-content">
                <div class="profile-text">
                  <pre>{{ formatPipelineProfile(selectedNode) }}</pre>
                </div>
              </div>
            </el-tab-pane>
          </el-tabs>
        </div>
      </div>
    </div>
  </div>
</template>

<script>
// Using local formatDuration method instead of imported utility

export default {
  name: "DAGVisualization",

  props: {
    executionTree: {
      type: Object,
      required: true,
    },
    summary: {
      type: Object,
      required: false,
      default: null,
    },
  },

  data() {
    return {
      NODE_WIDTH: 180,
      NODE_HEIGHT: 81, // 更新为新高度：28(头部) + 47(主体) + 6(进度条)
      NODE_HEADER_HEIGHT: 28,
      NODE_BODY_HEIGHT: 47,
      NODE_PROGRESS_HEIGHT: 6,

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

      maxTime: 0,
      totalTimeMs: 0,
      activeTab: "overview",
      topTab: "time", // Default to overview tab
    };
  },

  watch: {
    executionTree: {
      handler() {
        try {
          console.log("[DAG] watch executionTree change", {
            nodes:
              this.executionTree && this.executionTree.nodes
                ? this.executionTree.nodes.length
                : 0,
          });
        } catch (e) {
          console.warn("[DAG] log error", e);
        }
        this.renderDAG();
      },
      immediate: true,
    },
    summary: {
      handler(val) {
        try {
          console.log("[DAG] watch summary change", {
            exists: !!val,
            summary: val,
          });
        } catch (e) {
          console.warn("[DAG] log error", e);
        }
        this.debugOverview();
      },
      immediate: false,
    },
    selectedNodeId(val) {
      try {
        console.log("[DAG] selectedNodeId change", {
          selectedNodeId: val,
          activeTab: this.activeTab,
        });
      } catch (e) {
        console.warn("[DAG] log error", e);
      }
    },
  },

  mounted() {
    window.addEventListener("resize", this.onWindowResize);
    // Initialize state - don't select any node, so execution overview shows by default
    this.selectedNodeId = null;
    this.selectedNode = null;
    this.activeTab = "overview";
    try {
      // debug
      console.log("[DAG] mounted", {
        summaryExists: !!this.summary,
        summary: this.summary,
        execTreeNodes:
          this.executionTree && this.executionTree.nodes
            ? this.executionTree.nodes.length
            : 0,
        selectedNodeId: this.selectedNodeId,
        activeTab: this.activeTab,
      });
      this.debugOverview();
    } catch (e) {
      console.warn("[DAG] mounted log error", e);
    }
  },

  beforeUnmount() {
    window.removeEventListener("resize", this.onWindowResize);
  },

  computed: {
    topTimeNodes() {
      if (!this.executionTree || !this.executionTree.nodes) return [];

      return [...this.executionTree.nodes]
        .filter((n) => n.time_percentage != null)
        .sort((a, b) => (b.time_percentage || 0) - (a.time_percentage || 0))
        .slice(0, 10);
    },

    ioPercent() {
      if (!this.summary) return 0;
      const ioTime = this.summary.io_time_ms || 0;
      const totalTime = this.summary.query_execution_wall_time_ms || 1;
      return (ioTime / totalTime) * 100;
    },

    processingPercent() {
      if (!this.summary) return 0;
      const processingTime = this.summary.processing_time_ms || 0;
      const totalTime = this.summary.query_execution_wall_time_ms || 1;
      return (processingTime / totalTime) * 100;
    },

    ioTime() {
      if (!this.summary) return "0ms";
      return this.formatDuration(this.summary.io_time_ms);
    },

    processingTime() {
      if (!this.summary) return "0ms";
      return this.formatDuration(this.summary.processing_time_ms);
    },
  },

  methods: {
    renderDAG() {
      if (
        !this.executionTree ||
        !this.executionTree.nodes ||
        !this.executionTree.nodes.length
      ) {
        try {
          console.log("[DAG] renderDAG skipped: no executionTree");
        } catch (e) {
          console.warn("[DAG] log error", e);
        }
        return;
      }

      const nodeMap = new Map();
      const nodesByDepth = new Map();

      // 按深度分组
      this.executionTree.nodes.forEach((node) => {
        if (!node || !node.metrics) return;
        nodeMap.set(node.id, node);
        const depth = node.depth || 0;
        if (!nodesByDepth.has(depth)) {
          nodesByDepth.set(depth, []);
        }
        nodesByDepth.get(depth).push(node);
      });

      // 计算最大执行时间与总执行时间
      let maxTime = 0;
      let total = 0;
      this.executionTree.nodes.forEach((node) => {
        if (!node || !node.metrics) return;
        const time = this.getDurationMs(node.metrics.operator_total_time);
        maxTime = Math.max(maxTime, time);
        total += time;
      });
      this.maxTime = maxTime || 1;
      this.totalTimeMs = total || 1;
      try {
        console.log("[DAG] renderDAG computed", {
          maxTime: this.maxTime,
          totalTimeMs: this.totalTimeMs,
          nodeCount: this.executionTree.nodes.length,
        });
      } catch (e) {
        console.warn("[DAG] log error", e);
      }

      // 计算节点位置 - 垂直布局（从上到下）
      const levelHeight = 180; // 垂直间距
      const levelWidth = 250; // 水平间距

      // 计算最大深度以确定SVG高度
      let maxDepth = 0;
      this.executionTree.nodes.forEach((node) => {
        maxDepth = Math.max(maxDepth, node.depth || 0);
      });

      // 动态调整SVG高度
      this.svgHeight = Math.max(600, (maxDepth + 1) * levelHeight + 150);

      this.nodes = this.executionTree.nodes.map((node) => {
        const depth = node.depth || 0;
        const levelNodes = nodesByDepth.get(depth) || [];
        const indexInLevel = levelNodes.indexOf(node);
        const nodesCountInLevel = levelNodes.length;

        // 垂直布局：y根据depth增加，x根据同深度内的位置计算（水平居中）
        const y = depth * levelHeight + 80;
        const totalWidth = (nodesCountInLevel - 1) * levelWidth;
        const centerX = this.svgWidth / 2;
        const x = centerX - totalWidth / 2 + indexInLevel * levelWidth;

        return {
          ...node,
          x,
          y,
          is_scan: node.operator_name && node.operator_name.includes("SCAN"),
        };
      });

      // 构建连接线（箭头从下指向上）
      this.links = [];
      this.executionTree.nodes.forEach((sourceNode) => {
        if (!sourceNode || !sourceNode.children) return;
        sourceNode.children.forEach((childId, childIndex) => {
          const targetNode = nodeMap.get(childId);
          if (targetNode && targetNode.metrics) {
            const source = this.nodes.find((n) => n.id === sourceNode.id);
            const target = this.nodes.find((n) => n.id === targetNode.id);

            if (source && target && source.metrics && target.metrics) {
              // Arrow from child top (SCAN顶部) to parent bottom (TABLE_FUNCTION底部)
              const startX = target.x + this.NODE_WIDTH / 2;
              const startY = target.y; // From TOP of child (子节点顶部)
              const endX = source.x + this.NODE_WIDTH / 2;
              const endY = source.y + this.NODE_HEIGHT + 8; // To BOTTOM of parent (父节点底部，留8px间隙)

              const controlY = (startY + endY) / 2;
              const path = `M ${startX} ${startY} C ${startX} ${controlY}, ${endX} ${controlY}, ${endX} ${endY}`;

              // 显示行数在箭头中点
              const rows = this.getNodeRows(targetNode);
              let label = `Rows: ${this.formatRowsSimple(rows)}`;

              // 如果父节点是JOIN类型，添加PROBE/BUILD标记
              if (
                sourceNode.operator_name &&
                sourceNode.operator_name.includes("JOIN")
              ) {
                // 第一个子节点通常是PROBE侧，第二个是BUILD侧
                label += childIndex === 0 ? " (PROBE)" : " (BUILD)";
              }

              this.links.push({
                id: `${source.id}-${target.id}`,
                path,
                labelX: (startX + endX) / 2,
                labelY: controlY - 8,
                label: label,
                isHotspot: source.is_hotspot || target.is_hotspot,
                rowCount: rows, // Store row count for dynamic stroke width
                strokeWidth: this.calculateStrokeWidth(rows),
              });
            }
          }
        });
      });
      try {
        console.log("[DAG] renderDAG links built", {
          linkCount: this.links.length,
        });
      } catch (e) {
        console.warn("[DAG] log error", e);
      }
    },

    formatDuration(duration) {
      if (duration === null || duration === undefined) return "N/A";
      if (typeof duration === "string") {
        return this.formatChineseDurationString(duration);
      }

      // Get total nanoseconds for precise calculation
      let totalNanos = 0;
      if (typeof duration === "object" && duration !== null) {
        const secs = duration.secs || 0;
        const nanos = duration.nanos || 0;
        totalNanos = secs * 1_000_000_000 + nanos;
      } else if (typeof duration === "number") {
        // Check if it's milliseconds (from getTotalTime) or nanoseconds
        // If the number is large (> 1000000), assume it's nanoseconds
        // If the number is smaller, assume it's milliseconds
        if (duration > 1000000) {
          totalNanos = duration;
        } else {
          // Convert milliseconds to nanoseconds
          totalNanos = duration * 1_000_000;
        }
      }

      if (totalNanos === 0) return "0纳秒";

      // Calculate time units
      const hours = Math.floor(totalNanos / (3600 * 1_000_000_000));
      const minutes = Math.floor(
        (totalNanos % (3600 * 1_000_000_000)) / (60 * 1_000_000_000),
      );
      const seconds = Math.floor(
        (totalNanos % (60 * 1_000_000_000)) / 1_000_000_000,
      );
      const millis = Math.floor((totalNanos % 1_000_000_000) / 1_000_000);
      const micros = Math.floor((totalNanos % 1_000_000) / 1_000);
      const nanos = Math.floor(totalNanos % 1_000);

      // Build human-readable format
      const parts = [];
      if (hours > 0) parts.push(`${hours}时`);
      if (minutes > 0) parts.push(`${minutes}分`);
      if (seconds > 0) parts.push(`${seconds}秒`);
      if (millis > 0) parts.push(`${millis}毫秒`);
      if (micros > 0 && parts.length < 3) parts.push(`${micros}微秒`);
      if (nanos > 0 && parts.length < 2) parts.push(`${nanos}纳秒`);

      // Return appropriate precision based on magnitude
      if (parts.length === 0) {
        return `${nanos}纳秒`;
      } else if (parts.length > 3) {
        return parts.slice(0, 3).join("");
      } else {
        return parts.join("");
      }
    },

    formatChineseDurationString(durationStr) {
      // Handle string duration formats like "1h2m3s4ms5us6ns" or "44s875ms"
      if (!durationStr || typeof durationStr !== "string") return "N/A";

      // Parse different time units from string
      const timeRegex = /(\d+(?:\.\d+)?)(h|m|s|ms|us|ns)/g;
      const matches = [...durationStr.matchAll(timeRegex)];

      if (matches.length === 0) return durationStr; // Return original if no matches

      const parts = [];
      for (const match of matches) {
        const value = parseFloat(match[1]);
        const unit = match[2];

        switch (unit) {
          case "h":
            parts.push(`${Math.floor(value)}时`);
            break;
          case "m":
            parts.push(`${Math.floor(value)}分`);
            break;
          case "s":
            // For seconds, only add if it's not followed by ms/us/ns
            if (
              !durationStr.includes("ms") &&
              !durationStr.includes("us") &&
              !durationStr.includes("ns")
            ) {
              parts.push(`${Math.floor(value)}秒`);
            }
            break;
          case "ms":
            parts.push(`${Math.floor(value)}毫秒`);
            break;
          case "us":
            parts.push(`${Math.floor(value)}微秒`);
            break;
          case "ns":
            parts.push(`${Math.floor(value)}纳秒`);
            break;
        }
      }

      return parts.length > 0 ? parts.join("") : durationStr;
    },

    getDurationMs(duration) {
      if (!duration) return 0;
      if (typeof duration === "number") {
        // Backend now returns nanoseconds, convert to milliseconds
        return Math.floor(duration / 1_000_000);
      }
      if (typeof duration === "object" && duration !== null) {
        const secs = duration.secs || 0;
        const nanos = duration.nanos || 0;
        return secs * 1000 + Math.floor(nanos / 1_000_000);
      }
      if (typeof duration === "string") {
        return this.parseRawDurationMs(duration);
      }
      return 0;
    },

    formatBytes(bytes) {
      if (!bytes) return "N/A";
      const units = ["B", "KB", "MB", "GB", "TB"];
      const index = Math.floor(Math.log(bytes) / Math.log(1024));
      return `${(bytes / Math.pow(1024, index)).toFixed(2)} ${units[index]}`;
    },

    formatNumber(num) {
      if (num === null || num === undefined) return "N/A";
      if (num === 0) return "0";

      if (num >= 1000000000) {
        return (num / 1000000000).toFixed(1) + "B";
      } else if (num >= 1000000) {
        return (num / 1000000).toFixed(1) + "M";
      } else if (num >= 1000) {
        return (num / 1000).toFixed(1) + "K";
      } else {
        return num.toString();
      }
    },

    formatRows(rows) {
      if (!rows || rows === 0) return "行数：0";

      // Format large numbers for better readability
      if (rows >= 1000000) {
        const millions = (rows / 1000000).toFixed(2);
        return `行数：${millions}M`;
      } else if (rows >= 1000) {
        const thousands = (rows / 1000).toFixed(2);
        return `行数：${thousands}K`;
      } else {
        return `行数：${rows}`;
      }
    },

    // Format rows for connection labels (without prefix)
    formatRowsSimple(rows) {
      if (!rows || rows === 0) return "0";

      // Format large numbers with commas
      if (rows >= 1000000) {
        return rows.toString().replace(/\B(?=(\d{3})+(?!\d))/g, ",");
      } else if (rows >= 1000) {
        return rows.toString().replace(/\B(?=(\d{3})+(?!\d))/g, ",");
      } else {
        return rows.toString();
      }
    },

    // 获取节点的有意义的行数（优先使用 push_row_num，否则使用 pull_row_num）
    getNodeRows(node) {
      if (!node || !node.metrics) return 0;
      // 优先使用 push_row_num（表示输出行数）
      if (node.metrics.push_row_num && node.metrics.push_row_num > 0) {
        return node.metrics.push_row_num;
      }
      // 其次使用 pull_row_num（表示输入行数）
      if (node.metrics.pull_row_num && node.metrics.pull_row_num > 0) {
        return node.metrics.pull_row_num;
      }
      return 0;
    },

    // 根据行数计算箭头粗细（行数越多越粗，范围1-2.5px）
    calculateStrokeWidth(rows) {
      if (!rows || rows === 0) return 1;

      // 使用对数缩放，范围从1到2.5，保持精致纤细
      const minWidth = 1;
      const maxWidth = 2.5;
      const log10 = Math.log10(rows);
      const normalized = Math.min(log10 / 8, 1); // 假设最大行数约为10^8
      const width = minWidth + (maxWidth - minWidth) * normalized;

      return Math.max(minWidth, Math.min(maxWidth, width));
    },

    getPercentage(node) {
      // 优先使用后端计算的time_percentage（这是官方StarRocks解析逻辑）
      if (
        node &&
        node.time_percentage !== undefined &&
        node.time_percentage !== null
      ) {
        return node.time_percentage.toFixed(2);
      }

      // 回退到前端计算（仅用于兼容旧数据）
      if (!node || !node.metrics) return 0;
      const nodeMs = this.getDurationMs(
        node.metrics.operator_total_time_raw ||
          node.metrics.operator_total_time,
      );
      if (nodeMs === 0) return 0;
      const fragId = node.fragment_id;
      if (fragId) {
        const fragTotal = this.getFragmentTotalMs(fragId);
        if (fragTotal > 0) return ((nodeMs / fragTotal) * 100).toFixed(2);
      }
      if (this.totalTimeMs && this.totalTimeMs > 0) {
        return ((nodeMs / this.totalTimeMs) * 100).toFixed(2);
      }
      return 0;
    },

    getFragmentTotalMs(fragmentId) {
      // Calculate total time for a specific fragment by summing all nodes in that fragment
      if (!this.nodes || !fragmentId) return 0;

      let totalMs = 0;
      for (const node of this.nodes) {
        if (node.fragment_id === fragmentId && node.metrics) {
          const nodeMs = this.getDurationMs(
            node.metrics.operator_total_time_raw ||
              node.metrics.operator_total_time,
          );
          totalMs += nodeMs;
        }
      }
      return totalMs;
    },

    selectNode(node) {
      this.selectedNodeId = node.id;
      this.selectedNode = node;
      // 立即设置 activeTab 以确保内容显示
      this.$nextTick(() => {
        this.activeTab = "node";
      });
      try {
        console.log("[DAG] selectNode", {
          selectedNodeId: this.selectedNodeId,
          activeTab: this.activeTab,
          nodeName: node.operator_name,
        });
      } catch (e) {
        console.warn("[DAG] log error", e);
      }
    },

    resetView() {
      this.zoom = 1;
      this.panX = 20;
      this.panY = 20;
    },

    zoomIn() {
      this.zoom = Math.min(this.zoom + 0.2, 3);
    },

    zoomOut() {
      this.zoom = Math.max(this.zoom - 0.2, 0.2);
    },

    fitToScreen() {
      this.zoom = 1;
      this.resetView();
    },

    handleWheel(e) {
      if (e.deltaY < 0) {
        this.zoomIn();
      } else {
        this.zoomOut();
      }
    },

    startPan(e) {
      if (e.button !== 0) return;
      this.isPanning = true;
      this.panStartX = e.clientX - this.panX;
      this.panStartY = e.clientY - this.panY;
    },

    doPan(e) {
      if (!this.isPanning) return;
      this.panX = e.clientX - this.panStartX;
      this.panY = e.clientY - this.panStartY;
    },

    endPan() {
      this.isPanning = false;
    },

    onWindowResize() {
      this.svgWidth = window.innerWidth - 400;
    },

    // New methods for execution overview
    getTotalTime() {
      if (!this.summary) return 0;
      // Use execution wall time if available, otherwise fall back to total_time_ms
      return (
        this.summary.query_execution_wall_time_ms ||
        this.summary.total_time_ms ||
        0
      );
    },

    getTimeBreakdown() {
      if (!this.summary) return [];

      // Use execution wall time as the base total time
      const totalMs =
        this.summary.query_execution_wall_time_ms ||
        this.summary.total_time_ms ||
        0;

      if (totalMs === 0) return [];

      const breakdown = [];

      // Add CPU time (QueryCpuTime)
      if (this.summary.query_cumulative_cpu_time_ms !== undefined) {
        breakdown.push({
          name: "QueryCpuTime",
          duration: this.summary.query_cumulative_cpu_time || "0ns",
          percent: (
            (this.summary.query_cumulative_cpu_time_ms / totalMs) *
            100
          ).toFixed(2),
        });
      }

      // Add schedule time (QueryScheduleTime)
      if (this.summary.query_peak_schedule_time_ms !== undefined) {
        breakdown.push({
          name: "QueryScheduleTime",
          duration: this.summary.query_peak_schedule_time || "0ns",
          percent: (
            (this.summary.query_peak_schedule_time_ms / totalMs) *
            100
          ).toFixed(2),
        });
      }

      // Add scan time (QueryScanTime)
      if (this.summary.query_cumulative_scan_time_ms !== undefined) {
        breakdown.push({
          name: "QueryScanTime",
          duration: this.summary.query_cumulative_scan_time || "0ns",
          percent: (
            (this.summary.query_cumulative_scan_time_ms / totalMs) *
            100
          ).toFixed(2),
        });
      }

      // Add network time (QueryNetworkTime)
      if (this.summary.query_cumulative_network_time_ms !== undefined) {
        breakdown.push({
          name: "QueryNetworkTime",
          duration: this.summary.query_cumulative_network_time || "0ns",
          percent: (
            (this.summary.query_cumulative_network_time_ms / totalMs) *
            100
          ).toFixed(2),
        });
      }

      // Add result deliver time (ResultDeliverTime)
      if (this.summary.result_deliver_time_ms !== undefined) {
        breakdown.push({
          name: "ResultDeliverTime",
          duration: this.summary.result_deliver_time || "0ns",
          percent: (
            (this.summary.result_deliver_time_ms / totalMs) *
            100
          ).toFixed(2),
        });
      }

      // Add operator cumulative time (QueryCumulativeOperatorTime)
      if (this.summary.query_cumulative_operator_time_ms !== undefined) {
        breakdown.push({
          name: "QueryCumulativeOperatorTime",
          duration: this.summary.query_cumulative_operator_time || "0ns",
          percent: (
            (this.summary.query_cumulative_operator_time_ms / totalMs) *
            100
          ).toFixed(2),
        });
      }

      return breakdown;
    },

    getTimeColors() {
      return ["#FF9800", "#00BCD4", "#2196F3", "#9C27B0", "#E91E63", "#4CAF50"]; // Orange, Teal, Blue, Purple, Pink, Green
    },

    formatNodeProfile(node) {
      if (!node) return "No node data available.";
      // Return node-specific metrics information
      const nodeInfo = {
        id: node.id,
        operator_name: node.operator_name,
        plan_node_id: node.plan_node_id,
        depth: node.depth,
        is_hotspot: node.is_hotspot,
        metrics: node.metrics,
      };
      return JSON.stringify(nodeInfo, null, 2);
    },

    formatPipelineProfile(node) {
      if (!node) return "No pipeline data available.";
      // Return pipeline-level information
      // This would come from the backend in a real scenario
      const pipelineInfo = {
        note: "Pipeline information from the execution context",
        execution_context: {
          node_id: node.id,
          operator_name: node.operator_name,
          parent_node: node.parent_plan_node_id,
          depth: node.depth,
          children_count: node.children ? node.children.length : 0,
        },
        metrics: node.metrics,
      };
      return JSON.stringify(pipelineInfo, null, 2);
    },

    handleTabClick(tab) {
      this.activeTab = tab.paneName;
      try {
        console.log("[DAG] handleTabClick", { activeTab: this.activeTab });
      } catch (e) {
        console.warn("[DAG] log error", e);
      }
    },

    clearSelection() {
      this.selectedNodeId = null;
      this.selectedNode = null;
      this.activeTab = "overview"; // Switch to overview tab when clicking background
      try {
        console.log("[DAG] clearSelection -> show overview", {
          selectedNodeId: this.selectedNodeId,
          activeTab: this.activeTab,
        });
      } catch (e) {
        console.warn("[DAG] log error", e);
      }
      this.debugOverview();
    },

    debugOverview() {
      try {
        console.log("[DAG] debugOverview", {
          selectedNodeId: this.selectedNodeId,
          showOverview: !this.selectedNodeId,
          summaryExists: !!this.summary,
          summary: this.summary,
          breakdown: this.summary ? this.getTimeBreakdown() : [],
        });
      } catch (e) {
        console.warn("[DAG] debugOverview log error", e);
      }
    },

    // New methods for specialized metrics
    hasSpecializedMetrics(node) {
      if (!node || !node.metrics || !node.metrics.specialized) return false;
      const specialized = node.metrics.specialized;
      // Only return true if it's a valid object with content (not a string or null)
      return (
        typeof specialized === "object" &&
        specialized !== null &&
        !Array.isArray(specialized) &&
        Object.keys(specialized).length > 0
      );
    },

    getValidSpecializedMetrics(node) {
      if (!this.hasSpecializedMetrics(node)) return {};
      const specialized = node.metrics.specialized;

      // Check if specialized is a valid object (not a string like "None")
      if (
        typeof specialized !== "object" ||
        specialized === null ||
        Array.isArray(specialized)
      ) {
        return {};
      }

      const validMetrics = {};
      for (const key in specialized) {
        if (Object.prototype.hasOwnProperty.call(specialized, key)) {
          validMetrics[key] = specialized[key];
        }
      }
      return validMetrics;
    },

    // New methods for unique metrics (按照StarRocks官方逻辑)
    hasUniqueMetrics(node) {
      if (!node || !node.unique_metrics) return false;
      return Object.keys(node.unique_metrics).length > 0;
    },

    getUniqueMetrics(node) {
      if (!this.hasUniqueMetrics(node)) return {};

      const uniqueMetrics = {};
      for (const [key, value] of Object.entries(node.unique_metrics)) {
        // 过滤掉__MAX_OF_和__MIN_OF_前缀的指标，这些会在min/max值中单独处理
        if (!key.startsWith("__MAX_OF_") && !key.startsWith("__MIN_OF_")) {
          uniqueMetrics[key] = value;
        }
      }
      return uniqueMetrics;
    },

    hasMinMaxValues(node, key) {
      if (!node || !node.unique_metrics) return false;
      const maxKey = `__MAX_OF_${key}`;
      const minKey = `__MIN_OF_${key}`;
      return (
        Object.prototype.hasOwnProperty.call(node.unique_metrics, maxKey) ||
        Object.prototype.hasOwnProperty.call(node.unique_metrics, minKey)
      );
    },

    getMaxValue(node, key) {
      if (!node || !node.unique_metrics) return null;
      const maxKey = `__MAX_OF_${key}`;
      return node.unique_metrics[maxKey] || null;
    },

    getMinValue(node, key) {
      if (!node || !node.unique_metrics) return null;
      const minKey = `__MIN_OF_${key}`;
      return node.unique_metrics[minKey] || null;
    },

    formatMetricKey(key) {
      // Convert camelCase to readable format, ensure key is string
      if (typeof key !== "string") {
        key = String(key);
      }
      return key.replace(/([A-Z])/g, " $1").trim();
    },

    formatMetricValue(value) {
      if (value === null || value === undefined) {
        return "N/A";
      }
      // Check if it's a duration object (with secs and nanos)
      if (
        typeof value === "object" &&
        !Array.isArray(value) &&
        ("secs" in value || "nanos" in value)
      ) {
        return this.formatDuration(value);
      }
      if (typeof value === "number") {
        return value.toString();
      } else if (typeof value === "string") {
        return value;
      } else if (typeof value === "boolean") {
        return value ? "Yes" : "No";
      } else if (Array.isArray(value)) {
        return value.length > 0 ? `[${value.length} items]` : "[]";
      } else if (typeof value === "object") {
        return JSON.stringify(value, null, 2);
      }
      return String(value);
    },

    // Check if a metric exists and is not null/undefined
    hasMetric(metrics, key) {
      if (!metrics) return false;
      const value = metrics[key];
      return value !== null && value !== undefined;
    },

    // Check if there are any metrics to show
    hasMetricsToShow(node) {
      if (!node || !node.metrics) return false;
      return (
        this.hasMetric(node.metrics, "push_chunk_num") ||
        this.hasMetric(node.metrics, "push_row_num") ||
        this.hasMetric(node.metrics, "pull_chunk_num") ||
        this.hasMetric(node.metrics, "pull_row_num")
      );
    },

    // Check if metric value is N/A
    isMetricNA(value) {
      if (value === null || value === undefined) return true;
      const formatted = this.formatMetricValue(value);
      return formatted === "N/A" || formatted === "N/A";
    },

    // Get node color class based on percentage
    getNodeColorClass(node) {
      const percentage = parseFloat(this.getPercentage(node));
      if (percentage > 30) return "red";
      if (percentage >= 15) return "orange";
      return "normal";
    },

    // Get progress bar width
    getProgressWidth(node) {
      const percentage = parseFloat(this.getPercentage(node));
      const width = (percentage / 100) * this.NODE_WIDTH;
      return Math.max(0, Math.min(width, this.NODE_WIDTH));
    },

    // Get progress bar color
    getProgressColor(node) {
      const percentage = parseFloat(this.getPercentage(node));
      if (percentage > 30) return "#E57373";
      if (percentage >= 15) return "#FFB74D";
      return "#4CAF50";
    },

    // Get node border color based on performance
    getNodeBorderColor(node) {
      const percentage = parseFloat(this.getPercentage(node));
      if (percentage > 30) return "#EF5350";
      if (percentage >= 15) return "#BA68C8";
      return "#E0E0E0";
    },

    // Get node border width based on performance
    getNodeBorderWidth(node) {
      const percentage = parseFloat(this.getPercentage(node));
      if (percentage > 30) return 2;
      if (percentage >= 15) return 2;
      return 1;
    },

    // Check if node is in top 3 time-consuming nodes
    isTopTimeNode(node) {
      if (!this.topTimeNodes || this.topTimeNodes.length === 0) return false;
      const top3 = this.topTimeNodes.slice(0, 3);
      return top3.some((n) => n.id === node.id);
    },

    // Download DAG as image
    downloadAsImage() {
      const svgElement = this.$refs.dagSvg;
      if (!svgElement) return;

      const svgData = new XMLSerializer().serializeToString(svgElement);
      const canvas = document.createElement("canvas");
      const ctx = canvas.getContext("2d");
      const img = new Image();

      img.onload = () => {
        canvas.width = img.width;
        canvas.height = img.height;
        ctx.drawImage(img, 0, 0);
        canvas.toBlob((blob) => {
          const url = URL.createObjectURL(blob);
          const a = document.createElement("a");
          a.href = url;
          a.download = "dag-execution-tree.png";
          a.click();
          URL.revokeObjectURL(url);
        });
      };

      img.src =
        "data:image/svg+xml;base64," +
        btoa(unescape(encodeURIComponent(svgData)));
    },

    // Copy to clipboard
    async copyToClipboard() {
      const svgElement = this.$refs.dagSvg;
      if (!svgElement) return;

      try {
        const svgData = new XMLSerializer().serializeToString(svgElement);
        const blob = new Blob([svgData], { type: "image/svg+xml" });

        await navigator.clipboard.write([
          new ClipboardItem({ "image/svg+xml": blob }),
        ]);

        console.log("已复制到剪贴板");
      } catch (err) {
        console.error("复制失败:", err);
        // Fallback: copy as text
        try {
          const svgData = new XMLSerializer().serializeToString(svgElement);
          await navigator.clipboard.writeText(svgData);
          console.log("已复制SVG代码到剪贴板");
        } catch (e) {
          console.error("复制失败:", e);
        }
      }
    },
  },
};
</script>

<style scoped>
.dag-wrapper {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: #fafafa;
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

/* 右上角工具栏 */
.dag-toolbar-right {
  position: absolute;
  top: 16px;
  right: 16px;
  display: flex;
  gap: 8px;
  background: white;
  padding: 6px;
  border-radius: 6px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  z-index: 10;
}

.toolbar-icon-btn {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  background: #f5f5f5;
  border: none;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.2s;
  font-size: 14px;
  color: #666;
}

.toolbar-icon-btn:hover {
  background: #e8e8e8;
  color: #333;
}

.toolbar-icon-btn:active {
  background: #d8d8d8;
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

/* 连接线（精致纤细风格） */
.connection-line {
  stroke: #bdbdbd;
  fill: none;
  stroke-linecap: round;
  transition: stroke 0.2s;
}

.connection-line:hover {
  stroke: #9e9e9e;
}

.connection-hotspot {
  stroke: #e57373 !important;
}

.row-count-label {
  font-size: 11px;
  fill: #757575;
  pointer-events: none;
  text-anchor: middle;
  font-weight: 400;
}

/* 节点 */
.node-group {
  cursor: pointer;
  transition: all 0.2s;
}

.node-group:hover .node-header,
.node-group:hover .node-body {
  filter: drop-shadow(0 0 12px rgba(33, 150, 243, 0.5));
}

/* 移除选中节点的蓝色边框效果 */

/* 节点头部样式 */
.node-header {
  fill: #cfd8dc;
  stroke: none;
  transition: all 0.2s;
}

.node-header-normal {
  fill: #cfd8dc;
}

.node-header-orange {
  fill: #e1bee7;
}

.node-header-red {
  fill: #ffcdd2;
}

/* 节点主体样式 */
.node-body {
  fill: #ffffff;
  stroke: #e0e0e0;
  stroke-width: 1px;
  transition: all 0.2s;
}

.node-body-normal {
  fill: #ffffff;
  stroke: #e0e0e0;
}

.node-body-orange {
  fill: #f3e5f5;
  stroke: #e0e0e0;
  stroke-width: 1px;
}

.node-body-red {
  fill: #ffebee;
  stroke: #e0e0e0;
  stroke-width: 1px;
}

/* 进度条 */
.progress-bg {
  fill: #e0e0e0;
  stroke: #bdbdbd;
  fill: none;
  stroke-linecap: round;
  transition: stroke 0.2s;
}

.connection-line:hover {
  stroke: #9e9e9e;
}

.connection-hotspot {
  stroke: #e57373 !important;
}

.row-count-label {
  font-size: 11px;
  fill: #757575;
  pointer-events: none;
  text-anchor: middle;
  font-weight: 400;
}

/* 节点 */
.node-group {
  cursor: pointer;
  transition: all 0.2s;
}

.node-group:hover .node-header,
.node-group:hover .node-body {
  filter: drop-shadow(0 0 12px rgba(33, 150, 243, 0.5));
}

/* 移除选中节点的蓝色边框效果 */

/* 节点头部样式 */
.node-header {
  fill: #cfd8dc;
  stroke: none;
  transition: all 0.2s;
}

.node-header-normal {
  fill: #cfd8dc;
}

.node-header-orange {
  fill: #e1bee7;
}

.node-header-red {
  fill: #ffcdd2;
}

/* 节点主体样式 */
.node-body {
  fill: #ffffff;
  stroke: #e0e0e0;
  stroke-width: 1px;
  transition: all 0.2s;
}

.node-body-normal {
  fill: #ffffff;
  stroke: #e0e0e0;
}

.node-body-orange {
  fill: #f3e5f5;
  stroke: #e0e0e0;
  stroke-width: 1px;
}

.node-body-red {
  fill: #ffebee;
  stroke: #e0e0e0;
  stroke-width: 1px;
}

/* 进度条 */
.progress-bg {
  fill: #e0e0e0;
}

.progress-fill {
  transition: width 0.3s ease;
}

/* 文字样式 */
.node-title-header {
  fill: #424242;
  font-weight: 600;
  font-size: 14px;
  pointer-events: none;
  user-select: none;
}

.node-info-detail {
  fill: #666666;
  font-size: 10px;
  font-weight: 400;
  pointer-events: none;
  user-select: none;
}

.node-percentage-value {
  fill: #333333;
  font-size: 13px;
  font-weight: 700;
  pointer-events: none;
  user-select: none;
}

/* 右侧详情面板 */
.detail-panel {
  background: white;
  border-left: 1px solid #e8e8e8;
  overflow-y: auto;
  box-shadow: -1px 0 2px rgba(0, 0, 0, 0.05);
  position: relative;
  flex-shrink: 0;
  min-width: 200px;
  max-width: 800px;
}

/* 拖动条 */
.resize-handle {
  position: absolute;
  left: -2px;
  top: 0;
  width: 8px;
  height: 100%;
  background: transparent;
  cursor: col-resize;
  z-index: 10;
  transition: background-color 0.2s ease;
}

.resize-handle:hover {
  background: rgba(24, 144, 255, 0.1);
}

.resize-handle:active {
  background: rgba(24, 144, 255, 0.2);
}

/* 拖动时的视觉反馈 */
.detail-panel.resizing {
  user-select: none;
  pointer-events: none;
}

.detail-panel.resizing .resize-handle {
  background: rgba(24, 144, 255, 0.3);
}

.detail-tabs {
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

/* 确保内容能够正确换行和适应宽度 */
.detail-panel .metric-item,
.detail-panel .info-item,
.detail-panel .time-metric {
  word-wrap: break-word;
  overflow-wrap: break-word;
}

/* 优化小宽度时的显示 */
@media (max-width: 300px) {
  .detail-panel {
    font-size: 12px;
  }

  .detail-panel .metric-item .label,
  .detail-panel .info-item .info-label {
    font-size: 11px;
  }

  .detail-panel .metric-item .value,
  .detail-panel .info-item .info-value {
    font-size: 11px;
  }
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

.data-metrics-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
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

.execution-time-breakdown {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.time-metric {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px;
  background: #f9f9f9;
  border-left: 3px solid #1890ff;
  border-radius: 2px;
}

.time-metric .metric-name {
  font-size: 11px;
  color: #666;
  font-weight: 500;
}

.time-metric .metric-value {
  font-size: 12px;
  font-weight: 600;
  color: #333;
}

/* Top Most Time-consuming Nodes 样式 */
.top-nodes-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 10px;
}

.top-node-item {
  display: flex;
  align-items: center;
  padding: 10px 12px;
  background: #f9f9f9;
  border-radius: 4px;
  border-left: 3px solid #d9d9d9;
  transition: all 0.2s;
}

.top-node-item:hover {
  background: #f0f0f0;
  transform: translateX(2px);
}

.top-node-most-consuming {
  background: #ffebee !important;
  border-left-color: #f5222d !important;
}

.top-node-second-consuming {
  background: #fff5f5 !important;
  border-left-color: #fa8c16 !important;
}

.top-node-rank {
  font-size: 14px;
  font-weight: 700;
  color: #666;
  min-width: 25px;
}

.top-node-most-consuming .top-node-rank {
  color: #f5222d;
}

.top-node-second-consuming .top-node-rank {
  color: #fa8c16;
}

.top-node-name {
  flex: 1;
  font-size: 12px;
  font-weight: 600;
  color: #333;
  margin: 0 10px;
}

.top-node-time {
  font-size: 11px;
  color: #666;
  margin-right: 10px;
}

.top-node-percentage {
  font-size: 13px;
  font-weight: 700;
  color: #1890ff;
  min-width: 60px;
  text-align: right;
}

.top-node-most-consuming .top-node-percentage {
  color: #f5222d;
}

.top-node-second-consuming .top-node-percentage {
  color: #fa8c16;
}

.specialized-metrics {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.unique-metrics {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.unique-metrics .metric-item {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  padding: 8px;
  background: #f9f9f9;
  border-radius: 4px;
}

.unique-metrics .metric-item .label {
  font-size: 11px;
  color: #999;
  font-weight: 500;
  margin-bottom: 4px;
}

.unique-metrics .metric-item .value {
  font-size: 12px;
  color: #333;
  font-weight: 600;
}

.unique-metrics .metric-item .min-max-values {
  font-size: 10px;
  color: #666;
  margin-top: 4px;
  display: flex;
  align-items: center;
}

.unique-metrics .metric-item .max-value {
  color: #ff4d4f;
  font-weight: 500;
}

.unique-metrics .metric-item .min-value {
  color: #52c41a;
  font-weight: 500;
}

.unique-metrics .metric-item .separator {
  color: #999;
}

.metric-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px;
  background: #f9f9f9;
  border-radius: 4px;
}

.metric-item .label {
  font-size: 11px;
  color: #999;
  font-weight: 500;
}

.metric-item .value {
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

/* New styles for execution overview */
.execution-overview {
  padding: 16px;
  background: #f5f5f5;
  border-bottom: 1px solid #e8e8e8;
}

.overview-header h3 {
  margin: 0 0 12px 0;
  font-size: 16px;
  font-weight: 600;
  color: #333;
}

.overview-content {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.overview-metrics {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.empty-state {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 200px;
  background: #f9f9f9;
  border-radius: 4px;
  color: #999;
  font-size: 14px;
}

.metric-group {
  background: white;
  border-radius: 4px;
  padding: 12px;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.05);
}

.time-bar {
  height: 10px;
  background: #e0e0e0;
  border-radius: 5px;
  overflow: hidden;
  margin-bottom: 10px;
}

.time-value {
  height: 100%;
  background: linear-gradient(to right, #4caf50, #ffc107, #673ab7, #f44336);
  border-radius: 5px;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  padding-right: 5px;
  font-size: 11px;
  font-weight: bold;
  color: white;
}

.time-breakdown {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.time-item {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 11px;
  color: #555;
  margin-bottom: 4px;
  width: 100%;
  clear: both;
}

.time-label {
  font-size: 10px;
}

.memory-metrics {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
}

.memory-item {
  display: flex;
  justify-content: space-between;
  font-size: 11px;
  color: #555;
  margin-bottom: 4px;
  width: 100%;
  clear: both;
}

.memory-label {
  font-weight: 500;
}

.memory-value {
  font-weight: 600;
  color: #333;
}

.spill-warning {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.spill-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 12px;
  background-color: #fff3e0;
  border-radius: 4px;
  border-left: 3px solid #ff9800;
  font-size: 11px;
}

.spill-label {
  font-weight: 500;
  color: #424242;
}

.spill-value {
  font-weight: 600;
  color: #f57c00;
}

.node-detail-panel {
  padding: 16px;
}

.profile-text {
  background: #f9f9f9;
  border: 1px solid #e8e8e8;
  border-radius: 4px;
  padding: 12px;
  font-size: 12px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-all;
  overflow-x: auto;
}

.info-label {
  font-weight: 500;
  color: #666;
}

.info-value {
  font-weight: 600;
  color: #333;
}
/* Top 10 Panel Styles */
.top-panel {
  padding: 16px;
}

.top-list {
  max-height: 300px;
  overflow-y: auto;
}

.top-header {
  display: flex;
  justify-content: space-between;
  padding: 8px 12px;
  background: #fafafa;
  font-weight: 600;
  font-size: 12px;
  color: #666;
}

.top-item {
  display: flex;
  justify-content: space-between;
  padding: 10px 12px;
  border-bottom: 1px solid #f0f0f0;
  font-size: 13px;
  cursor: pointer;
  transition: background 0.2s;
}

.top-item:hover {
  background: #fafafa;
}

.top-item.highlight-red {
  background: #fff1f0;
  border-left: 3px solid #f5222d;
}

.top-item.highlight-orange {
  background: #fff7e6;
  border-left: 3px solid #fa8c16;
}

.node-name {
  flex: 1;
  font-weight: 500;
}

.node-time {
  color: #666;
  font-weight: 600;
}

.overview-section {
  margin-top: 24px;
  padding-top: 16px;
  border-top: 1px solid #e8e8e8;
}

.overview-section h4 {
  font-size: 14px;
  font-weight: 600;
  margin-bottom: 12px;
  color: #333;
}

.progress-bar {
  height: 24px;
  display: flex;
  border-radius: 4px;
  overflow: hidden;
  background: #f0f0f0;
  margin: 8px 0;
}

.progress-segment {
  transition: width 0.3s;
}

.progress-segment.io {
  background: linear-gradient(90deg, #1890ff, #40a9ff);
}

.progress-segment.processing {
  background: linear-gradient(90deg, #52c41a, #73d13d);
}

.metric-details {
  margin-top: 8px;
}

.detail-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 0;
  font-size: 12px;
  color: #666;
}

.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.dot.io-dot {
  background: #1890ff;
}

.dot.processing-dot {
  background: #52c41a;
}

.metric-row {
  display: flex;
  justify-content: space-between;
  padding: 8px 0;
  font-size: 13px;
  border-bottom: 1px solid #f0f0f0;
}

.metric-row span:first-child {
  color: #666;
}

.metric-row span:last-child {
  font-weight: 600;
  color: #333;
}

.suggestions-section {
  margin-top: 24px;
  padding-top: 16px;
  border-top: 1px solid #e8e8e8;
}

.suggestions-section h4 {
  font-size: 14px;
  font-weight: 600;
  margin-bottom: 12px;
  color: #333;
  display: flex;
  align-items: center;
  gap: 6px;
}

.suggestion-box {
  border: 2px solid #fa8c16;
  border-radius: 4px;
  padding: 12px;
  background: #fffbf0;
  margin-top: 8px;
}

.suggestion-title {
  font-weight: 600;
  color: #d46b08;
  margin-bottom: 8px;
  font-size: 13px;
}

.suggestion-title a {
  color: #1890ff;
  text-decoration: none;
  margin-left: 8px;
  font-weight: normal;
}

.suggestion-title a:hover {
  text-decoration: underline;
}

.suggestion-content {
  font-size: 12px;
  color: #666;
  line-height: 1.6;
}
</style>
