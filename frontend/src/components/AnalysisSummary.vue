<template>
  <div class="analysis-summary">
    <el-row :gutter="20">
      <el-col :span="12">
        <div class="metric-card">
          <i class="fas fa-tachometer-alt metric-icon performance"></i>
          <div class="metric-content">
            <h3>性能评分</h3>
            <p class="performance-score">{{ performanceScore.toFixed(1) }}%</p>
          </div>
        </div>
      </el-col>

      <el-col :span="12">
        <div class="metric-card">
          <i class="fas fa-exclamation-triangle metric-icon warnings"></i>
          <div class="metric-content">
            <h3>热点数量</h3>
            <p class="hotspots-count">{{ hotspotsCount }}</p>
          </div>
        </div>
      </el-col>
    </el-row>

    <div class="conclusion-section">
      <h4>分析结论</h4>
      <p class="conclusion">{{ result.conclusion }}</p>
    </div>

    <div class="hotspots-overview" v-if="hotspotsCount > 0">
      <h4>热点分布</h4>
      <el-row :gutter="10">
        <el-col
          v-for="(count, severity) in hotspotsBySeverity"
          :key="severity"
          :span="4"
        >
          <div class="severity-indicator">
            <span class="severity-dot" :class="`hotspot-${severity}`">
              {{ count }}
            </span>
            <span class="severity-label">{{ getSeverityLabel(severity) }}</span>
          </div>
        </el-col>
      </el-row>
    </div>

    <div
      class="suggestions-section"
      v-if="result.suggestions && result.suggestions.length > 0"
    >
      <h4>优化建议</h4>
      <el-collapse>
        <el-collapse-item
          v-for="(suggestion, index) in result.suggestions"
          :key="index"
          :title="`建议 ${index + 1}`"
        >
          <p>{{ suggestion }}</p>
        </el-collapse-item>
      </el-collapse>
    </div>
  </div>
</template>

<script>
export default {
  name: "AnalysisSummary",

  props: {
    result: {
      type: Object,
      required: true,
    },
  },

  computed: {
    performanceScore() {
      return this.result.performance_score || 0;
    },

    hotspotsCount() {
      return this.result.hotspots ? this.result.hotspots.length : 0;
    },

    hotspotsBySeverity() {
      if (!this.result.hotspots) return {};

      return this.result.hotspots.reduce((acc, hotspot) => {
        const severity = hotspot.severity.toLowerCase();
        acc[severity] = (acc[severity] || 0) + 1;
        return acc;
      }, {});
    },
  },

  methods: {
    getSeverityLabel(severity) {
      const labels = {
        normal: "正常",
        mild: "轻微",
        moderate: "中等",
        severe: "严重",
        critical: "严重",
        high: "高",
      };
      return labels[severity] || severity;
    },
  },
};
</script>

<style scoped>
.analysis-summary {
  width: 100%;
}

.metric-card {
  display: flex;
  align-items: center;
  padding: 20px;
  background: linear-gradient(135deg, #f5f7fa 0%, #c3cfe2 100%);
  border-radius: 8px;
  margin-bottom: 16px;
}

.metric-icon {
  font-size: 24px;
  margin-right: 16px;
}

.metric-icon.performance {
  color: #52c41a;
}

.metric-icon.warnings {
  color: #faad14;
}

.metric-content h3 {
  margin: 0 0 8px 0;
  font-size: 14px;
  color: #666;
  font-weight: 500;
}

.performance-score {
  font-size: 28px;
  font-weight: bold;
  color: #52c41a;
  margin: 0;
}

.hotspots-count {
  font-size: 28px;
  font-weight: bold;
  color: #fa541a;
  margin: 0;
}

.conclusion-section,
.hotspots-overview,
.suggestions-section {
  margin-top: 24px;
}

.conclusion-section h4,
.hotspots-overview h4,
.suggestions-section h4 {
  margin-bottom: 12px;
  font-weight: 600;
  color: #303133;
}

.conclusion {
  text-align: justify;
  line-height: 1.6;
  color: #606266;
}

.severity-indicator {
  text-align: center;
  margin-bottom: 8px;
}

.severity-dot {
  display: inline-block;
  width: 32px;
  height: 32px;
  border-radius: 50%;
  text-align: center;
  line-height: 32px;
  font-weight: bold;
  color: white;
  margin-bottom: 4px;
}

.severity-label {
  display: block;
  font-size: 12px;
  color: #909399;
}

:deep(.el-collapse-item__header) {
  font-weight: 500;
}

:deep(.el-collapse-item__content) {
  text-align: justify;
  line-height: 1.6;
}
</style>
