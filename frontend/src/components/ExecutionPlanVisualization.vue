<template>
  <div class="execution-plan-viz">
    <!-- DAG 图表视图 - 全屏显示 -->
    <div class="dag-container-wrapper">
      <DAGVisualization
        v-if="result.execution_tree"
        :executionTree="result.execution_tree"
        :summary="result.summary"
      />
    </div>

    <!-- 详情面板（保持不变） -->
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
              {{ selectedNode.is_hotspot ? "是" : "否" }}
            </el-tag>
          </el-descriptions-item>
          <el-descriptions-item
            label="热点严重程度"
            v-if="selectedNode.is_hotspot"
          >
            <span
              :class="`hotspot-${selectedNode.hotspot_severity.toLowerCase()}`"
            >
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
                    <span class="metric-value">{{ formatDuration(node.metrics.operator_total_time_raw || node.metrics.operator_total_time) }}</span>
                  </div>
                  <div class="metric-item">
                    <span class="metric-label">推入数据块:</span>
                    <span class="metric-value">{{
                      selectedNode.metrics.push_chunk_num || "N/A"
                    }}</span>
                  </div>
                  <div class="metric-item">
                    <span class="metric-label">推入行数:</span>
                    <span class="metric-value">{{
                      selectedNode.metrics.push_row_num || "N/A"
                    }}</span>
                  </div>
                  <div class="metric-item">
                    <span class="metric-label">内存使用:</span>
                    <span class="metric-value">{{
                      formatBytes(selectedNode.metrics.memory_usage)
                    }}</span>
                  </div>
                </div>
              </el-card>
            </el-col>

            <el-col :span="12" v-if="selectedNode.metrics.specialized">
              <el-card size="small" shadow="hover">
                <template #header>专业指标</template>
                <div class="metric-grid">
                  <template
                    v-if="selectedNode.metrics.specialized.ConnectorScan"
                  >
                    <div class="metric-item">
                      <span class="metric-label">读取字节:</span>
                      <span class="metric-value">{{
                        formatBytes(
                          selectedNode.metrics.specialized.ConnectorScan
                            .bytes_read,
                        )
                      }}</span>
                    </div>
                    <div class="metric-item">
                      <span class="metric-label">读取行数:</span>
                      <span class="metric-value">{{
                        selectedNode.metrics.specialized.ConnectorScan
                          .rows_read || "N/A"
                      }}</span>
                    </div>
                    <div class="metric-item">
                      <span class="metric-label">IO时间:</span>
                      <span class="metric-value">{{
                        formatDuration(
                          selectedNode.metrics.specialized.ConnectorScan
                            .io_time,
                        )
                      }}</span>
                    </div>
                  </template>
                </div>
              </el-card>
            </el-col>
          </el-row>
        </div>

        <div
          class="children-section"
          v-if="selectedNode.children && selectedNode.children.length > 0"
        >
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
import DAGVisualization from "./DAGVisualization.vue";

export default {
  name: "ExecutionPlanVisualization",

  components: {
    DAGVisualization,
  },

  props: {
    result: {
      type: Object,
      required: true,
    },
  },

  data() {
    return {
      detailVisible: false,
      selectedNode: null,
    };
  },

  methods: {
    closeDetailModal(done) {
      done();
    },

    // 移除硬编码的 getNodeTypeLabel，使用 setup() 中的

    getSeverityLabel(severity) {
      const labels = {
        CRITICAL: "严重",
        WARNING: "警告",
        NORMAL: "正常",
      };
      return labels[severity] || severity;
    },

    formatDuration(duration) {
      if (duration === null || duration === undefined) return "N/A";
      if (typeof duration === "string") {
        return this.toChineseDurationString(duration);
      }
      const ms = this.getDurationMs(duration);
      if (ms < 1) return `${ms.toFixed(3)}毫秒`;
      if (ms < 1000) return `${ms.toFixed(3)}毫秒`;
      const totalSeconds = ms / 1000;
      if (totalSeconds < 60) return `${totalSeconds.toFixed(2)}秒`;
      const hours = Math.floor(totalSeconds / 3600);
      const minutes = Math.floor((totalSeconds % 3600) / 60);
      const seconds = Math.floor(totalSeconds % 60);
      const millis = Math.floor(ms % 1000);
      if (hours > 0) {
        return `${hours}小时${minutes}分钟`;
      }
      if (minutes > 0) {
        return `${minutes}分钟${seconds}秒`;
      }
      return `${seconds}秒${millis}毫秒`;
    },

    getDurationMs(duration) {
      if (!duration) return 0;
      if (typeof duration === "number") return duration;
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

    onNodeClick(node) {
      this.selectedNode = node;
      this.detailVisible = true;
    },
  },
};
</script>

<style scoped>
.execution-plan-viz {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  background: #fff;
}

.dag-container-wrapper {
  flex: 1;
  width: 100%;
  overflow: hidden;
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

.hotspot-critical {
  color: #f5222d;
  font-weight: bold;
}

.hotspot-severe {
  color: #fa541a;
  font-weight: bold;
}

.hotspot-warning {
  color: #faad14;
  font-weight: bold;
}

.hotspot-normal {
  color: #52c41a;
}
</style>
