<template>
  <div class="text-input-section">
    <!-- 文本粘贴区域 -->
    <div class="text-input-container">
      <div class="input-header">
        <div class="input-title">
          <i class="fas fa-edit"></i>
          Profile 文本粘贴
        </div>
        <div class="input-stats" v-if="textContent">
          <el-tag size="small" type="info">
            {{ textStats.lines }} 行, {{ textStats.characters }} 字符
          </el-tag>
        </div>
      </div>

      <div class="text-input-wrapper">
        <textarea
          ref="textInput"
          v-model="textContent"
          class="profile-textarea"
          :class="{
            'has-content': textContent,
            'has-error': validationError,
          }"
          placeholder="请粘贴 StarRocks Profile 文本内容到此处...
或使用 Ctrl+V 粘贴

支持的格式：
• Query 统计信息
• 执行计划详情
• 性能指标数据"
          @paste="handlePaste"
          @input="handleInput"
          :disabled="disabled"
        ></textarea>

        <!-- 粘贴提示遮罩 -->
        <div v-if="!textContent && !disabled" class="paste-overlay">
          <div class="paste-hint">
            <i class="fas fa-paste"></i>
            <p>在此处粘贴 Profile 文本</p>
            <small>或使用 Ctrl+V</small>
          </div>
        </div>
      </div>

      <!-- 格式验证状态 -->
      <div class="validation-status" v-if="textContent">
        <el-alert
          v-if="validationError"
          :title="validationError"
          type="error"
          show-icon
          :closable="false"
        />
        <el-alert
          v-else-if="isValidFormat"
          title="文本格式验证通过"
          type="success"
          show-icon
          :closable="false"
        />
      </div>
    </div>

    <!-- 小型文件上传区域 -->
    <div class="file-upload-compact">
      <el-divider content-position="center">
        <span class="divider-text">或从文件导入</span>
      </el-divider>

      <div class="compact-upload">
        <el-upload
          ref="uploadRef"
          class="compact-upload-demo"
          drag
          action=""
          :auto-upload="false"
          :multiple="false"
          :file-list="fileList"
          :before-upload="beforeUpload"
          :on-change="handleFileChange"
          accept=".txt,.log,.profile"
        >
          <div class="compact-upload-content">
            <i class="fas fa-file-upload"></i>
            <div class="upload-text">
              <p>拖拽文件到此处或</p>
              <el-button size="small" type="primary" plain>
                选择文件
              </el-button>
            </div>
          </div>
        </el-upload>
      </div>
    </div>

    <!-- 操作按钮区域 -->
    <div class="input-actions">
      <el-space wrap>
        <el-button
          type="primary"
          :loading="loading"
          @click="handleAnalyze"
          :disabled="!canAnalyze"
        >
          <i class="fas fa-play"></i>
          开始分析
        </el-button>

        <el-button v-if="textContent" @click="copyToClipboard">
          <i class="fas fa-copy"></i>
          复制内容
        </el-button>

        <el-button v-if="textContent" @click="exportToFile">
          <i class="fas fa-download"></i>
          导出文件
        </el-button>

        <el-button
          @click="clearContent"
          :disabled="!textContent && fileList.length === 0"
        >
          <i class="fas fa-trash"></i>
          清空内容
        </el-button>
      </el-space>
    </div>
  </div>
</template>

<script>
export default {
  name: "TextInput",

  props: {
    loading: {
      type: Boolean,
      default: false,
    },
    disabled: {
      type: Boolean,
      default: false,
    },
  },

  emits: ["text-analyze", "file-analyze"],

  data() {
    return {
      textContent: "",
      fileList: [],
      validationError: null,
      isValidFormat: false,
    };
  },

  computed: {
    textStats() {
      if (!this.textContent) {
        return { lines: 0, characters: 0 };
      }

      const lines = this.textContent.split("\n").length;
      const characters = this.textContent.length;

      return { lines, characters };
    },

    canAnalyze() {
      return (
        (this.textContent && this.isValidFormat) ||
        (this.fileList.length > 0 && this.selectedFile)
      );
    },

    selectedFile() {
      return this.fileList.length > 0 ? this.fileList[0].raw : null;
    },
  },

  mounted() {
    // 自动聚焦到文本框
    this.$nextTick(() => {
      if (this.$refs.textInput) {
        this.$refs.textInput.focus();
      }
    });
  },

  methods: {
    handlePaste(event) {
      // 处理粘贴事件
      const items = event.clipboardData?.items;
      if (items) {
        for (let item of items) {
          if (item.type === "text/plain") {
            item.getAsString((text) => {
              this.textContent = text;
              this.validateFormat();
              this.$message.success("文本已粘贴");
            });
            break;
          }
        }
      }
    },

    handleInput() {
      // 实时验证格式
      if (this.textContent.trim()) {
        this.validateFormat();
      } else {
        this.validationError = null;
        this.isValidFormat = false;
      }
    },

    validateFormat() {
      // 简单的格式验证逻辑
      const content = this.textContent.trim();

      if (content.length < 100) {
        this.validationError = "文本内容过短，可能不是有效的 Profile 数据";
        this.isValidFormat = false;
        return;
      }

      // 检查是否包含典型的 Profile 关键词
      const profileKeywords = [
        "Query",
        "Fragment",
        "Operator",
        "Execution Profile",
        "Rows",
        "Time",
        "Memory",
      ];

      const hasKeywords = profileKeywords.some((keyword) =>
        content.includes(keyword),
      );

      if (!hasKeywords) {
        this.validationError = "未检测到有效的 Profile 数据特征";
        this.isValidFormat = false;
        return;
      }

      this.validationError = null;
      this.isValidFormat = true;
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

    async handleFileChange(file, fileList) {
      this.fileList = fileList.slice(-1);

      if (file.raw) {
        try {
          const text = await file.raw.text();
          this.textContent = text;
          this.validateFormat();
          this.$message.success("文件内容已加载");
        } catch (error) {
          this.$message.error("文件读取失败: " + error.message);
        }
      }
    },

    async handleAnalyze() {
      if (!this.canAnalyze) {
        this.$message.warning("请先输入有效的 Profile 数据");
        return;
      }

      try {
        if (this.textContent && this.isValidFormat) {
          this.$emit("text-analyze", this.textContent);
        } else if (this.selectedFile) {
          const text = await this.selectedFile.text();
          this.$emit("file-analyze", text);
        }
      } catch (error) {
        this.$message.error("数据读取失败: " + error.message);
      }
    },

    async copyToClipboard() {
      try {
        await navigator.clipboard.writeText(this.textContent);
        this.$message.success("内容已复制到剪贴板");
      } catch (error) {
        this.$message.error("复制失败: " + error.message);
      }
    },

    exportToFile() {
      if (!this.textContent) {
        this.$message.warning("没有内容可导出");
        return;
      }

      const blob = new Blob([this.textContent], { type: "text/plain" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `profile_${new Date().getTime()}.txt`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);

      this.$message.success("文件已导出");
    },

    clearContent() {
      this.textContent = "";
      this.fileList = [];
      this.validationError = null;
      this.isValidFormat = false;

      if (this.$refs.uploadRef) {
        this.$refs.uploadRef.clearFiles();
      }

      this.$nextTick(() => {
        if (this.$refs.textInput) {
          this.$refs.textInput.focus();
        }
      });
    },
  },
};
</script>

<style scoped>
.text-input-section {
  width: 100%;
}

.text-input-container {
  margin-bottom: 16px;
}

.input-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.input-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 600;
  color: #303133;
}

.input-stats {
  display: flex;
  align-items: center;
}

.text-input-wrapper {
  position: relative;
  border: 2px dashed #dcdfe6;
  border-radius: 8px;
  transition: all 0.3s ease;
}

.text-input-wrapper:hover {
  border-color: #409eff;
}

.profile-textarea {
  width: 100%;
  min-height: 300px;
  padding: 16px;
  border: none;
  outline: none;
  resize: vertical;
  font-family: "Monaco", "Menlo", "Ubuntu Mono", monospace;
  font-size: 14px;
  line-height: 1.5;
  background: transparent;
}

.profile-textarea:focus {
  outline: none;
}

.profile-textarea.has-content {
  border-color: #67c23a;
}

.profile-textarea.has-error {
  border-color: #f56c6c;
}

.paste-overlay {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(255, 255, 255, 0.9);
  pointer-events: none;
}

.paste-hint {
  text-align: center;
  color: #909399;
}

.paste-hint i {
  font-size: 48px;
  margin-bottom: 16px;
  display: block;
}

.paste-hint p {
  font-size: 16px;
  margin: 8px 0 4px 0;
}

.paste-hint small {
  font-size: 12px;
  color: #c0c4cc;
}

.validation-status {
  margin-top: 12px;
}

.divider-text {
  font-size: 12px;
  color: #909399;
}

.compact-upload {
  display: flex;
  justify-content: center;
}

.compact-upload-demo {
  width: 100%;
  max-width: 300px;
}

.compact-upload-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 16px;
  gap: 12px;
}

.compact-upload-content i {
  font-size: 24px;
  color: #c0c4cc;
}

.upload-text {
  text-align: center;
}

.upload-text p {
  margin: 0 0 8px 0;
  font-size: 12px;
  color: #606266;
}

.input-actions {
  margin-top: 20px;
  padding-top: 16px;
  border-top: 1px solid #ebeef5;
  display: flex;
  justify-content: center;
}

:deep(.el-divider--horizontal) {
  margin: 16px 0;
}

:deep(.el-upload-dragger) {
  border: 1px dashed #dcdfe6;
  border-radius: 6px;
  padding: 12px;
}

:deep(.el-upload-dragger:hover) {
  border-color: #409eff;
  background-color: #ecf5ff;
}
</style>
