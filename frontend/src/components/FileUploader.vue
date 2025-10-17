<template>
  <div class="file-uploader">
    <!-- 文本框输入区域 (主要输入方式) -->
    <div class="text-input-section">
      <div class="section-header">
        <i class="fas fa-keyboard"></i>
        <span>粘贴 Profile 文本</span>
      </div>

      <el-input
        v-model="profileText"
        type="textarea"
        :rows="10"
        placeholder="在这里粘贴完整的 StarRocks Profile 文本内容..."
        :maxlength="10000000"
        show-word-limit
        class="profile-textarea"
      />

      <div class="text-actions">
        <el-button
          type="primary"
          :loading="loading"
          :disabled="!profileText.trim()"
          @click="handleTextAnalyze"
          size="large"
        >
          <i class="fas fa-play"></i>
          开始分析
        </el-button>

        <el-button @click="clearText" :disabled="!profileText.trim()">
          清空文本
        </el-button>
      </div>
    </div>

    <!-- 分隔线 -->
    <el-divider>
      <span class="divider-text">或使用文件上传</span>
    </el-divider>

    <!-- 文件上传区域 (辅助输入方式) -->
    <div class="file-upload-section">
      <div class="section-header">
        <i class="fas fa-file-upload"></i>
        <span>上传 Profile 文件</span>
      </div>

      <el-upload
        ref="uploadRef"
        class="upload-demo"
        drag
        action=""
        :auto-upload="false"
        :multiple="false"
        :file-list="fileList"
        :before-upload="beforeUpload"
        :on-change="handleChange"
        accept=".txt,.log,.profile"
      >
        <div class="upload-content">
          <i class="fas fa-cloud-upload-alt upload-icon"></i>
          <div class="upload-text">
            <p>点击或拖拽 Profile 文件到这里</p>
            <p class="upload-hint">
              支持 .txt, .log, .profile 格式文件，最大 50MB
            </p>
          </div>
        </div>
      </el-upload>

      <div class="upload-actions" v-if="fileList.length > 0">
        <el-button
          type="primary"
          :loading="loading"
          @click="handleFileAnalyze"
          :disabled="!selectedFile"
        >
          <i class="fas fa-play"></i>
          分析上传的文件
        </el-button>

        <el-button @click="clearFile"> 移除文件 </el-button>
      </div>

      <div class="file-info" v-if="selectedFile">
        <el-alert
          :title="`已选择文件: ${selectedFile.name} (${formatFileSize(selectedFile.size)})`"
          type="success"
          :closable="false"
          show-icon
        />
      </div>
    </div>
  </div>
</template>

<script>
export default {
  name: "FileUploader",

  props: {
    loading: {
      type: Boolean,
      default: false,
    },
  },

  emits: ["file-uploaded"],

  data() {
    return {
      profileText: "",
      fileList: [],
      selectedFile: null,
    };
  },

  methods: {
    handleTextAnalyze() {
      if (!this.profileText.trim()) {
        this.$message.warning("请输入Profile文本");
        return;
      }
      this.$emit("file-uploaded", this.profileText);
    },

    clearText() {
      this.profileText = "";
    },

    beforeUpload(file) {
      const isValidType =
        ["text/plain", "application/octet-stream"].includes(file.type) ||
        file.name.endsWith(".txt") ||
        file.name.endsWith(".log") ||
        file.name.endsWith(".profile");

      if (!isValidType) {
        this.$message.error("只支持 .txt, .log, .profile 格式的文件!");
        return false;
      }

      const isLt50M = file.size / 1024 / 1024 < 50;
      if (!isLt50M) {
        this.$message.error("文件大小不能超过 50MB!");
        return false;
      }

      return false; // 不自动上传，由手动触发
    },

    handleChange(file, fileList) {
      this.fileList = fileList.slice(-1);
      this.selectedFile = file.raw;
    },

    async handleFileAnalyze() {
      if (!this.selectedFile) {
        this.$message.warning("请先选择文件");
        return;
      }

      try {
        const text = await this.selectedFile.text();
        this.$emit("file-uploaded", text);
      } catch (error) {
        this.$message.error("文件读取失败: " + error.message);
      }
    },

    clearFile() {
      this.fileList = [];
      this.selectedFile = null;
      this.$refs.uploadRef.clearFiles();
    },

    formatFileSize(bytes) {
      if (bytes === 0) return "0 B";
      const k = 1024;
      const sizes = ["B", "KB", "MB", "GB"];
      const i = Math.floor(Math.log(bytes) / Math.log(k));
      return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
    },
  },
};
</script>

<style scoped>
.file-uploader {
  width: 100%;
  max-width: 1000px;
  margin: 0 auto;
}

/* 文本输入部分 */
.text-input-section {
  margin-bottom: 20px;
  text-align: center;
}

.section-header {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 500;
  margin-bottom: 12px;
  color: #303133;
}

.section-header i {
  color: #409eff;
  font-size: 16px;
}

.profile-textarea {
  margin-bottom: 12px;
  font-family: "Monaco", "Courier New", monospace;
}

:deep(.profile-textarea textarea) {
  font-size: 12px;
  line-height: 1.5;
  color: #606266;
}

.text-actions {
  display: flex;
  gap: 12px;
  justify-content: center;
}

/* 分隔线 */
:deep(.el-divider) {
  margin: 20px 0;
  background-color: #dcdfe6;
}

.divider-text {
  color: #909399;
  font-size: 12px;
}

/* 文件上传部分 */
.file-upload-section {
  margin-bottom: 16px;
  text-align: center;
}

.upload-demo {
  width: 100%;
}

.upload-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 30px 20px;
  gap: 12px;
}

.upload-icon {
  font-size: 40px;
  color: #c0c4cc;
}

.upload-text {
  text-align: center;
}

.upload-text p {
  margin: 4px 0;
  color: #606266;
}

.upload-hint {
  font-size: 12px;
  color: #909399 !important;
}

.upload-actions {
  display: flex;
  gap: 12px;
  margin-top: 12px;
  justify-content: flex-start;
}

.file-info {
  margin-top: 12px;
}

:deep(.el-upload-dragger) {
  border-radius: 8px;
  transition: all 0.3s ease;
}

:deep(.el-upload-dragger:hover) {
  border-color: #409eff;
  background-color: #ecf5ff;
}

:deep(.el-upload-dragger.is-dragover) {
  border-color: #409eff;
  background-color: #ecf5ff;
}
</style>
