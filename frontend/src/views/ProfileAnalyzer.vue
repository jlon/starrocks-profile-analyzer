<template>
  <div class="profile-analyzer">
    <el-container class="main-container">
      <el-header class="app-header">
        <h1 class="title">
          <i class="fas fa-chart-line"></i>
          StarRocks Profile 分析器
        </h1>
      </el-header>

      <el-container class="content-container">
        <el-main class="main-content">
          <!-- 上传区域（分析前显示） -->
          <div v-if="!isAnalyzing" class="upload-container">
            <el-row :gutter="20">
              <el-col :span="24">
                <el-card shadow="hover" class="upload-section">
                  <template #header>
                    <div class="card-header">
                      <i class="fas fa-upload"></i>
                      上传Profile文件
                    </div>
                  </template>

                  <FileUploader
                    @file-uploaded="handleFileUpload"
                    :loading="loading"
                  />

                  <div v-if="error" class="error-message">
                    <el-alert
                      :title="error"
                      type="error"
                      show-icon
                      :closable="false"
                    />
                  </div>
                </el-card>
              </el-col>
            </el-row>
          </div>

          <!-- 执行计划可视化（分析后显示） -->
          <div v-if="isAnalyzing" class="visualization-container">
            <el-row :gutter="20">
              <el-col :span="24">
                <el-card shadow="hover" class="visualization-section">
                  <template #header>
                    <div class="card-header">
                      <el-button
                        @click="goBack"
                        type="primary"
                        size="small"
                        icon="el-icon-arrow-left"
                        style="margin-right: 10px"
                      >
                        返回
                      </el-button>
                      <i class="fas fa-project-diagram"></i>
                      执行计划可视化
                    </div>
                  </template>

                  <ExecutionPlanVisualization :result="analysisResult" />
                </el-card>
              </el-col>
            </el-row>
          </div>
        </el-main>
      </el-container>
    </el-container>
  </div>
</template>

<script>
import { mapState, mapGetters } from "vuex";
import FileUploader from "../components/FileUploader.vue";
import ExecutionPlanVisualization from "../components/ExecutionPlanVisualization.vue";

export default {
  name: "ProfileAnalyzer",

  components: {
    FileUploader,
    ExecutionPlanVisualization,
  },

  data() {
    return {
      isAnalyzing: false,
    };
  },

  computed: {
    ...mapState(["analysisResult", "loading", "error"]),
    ...mapGetters(["hasAnalysisResult"]),
  },

  watch: {
    hasAnalysisResult(newVal) {
      if (newVal) {
        this.isAnalyzing = true;
      }
    },
  },

  methods: {
    async handleFileUpload(file) {
      await this.$store.dispatch("analyzeProfile", file);
    },

    goBack() {
      this.isAnalyzing = false;
      this.$store.commit("clearAnalysisResult");
    },
  },
};
</script>

<style scoped>
.profile-analyzer {
  height: 100vh;
  background-color: #f5f5f5;
}

.main-container {
  height: 100%;
}

.app-header {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
  display: flex;
  align-items: center;
  padding: 0 20px;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.1);
}

.title {
  font-size: 1.5rem;
  font-weight: 600;
  margin: 0;
  display: flex;
  align-items: center;
  gap: 10px;
}

.content-container {
  height: calc(100vh - 60px);
}

.main-content {
  padding: 20px;
  overflow-y: auto;
}

.card-header {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 600;
  color: #303133;
}

.upload-section,
.visualization-section {
  height: fit-content;
}

.upload-container,
.visualization-container {
  height: 100%;
}

.error-message {
  margin-top: 16px;
}
</style>
