# StarRocks Profile 智能分析器

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![Vue.js](https://img.shields.io/badge/vue.js-3.x-green.svg)](https://vuejs.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

一款专门用于分析 StarRocks OLAP 引擎查询 Profile 的智能工具，实现精准性能分析、智能热点检测和可视化展示。

## ✨ 核心特性

- 🎯 **精准解析**：基于 StarRocks 官方解析逻辑的通用百分比计算
- 🔍 **智能诊断**：自动识别执行计划中的性能瓶颈
- 📊 **可视化展示**：交互式 DAG 图展示执行计划
- 💡 **优化建议**：基于官方 tuning recipes 的自动化诊断
- 🚀 **高性能**：支持大文件解析，内存使用优化
- 🌐 **易用界面**：现代化 Web 界面，支持文件上传和文本粘贴

## 🏗️ 项目结构

```
starrocks-profile/
├── doc/                           # 项目文档
│   └── STARROCKS_PROFILE_ANALYZER_DESIGN.md
├── backend/                       # Rust 后端
│   ├── src/
│   │   ├── lib.rs                 # 主入口
│   │   ├── models.rs              # 数据模型
│   │   ├── api/                   # HTTP API
│   │   ├── parser/                # Profile 解析器
│   │   └── analyzer/              # 性能分析器
│   └── Cargo.toml
├── frontend/                      # Vue.js 前端
│   ├── src/
│   │   ├── components/            # Vue 组件
│   │   ├── views/                 # 页面视图
│   │   ├── store/                 # 状态管理
│   │   └── utils/                 # 工具函数
│   └── package.json
├── profiles/                      # 测试数据
│   ├── profile1.txt
│   ├── profile2.txt
│   └── ...
└── README.md
```

## 🚀 快速开始

### 环境要求

- Rust 1.70+
- Node.js 18+
- npm 或 yarn

### 安装和运行

1. **克隆项目**
```bash
git clone <repository-url>
cd starrocks-profile
```

2. **启动后端服务**
```bash
cd backend
cargo build --release
./target/release/starrocks-profile-analyzer
```

3. **启动前端服务**
```bash
cd frontend
npm install
npm run build
npx http-server dist -p 8080
```

4. **访问应用**
- 前端界面：http://localhost:8080
- 后端 API：http://localhost:3030

### 一键启动

```bash
./start_all.sh
```

## 📖 使用指南

### 1. 上传 Profile 文件

- **文件上传**：支持 `.txt`、`.log`、`.profile` 格式，最大 50MB
- **文本粘贴**：直接粘贴 Profile 文本内容
- **拖拽上传**：拖拽文件到上传区域

### 2. 查看分析结果

- **执行树可视化**：交互式 DAG 图展示执行计划
- **热点问题**：自动识别的性能瓶颈
- **优化建议**：基于官方最佳实践的建议
- **性能评分**：整体性能评估

### 3. API 使用

#### 健康检查
```bash
curl http://localhost:3030/health
```

#### 文本分析
```bash
curl -X POST http://localhost:3030/analyze \
  -H "Content-Type: application/json" \
  -d '{"profile_text": "Profile 文本内容"}'
```

#### 文件上传
```bash
curl -X POST http://localhost:3030/analyze-file \
  -F "file=@profile.txt"
```

## 🔧 开发指南

### 后端开发

```bash
cd backend

# 开发模式运行
cargo run

# 运行测试
cargo test

# 代码格式化
cargo fmt

# 代码检查
cargo clippy
```

### 前端开发

```bash
cd frontend

# 安装依赖
npm install

# 开发模式
npm run serve

# 构建生产版本
npm run build

# 代码检查
npm run lint
```

### 项目构建

```bash
# 构建后端
cd backend && cargo build --release

# 构建前端
cd frontend && npm run build
```

## 🧪 测试

### 单元测试
```bash
cd backend
cargo test
```

### 集成测试
```bash
# 测试 API 端点
curl -X POST http://localhost:3030/analyze \
  -H "Content-Type: application/json" \
  -d '{"profile_text": "测试 Profile 内容"}'
```

### 性能测试
```bash
# 使用测试数据
cd profiles
curl -X POST http://localhost:3030/analyze-file \
  -F "file=@profile1.txt"
```

## 📊 技术架构

### 核心技术栈

- **后端**：Rust + Warp + Tokio
- **前端**：Vue.js 3 + Element Plus + D3.js
- **解析引擎**：基于 StarRocks 官方逻辑的通用解析器
- **可视化**：D3.js 驱动的交互式图表

### 关键算法

1. **通用解析逻辑**：基于 StarRocks 源码的复杂聚合算法
2. **节点匹配**：智能匹配 Topology 和 Fragment 中的操作符
3. **时间计算**：精确的百分比计算，与官方工具一致
4. **热点检测**：多层次的性能瓶颈识别

## 📈 性能指标

- **解析速度**：支持大文件（50MB+）快速解析
- **内存使用**：优化的内存管理，支持流式处理
- **准确性**：与官方解析工具结果高度一致（误差 < 5%）
- **兼容性**：支持 StarRocks 3.x 版本的 Profile 格式

## 🤝 贡献指南

1. Fork 项目
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 打开 Pull Request

### 代码规范

- 使用 `cargo fmt` 格式化 Rust 代码
- 使用 `npm run lint` 检查前端代码
- 编写清晰的注释和文档
- 添加适当的测试用例

## 📄 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 🙏 致谢

- [StarRocks](https://github.com/StarRocks/starrocks) - 优秀的 OLAP 引擎
- [Vue.js](https://vuejs.org/) - 现代化的前端框架
- [Rust](https://www.rust-lang.org/) - 安全高效的编程语言

## 📞 联系我们

- 项目主页：[GitHub Repository]
- 问题反馈：[Issues]
- 技术讨论：[Discussions]

---

**StarRocks Profile 智能分析器** - 让查询性能分析更简单、更智能！