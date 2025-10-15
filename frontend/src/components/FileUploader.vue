<template>
  <div class="file-uploader">
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
          <p>点击或拖拽 Profile 文件到这里上传</p>
          <p class="upload-hint">支持 .txt, .log, .profile 格式文件</p>
        </div>
      </div>
    </el-upload>

    <div class="upload-actions" v-if="fileList.length > 0">
      <el-button
        type="primary"
        :loading="loading"
        @click="handleAnalyze"
        :disabled="!selectedFile"
      >
        开始分析
      </el-button>

      <el-button @click="clearFile">
        清空文件
      </el-button>
    </div>

    <div class="file-info" v-if="selectedFile">
      <el-descriptions :column="1" size="small" border>
        <el-descriptions-item label="文件名">
          {{ selectedFile.name }}
        </el-descriptions-item>
        <el-descriptions-item label="文件大小">
          {{ formatFileSize(selectedFile.size) }}
        </el-descriptions-item>
      </el-descriptions>
    </div>
  </div>
</template>

<script>
export default {
  name: 'FileUploader',

  props: {
    loading: {
      type: Boolean,
      default: false
    }
  },

  emits: ['file-uploaded'],

  data() {
    return {
      fileList: [],
      selectedFile: null
    }
  },

  methods: {
    beforeUpload(file) {
      const isValidType = ['text/plain', 'application/octet-stream'].includes(file.type) ||
                         file.name.endsWith('.txt') ||
                         file.name.endsWith('.log') ||
                         file.name.endsWith('.profile')

      if (!isValidType) {
        this.$message.error('只支持 .txt, .log, .profile 格式的文件!')
        return false
      }

      const isLt50M = file.size / 1024 / 1024 < 50
      if (!isLt50M) {
        this.$message.error('文件大小不能超过 50MB!')
        return false
      }

      return false // 不自动上传，由手动触发
    },

    handleChange(file, fileList) {
      this.fileList = fileList.slice(-1) // 只保留最后一个文件
      this.selectedFile = file.raw
    },

    async handleAnalyze() {
      if (!this.selectedFile) {
        this.$message.warning('请先选择文件')
        return
      }

      try {
        const text = await this.selectedFile.text()
        this.$emit('file-uploaded', text)
      } catch (error) {
        this.$message.error('文件读取失败: ' + error.message)
      }
    },

    clearFile() {
      this.fileList = []
      this.selectedFile = null
      this.$refs.uploadRef.clearFiles()
    },

    formatFileSize(bytes) {
      if (bytes === 0) return '0 B'
      const k = 1024
      const sizes = ['B', 'KB', 'MB', 'GB']
      const i = Math.floor(Math.log(bytes) / Math.log(k))
      return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
    }
  }
}
</script>

<style scoped>
.file-uploader {
  width: 100%;
}

.upload-demo {
  width: 100%;
}

.upload-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 20px;
  gap: 16px;
}

.upload-icon {
  font-size: 48px;
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
  margin-top: 16px;
  justify-content: center;
}

.file-info {
  margin-top: 16px;
}

:deep(.el-upload-dragger) {
  border-radius: 8px;
}

:deep(.el-upload-dragger:hover) {
  border-color: #409eff;
  background-color: #ecf5ff;
}
</style>
