<template>
  <div class="hotspots-panel">
    <div v-if="hotspots.length === 0" class="no-hotspots">
      <i class="fas fa-check-circle"></i>
      <p>未发现性能热点问题</p>
      <small>执行计划运行良好</small>
    </div>

    <div v-else class="hotspots-container">
      <!-- Summary stats -->
      <div class="hotspots-summary">
        <el-row :gutter="20">
          <el-col
            :span="6"
            v-for="(count, severity) in hotspotsBySeverity"
            :key="severity"
          >
            <div class="severity-stat">
              <div class="severity-circle" :class="`hotspot-${severity}`">
                {{ count }}
              </div>
              <div class="severity-label">
                {{ getSeverityZhLabel(severity) }}
              </div>
            </div>
          </el-col>
        </el-row>
      </div>

      <!-- Hotspots list -->
      <div class="hotspots-list">
        <el-collapse accordion>
          <el-collapse-item
            v-for="(hotspot, index) in sortedHotspots"
            :key="index"
            :title="getHotspotTitle(hotspot)"
            :name="index"
          >
            <template #title>
              <div class="hotspot-header">
                <div class="hotspot-icon">
                  <i
                    :class="getHotspotIcon(hotspot.issue_type)"
                    :style="{ color: getSeverityColor(hotspot.severity) }"
                  ></i>
                </div>
                <div class="hotspot-info">
                  <div class="hotspot-title">{{ hotspot.issue_type }}</div>
                  <div class="hotspot-node">{{ hotspot.node_path }}</div>
                </div>
                <div class="hotspot-severity">
                  <span
                    class="severity-badge"
                    :class="`hotspot-${hotspot.severity.toLowerCase()}`"
                  >
                    {{ getSeverityZhLabel(hotspot.severity) }}
                  </span>
                </div>
              </div>
            </template>

            <div class="hotspot-detail">
              <div class="hotspot-description">
                <h5>问题描述</h5>
                <p>{{ hotspot.description }}</p>
              </div>

              <div
                v-if="hotspot.suggestions && hotspot.suggestions.length > 0"
                class="hotspot-suggestions"
              >
                <h5>优化建议</h5>
                <ol>
                  <li
                    v-for="suggestion in hotspot.suggestions"
                    :key="suggestion"
                  >
                    {{ suggestion }}
                  </li>
                </ol>
              </div>
            </div>
          </el-collapse-item>
        </el-collapse>
      </div>
    </div>
  </div>
</template>

<script>
export default {
  name: "HotSpotsPanel",

  props: {
    hotspots: {
      type: Array,
      default: () => [],
    },
  },

  computed: {
    hotspotsBySeverity() {
      return this.hotspots.reduce((acc, hotspot) => {
        const severity = hotspot.severity.toLowerCase();
        acc[severity] = (acc[severity] || 0) + 1;
        return acc;
      }, {});
    },

    sortedHotspots() {
      // Sort hotspots by severity (critical first, then high, severe, etc.)
      const severityOrder = {
        critical: 0,
        high: 1,
        severe: 2,
        moderate: 3,
        mild: 4,
        normal: 5,
      };

      return [...this.hotspots].sort((a, b) => {
        const aOrder = severityOrder[a.severity.toLowerCase()] || 999;
        const bOrder = severityOrder[b.severity.toLowerCase()] || 999;
        return aOrder - bOrder;
      });
    },
  },

  methods: {
    getSeverityZhLabel(severity) {
      const labels = {
        critical: "严重",
        high: "高",
        severe: "重度",
        moderate: "中度",
        mild: "轻度",
        normal: "正常",
      };
      return labels[severity.toLowerCase()] || severity;
    },

    getSeverityColor(severity) {
      const colors = {
        critical: "#f5222d",
        high: "#722ed1",
        severe: "#fa541a",
        moderate: "#fa8c16",
        mild: "#faad14",
        normal: "#52c41a",
      };
      return colors[severity.toLowerCase()] || "#1890ff";
    },

    getHotspotTitle(hotspot) {
      // 移除硬编码，使用通用逻辑：将英文转中文（如果有翻译配置），否则直接显示
      // 未来可从后端获取翻译配置
      return hotspot.issue_type; // 保持通用，显示原始类型
    },

    getHotspotIcon(issueType) {
      // 移除硬编码，使用通用的图标映射规则
      // 根据关键词智能匹配图标
      const type = issueType.toLowerCase();

      if (type.includes("segment") || type.includes("fragmentation"))
        return "fas fa-puzzle-piece";
      if (type.includes("connector") || type.includes("scan"))
        return "fas fa-plug";
      if (type.includes("memory")) return "fas fa-memory";
      if (type.includes("io") || type.includes("disk")) return "fas fa-hdd";
      if (type.includes("cpu")) return "fas fa-microchip";
      if (type.includes("network")) return "fas fa-network-wired";
      if (type.includes("join")) return "fas fa-code-branch";
      if (type.includes("aggregate") || type.includes("agg"))
        return "fas fa-layer-group";
      if (type.includes("sort")) return "fas fa-sort-amount-down";
      if (type.includes("exchange")) return "fas fa-exchange-alt";

      return "fas fa-exclamation-triangle"; // 默认图标
    },
  },
};
</script>

<style scoped>
.hotspots-panel {
  width: 100%;
}

.no-hotspots {
  text-align: center;
  padding: 40px 20px;
  color: #52c41a;
}

.no-hotspots i {
  font-size: 48px;
  margin-bottom: 16px;
  opacity: 0.8;
}

.no-hotspots p {
  font-size: 18px;
  font-weight: 500;
  margin-bottom: 8px;
}

.no-hotspots small {
  color: #909399;
}

.hotspots-container {
  width: 100%;
}

.hotspots-summary {
  margin-bottom: 24px;
  padding: 20px;
  background: #fafbfc;
  border-radius: 8px;
}

.severity-stat {
  text-align: center;
}

.severity-circle {
  width: 48px;
  height: 48px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  margin: 0 auto 8px;
  font-size: 16px;
  font-weight: bold;
  color: white;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

.severity-label {
  font-size: 12px;
  color: #666;
  font-weight: 500;
}

.hotspots-list {
  margin-top: 16px;
}

.hotspot-header {
  display: flex;
  align-items: center;
  width: 100%;
  gap: 12px;
}

.hotspot-icon {
  flex-shrink: 0;
}

.hotspot-icon i {
  font-size: 18px;
}

.hotspot-info {
  flex: 1;
}

.hotspot-title {
  font-weight: 600;
  color: #303133;
  font-size: 14px;
}

.hotspot-node {
  font-size: 12px;
  color: #909399;
  margin-top: 2px;
}

.hotspot-severity {
  flex-shrink: 0;
}

.severity-badge {
  padding: 2px 8px;
  border-radius: 12px;
  font-size: 11px;
  font-weight: bold;
  color: white;
  text-transform: uppercase;
}

.hotspot-detail {
  padding: 16px 0;
}

.hotspot-detail h5 {
  color: #303133;
  font-size: 14px;
  font-weight: 600;
  margin-bottom: 8px;
  margin-top: 16px;
}

.hotspot-detail h5:first-child {
  margin-top: 0;
}

.hotspot-description p {
  color: #606266;
  line-height: 1.6;
  margin-bottom: 16px;
}

.hotspot-suggestions {
  margin-top: 16px;
}

.hotspot-suggestions ol {
  padding-left: 20px;
}

.hotspot-suggestions li {
  color: #606266;
  line-height: 1.6;
  margin-bottom: 8px;
}

.hotspot-suggestions li:last-child {
  margin-bottom: 0;
}

:deep(.el-collapse-item__header) {
  background: #fafbfc;
  border-bottom: 1px solid #f0f2f5;
}

:deep(.el-collapse-item__header.is-active) {
  background: #f5f7fa;
  border-bottom-color: #e4e7ed;
}

:deep(.el-collapse-item__wrap) {
  border-bottom: 1px solid #f0f2f5;
}
</style>
