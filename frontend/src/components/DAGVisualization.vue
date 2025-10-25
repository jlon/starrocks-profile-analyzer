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
            <!-- 箭头标记 -->
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
                :marker-start="`url(#circle-${link.isHotspot ? 'red' : 'normal'})`"
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
                  {{ formatDuration(node.metrics.operator_total_time_raw || node.metrics.operator_total_time) }}
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
      <div class="detail-panel">
        <!-- 执行概览（未选中节点时显示） -->
        <div v-if="!selectedNodeId" class="execution-overview">
          <div class="overview-header">
            <h3>执行概览</h3>
                    </div>

          <div class="overview-content">
            <div v-if="summary && summary.total_time_ms" class="overview-metrics">
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

              <!-- 内存 -->
              <div class="metric-group" style="margin-top: 20px">
                <h5>Memory</h5>
                <div class="memory-metrics">
                  <div class="memory-item">
                    <span class="memory-label">AllocatedMemoryUsage</span>
                    <span class="memory-value">{{
                      formatBytes(summary.query_allocated_memory)
                    }}</span>
                  </div>
                  <div class="memory-item">
                    <span class="memory-label">PeakMemoryUsage</span>
                    <span class="memory-value">{{
                      formatBytes(summary.query_peak_memory)
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
                  <h5>Execution time</h5>
                  <div class="execution-time-breakdown">
                    <!-- 基础执行时间 -->
                    <div v-if="selectedNode.metrics.operator_total_time" class="time-metric">
                      <span class="metric-name">Total Time</span>
                      <span class="metric-value">{{ formatDuration(selectedNode.metrics.operator_total_time) }}</span>
                    </div>
                    
                    <!-- 操作符特定的时间分解 -->
                    <template v-if="selectedNode.operator_name === 'OLAP_SCAN'">
                      <div v-if="selectedNode.metrics.specialized?.scan_time" class="time-metric">
                        <span class="metric-name">ScanTime</span>
                        <span class="metric-value">{{ formatDuration(selectedNode.metrics.specialized.scan_time) }}</span>
                    </div>
                      <div v-if="selectedNode.metrics.specialized?.io_time" class="time-metric">
                        <span class="metric-name">IOTime</span>
                        <span class="metric-value">{{ formatDuration(selectedNode.metrics.specialized.io_time) }}</span>
                    </div>
                    </template>
                    
                    <template v-if="selectedNode.operator_name === 'EXCHANGE'">
                      <div v-if="selectedNode.metrics.specialized?.network_time" class="time-metric">
                        <span class="metric-name">NetworkTime</span>
                        <span class="metric-value">{{ formatDuration(selectedNode.metrics.specialized.network_time) }}</span>
                    </div>
                      <div v-if="selectedNode.metrics.specialized?.overall_time" class="time-metric">
                        <span class="metric-name">OverallTime</span>
                        <span class="metric-value">{{ formatDuration(selectedNode.metrics.specialized.overall_time) }}</span>
                    </div>
                    </template>
                  </div>
                </div>

                <!-- 通用执行指标 -->
                <div class="metrics-section" style="margin-top: 20px">
                  <h5>执行指标</h5>
                  <div class="metrics-grid">
                    <div v-if="selectedNode.metrics.push_chunk_num" class="metric">
                      <span class="metric-label">推入数据块</span>
                      <span class="metric-value">{{
                        selectedNode.metrics.push_chunk_num || "N/A"
                      }}</span>
                    </div>
                    <div v-if="selectedNode.metrics.push_row_num" class="metric">
                      <span class="metric-label">推入行数</span>
                      <span class="metric-value">{{
                        selectedNode.metrics.push_row_num || "N/A"
                      }}</span>
                    </div>
                    <div v-if="selectedNode.metrics.pull_chunk_num" class="metric">
                      <span class="metric-label">拉取数据块</span>
                      <span class="metric-value">{{
                        selectedNode.metrics.pull_chunk_num || "N/A"
                      }}</span>
                    </div>
                    <div v-if="selectedNode.metrics.pull_row_num" class="metric">
                      <span class="metric-label">拉取行数</span>
                      <span class="metric-value">{{
                        selectedNode.metrics.pull_row_num || "N/A"
                      }}</span>
                    </div>
                  </div>
                </div>

                <!-- 操作符特定指标 -->
                <div v-if="hasSpecializedMetrics(selectedNode)" class="metrics-section" style="margin-top: 20px">
                  <h5>{{ selectedNode.operator_name }} 专用指标</h5>
                  <div class="specialized-metrics">
                    <!-- 动态渲染所有专用指标 -->
                    <template v-for="(specObj, specKey) in getValidSpecializedMetrics(selectedNode)" :key="specKey">
                      <div v-for="(value, key) in specObj" :key="`${specKey}-${key}`" class="metric-item">
                        <span class="label">{{ formatMetricKey(key) }}:</span>
                        <span class="value">{{ formatMetricValue(value) }}</span>
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
      
      maxTime: 0,
      totalTimeMs: 0,
      activeTab: "overview", // Default to overview tab
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
      const levelHeight = 150; // 垂直间距
      const levelWidth = 200; // 水平间距
      
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
        const y = depth * levelHeight + 50;
        const totalWidth = (nodesCountInLevel - 1) * levelWidth;
        const centerX = this.svgWidth / 2;
        const x = centerX - totalWidth / 2 + indexInLevel * levelWidth;
        
        return {
          ...node,
          x,
          y,
        };
      });

      // 构建连接线（箭头从下指向上）
      this.links = [];
      this.executionTree.nodes.forEach((sourceNode) => {
        if (!sourceNode || !sourceNode.children) return;
        sourceNode.children.forEach((childId) => {
          const targetNode = nodeMap.get(childId);
          if (targetNode && targetNode.metrics) {
            const source = this.nodes.find((n) => n.id === sourceNode.id);
            const target = this.nodes.find((n) => n.id === targetNode.id);
            
            if (source && target && source.metrics && target.metrics) {
              // 箭头从child指向parent（从下往上）
              const startX = target.x + this.NODE_WIDTH / 2;
              const startY = target.y + this.NODE_HEIGHT; // From bottom of child
              const endX = source.x + this.NODE_WIDTH / 2;
              const endY = source.y; // To top of parent

              const controlY = (startY + endY) / 2;
              const path = `M ${startX} ${startY} C ${startX} ${controlY}, ${endX} ${controlY}, ${endX} ${endY}`;

              // 显示行数在箭头中点
              const rows = this.getNodeRows(targetNode);
              
              this.links.push({
                id: `${source.id}-${target.id}`,
                path,
                labelX: (startX + endX) / 2,
                labelY: controlY - 8,
                label: this.formatRows(rows),
                isHotspot: source.is_hotspot || target.is_hotspot,
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
        // Backend now returns nanoseconds directly
        totalNanos = duration;
      }
      
      if (totalNanos === 0) return "0纳秒";
      
      // Calculate time units
      const hours = Math.floor(totalNanos / (3600 * 1_000_000_000));
      const minutes = Math.floor((totalNanos % (3600 * 1_000_000_000)) / (60 * 1_000_000_000));
      const seconds = Math.floor((totalNanos % (60 * 1_000_000_000)) / 1_000_000_000);
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
        return parts.slice(0, 3).join('');
      } else {
        return parts.join('');
      }
    },

    formatChineseDurationString(durationStr) {
      // Handle string duration formats like "1h2m3s4ms5us6ns"
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
          case 'h':
            parts.push(`${Math.floor(value)}时`);
            break;
          case 'm':
            parts.push(`${Math.floor(value)}分`);
            break;
          case 's':
            parts.push(`${Math.floor(value)}秒`);
            break;
          case 'ms':
            parts.push(`${Math.floor(value)}毫秒`);
            break;
          case 'us':
            parts.push(`${Math.floor(value)}微秒`);
            break;
          case 'ns':
            parts.push(`${Math.floor(value)}纳秒`);
            break;
        }
      }
      
      return parts.length > 0 ? parts.join('') : durationStr;
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

    getPercentage(node) {
      // 优先使用后端计算的time_percentage（这是官方StarRocks解析逻辑）
      if (node && node.time_percentage !== undefined && node.time_percentage !== null) {
        return node.time_percentage.toFixed(2);
      }
      
      // 回退到前端计算（仅用于兼容旧数据）
      if (!node || !node.metrics) return 0;
      const nodeMs = this.getDurationMs(node.metrics.operator_total_time_raw || node.metrics.operator_total_time);
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
          const nodeMs = this.getDurationMs(node.metrics.operator_total_time_raw || node.metrics.operator_total_time);
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
      return this.summary.total_time_ms || 0;
    },

    getTimeBreakdown() {
      if (!this.summary) return [];
      const totalMs = this.summary.total_time_ms || 0;
      const pushMs = this.summary.push_total_time || 0;
      const pullMs = this.summary.pull_total_time || 0;

      if (totalMs === 0) return [];

      const otherMs = Math.max(0, totalMs - pushMs - pullMs);
      const breakdown = [
        {
          name: "Total Execution Time",
          duration: this.summary.total_time,
          percent: 100,
        },
        {
          name: "Push Time",
          duration: this.formatDuration(pushMs),
          percent: totalMs > 0 ? ((pushMs / totalMs) * 100).toFixed(2) : 0,
        },
        {
          name: "Pull Time",
          duration: this.formatDuration(pullMs),
          percent: totalMs > 0 ? ((pullMs / totalMs) * 100).toFixed(2) : 0,
        },
        {
          name: "Other Time",
          duration: this.formatDuration(otherMs),
          percent: totalMs > 0 ? ((otherMs / totalMs) * 100).toFixed(2) : 0,
        },
      ];
      try {
        console.log("[DAG] getTimeBreakdown", {
          totalMs,
          pushMs,
          pullMs,
          breakdown,
        });
      } catch (e) {
        console.warn("[DAG] log error", e);
      }
      return breakdown;
    },

    getTimeColors() {
      return ["#4CAF50", "#FFC107", "#673AB7", "#F44336"]; // Green, Amber, Purple, Red
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
      return typeof specialized === 'object' && specialized !== null && !Array.isArray(specialized) && Object.keys(specialized).length > 0;
    },

    getValidSpecializedMetrics(node) {
      if (!this.hasSpecializedMetrics(node)) return {};
      const specialized = node.metrics.specialized;
      
      // Check if specialized is a valid object (not a string like "None")
      if (typeof specialized !== 'object' || specialized === null || Array.isArray(specialized)) {
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

    formatMetricKey(key) {
      // Convert camelCase to readable format, ensure key is string
      if (typeof key !== 'string') {
        key = String(key);
      }
      return key.replace(/([A-Z])/g, ' $1').trim();
    },

    formatMetricValue(value) {
      if (value === null || value === undefined) {
        return 'N/A';
      }
      // Check if it's a duration object (with secs and nanos)
      if (typeof value === 'object' && !Array.isArray(value) && ('secs' in value || 'nanos' in value)) {
        return this.formatDuration(value);
      }
      if (typeof value === 'number') {
        return value.toString();
      } else if (typeof value === 'string') {
        return value;
      } else if (typeof value === 'boolean') {
        return value ? 'Yes' : 'No';
      } else if (Array.isArray(value)) {
        return value.length > 0 ? `[${value.length} items]` : '[]';
      } else if (typeof value === 'object') {
        return JSON.stringify(value, null, 2);
      }
      return String(value);
    }
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
  0%,
  100% {
    opacity: 1;
    r: 4;
  }
  50% {
    opacity: 0.6;
    r: 5;
  }
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

.specialized-metrics {
  display: flex;
  flex-direction: column;
  gap: 8px;
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
}

.memory-label {
  font-weight: 500;
}

.memory-value {
  font-weight: 600;
  color: #333;
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
</style>
