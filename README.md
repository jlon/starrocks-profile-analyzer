# StarRocks Profile 分析器

一个强大的 StarRocks 查询执行计划分析工具，能够可视化执行计划、检测性能热点并提供优化建议。

## 功能特性

### 🎯 核心功能
- **Profile文件上传**：支持拖拽上传 StarRocks Profile 文件
- **执行计划可视化**：
  - 树形视图：层次清晰的执行计划展示
  - 图表视图：力导向图展示操作符关系
- **热点分析**：
  - 颜色编码的严重程度标识
  - 分段碎片化检测
  - I/O瓶颈分析
  - 数据倾斜识别
- **性能统计**：完整的性能指标展示

### 🔍 热点检测规则
- **Segment碎片化**：检测过多的元信息段读取
- **I/O效率**：分析远程vs本地存储访问模式
- **内存使用**：监控异常内存消耗
- **执行时间**：识别超时操作
- **数据倾斜**：JOIN操作的数据分布分析

## 技术架构

### 前端 (Vue.js + Element Plus + D3.js)
```
frontend/
├── src/
│   ├── components/      # Vue组件
│   │   ├── FileUploader.vue
│   │   ├── ExecutionPlanVisualization.vue
│   │   ├── AnalysisSummary.vue
│   │   └── HotSpotsPanel.vue
│   ├── views/          # 主视图
│   ├── router/         # 路由配置
│   ├── store/          # Vuex状态管理
│   └── styles/         # 样式文件
```

### 后端 (Rust + Warp)
```
backend/
├── src/
│   ├── analyzer/       # 热点检测引擎
│   ├── parser/         # Profile解析器
│   ├── api/           # REST API
│   └── models.rs      # 数据模型
```

## 快速开始

### 环境要求
- Node.js 16+
- Rust 1.70+
- npm 或 yarn

### 安装运行

1. **克隆项目**
```bash
git clone https://github.com/jlon/starrocks-profile-analyzer.git
cd starrocks-profile-analyzer
```

2. **启动后端服务**
```bash
cargo run --bin starrocks-profile-analyzer
```
后端API将在 `http://localhost:3030` 启动

3. **启动前端服务**
```bash
cd frontend
npm install
npm run serve
```
前端将在 `http://localhost:8080` 启动

### 使用方法

1. 打开浏览器访问前端地址
2. 点击或拖拽 StarRocks Profile 文件到上传区域
3. 点击"开始分析"按钮
4. 查看分析结果：
   - **分析概览**：性能评分和热点统计
   - **执行计划**：树形或图表形式的执行计划可视化
   - **热点分析**：详细信息和优化建议

## API说明

### POST /analyze
分析StarRocks Profile文件

**请求**
```json
{
  "profile_text": "完整的Profile文本内容"
}
```

**响应**
```json
{
  "success": true,
  "data": {
    "hotspots": [...],
    "conclusion": "...",
    "suggestions": [...],
    "performance_score": 85.5,
    "execution_tree": {...}
  }
}
```

### GET /health
健康检查

## 贡献

欢迎提交Issue和Pull Request！

## 许可证

MIT License
