# StarRocks Profile Analyzer

<div align="center">

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![Vue.js](https://img.shields.io/badge/vue.js-3.x-green.svg)](https://vuejs.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**A professional tool for analyzing StarRocks query profiles with intelligent performance insights**

[English](#english) | [中文](#中文)

</div>

---

## English

### Overview

StarRocks Profile Analyzer is a powerful tool designed to parse, analyze, and visualize StarRocks OLAP query profiles. It provides accurate performance metrics, intelligent bottleneck detection, and actionable optimization suggestions based on official StarRocks parsing logic.

### ✨ Key Features

- 🎯 **Accurate Parsing**: Universal percentage calculation based on official StarRocks logic
- 🔍 **Smart Diagnostics**: Automatic performance bottleneck identification
- 📊 **Interactive Visualization**: DAG-based execution plan visualization
- 💡 **Optimization Suggestions**: Automated recommendations based on official tuning recipes
- 🚀 **High Performance**: Optimized for large files with efficient memory usage
- 🌐 **Modern UI**: Web interface with file upload and text paste support

### 🚀 Quick Start

#### Prerequisites

- Rust 1.70+
- Node.js 18+
- npm or yarn

#### Installation

```bash
# Clone the repository
git clone <repository-url>
cd starrocks-profile

# One-command startup
./start_all.sh
```

#### Manual Setup

**Backend:**
```bash
cd backend
cargo build --release
./target/release/starrocks-profile-analyzer
```

**Frontend:**
```bash
cd frontend
npm install
npm run build
npx http-server dist -p 8080
```

**Access:**
- Frontend: http://localhost:8080
- Backend API: http://localhost:3030

### 📖 Usage

#### Upload Profile

- **File Upload**: Supports `.txt`, `.log`, `.profile` formats (max 50MB)
- **Text Paste**: Directly paste profile content
- **Drag & Drop**: Drag files to upload area

#### View Analysis

- **Execution Tree**: Interactive DAG visualization
- **Hotspots**: Automatically identified performance bottlenecks
- **Suggestions**: Optimization recommendations
- **Performance Score**: Overall performance assessment

#### API Examples

**Health Check:**
```bash
curl http://localhost:3030/health
```

**Analyze Text:**
```bash
curl -X POST http://localhost:3030/analyze \
  -H "Content-Type: application/json" \
  -d '{"profile_text": "Your profile content"}'
```

**Upload File:**
```bash
curl -X POST http://localhost:3030/analyze-file \
  -F "file=@profile.txt"
```

### 🏗️ Architecture

```
backend/src/
├── api/              # HTTP API layer
├── parser/           # Profile parser
│   ├── core/         # Core parsing components
│   └── specialized/  # Operator-specific parsers
├── analyzer/         # Performance analyzer
├── models.rs         # Data models
└── constants.rs      # Configuration constants

frontend/src/
├── components/       # Vue components
├── views/            # Page views
├── store/            # State management
└── utils/            # Utility functions
```

### 🧪 Testing

```bash
# Backend tests
cd backend && cargo test

# Validate all profiles
cargo run --release --bin validate_all_profiles

# Frontend tests
cd frontend && npm run test
```

### 📊 Performance

- **Parsing Speed**: Fast processing for large files (50MB+)
- **Memory Usage**: Optimized memory management
- **Accuracy**: High consistency with official parser (<0.3% error)
- **Compatibility**: Supports StarRocks 3.x profile format

### 🤝 Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

### 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

### 🙏 Acknowledgments

- [StarRocks](https://github.com/StarRocks/starrocks) - Excellent OLAP engine
- [Vue.js](https://vuejs.org/) - Progressive JavaScript framework
- [Rust](https://www.rust-lang.org/) - Safe and efficient programming language

---

## 中文

### 概述

StarRocks Profile 分析器是一款专业的查询性能分析工具，用于解析、分析和可视化 StarRocks OLAP 查询 Profile。基于官方 StarRocks 解析逻辑，提供精准的性能指标、智能瓶颈检测和可执行的优化建议。

### ✨ 核心特性

- 🎯 **精准解析**：基于 StarRocks 官方解析逻辑的通用百分比计算
- 🔍 **智能诊断**：自动识别执行计划中的性能瓶颈
- 📊 **可视化展示**：基于 DAG 的交互式执行计划可视化
- 💡 **优化建议**：基于官方调优方案的自动化建议
- 🚀 **高性能**：支持大文件解析，内存使用优化
- 🌐 **现代界面**：Web 界面，支持文件上传和文本粘贴

### 🚀 快速开始

#### 环境要求

- Rust 1.70+
- Node.js 18+
- npm 或 yarn

#### 安装

```bash
# 克隆项目
git clone <repository-url>
cd starrocks-profile

# 一键启动
./start_all.sh
```

#### 手动启动

**后端：**
```bash
cd backend
cargo build --release
./target/release/starrocks-profile-analyzer
```

**前端：**
```bash
cd frontend
npm install
npm run build
npx http-server dist -p 8080
```

**访问：**
- 前端界面：http://localhost:8080
- 后端 API：http://localhost:3030

### 📖 使用指南

#### 上传 Profile

- **文件上传**：支持 `.txt`、`.log`、`.profile` 格式（最大 50MB）
- **文本粘贴**：直接粘贴 Profile 文本内容
- **拖拽上传**：拖拽文件到上传区域

#### 查看分析结果

- **执行树**：交互式 DAG 图展示
- **热点问题**：自动识别的性能瓶颈
- **优化建议**：基于官方最佳实践的建议
- **性能评分**：整体性能评估

#### API 示例

**健康检查：**
```bash
curl http://localhost:3030/health
```

**文本分析：**
```bash
curl -X POST http://localhost:3030/analyze \
  -H "Content-Type: application/json" \
  -d '{"profile_text": "Profile 文本内容"}'
```

**文件上传：**
```bash
curl -X POST http://localhost:3030/analyze-file \
  -F "file=@profile.txt"
```

### 🏗️ 架构

```
backend/src/
├── api/              # HTTP API 层
├── parser/           # Profile 解析器
│   ├── core/         # 核心解析组件
│   └── specialized/  # 操作符特化解析器
├── analyzer/         # 性能分析器
├── models.rs         # 数据模型
└── constants.rs      # 配置常量

frontend/src/
├── components/       # Vue 组件
├── views/            # 页面视图
├── store/            # 状态管理
└── utils/            # 工具函数
```

### 🧪 测试

```bash
# 后端测试
cd backend && cargo test

# 验证所有 profiles
cargo run --release --bin validate_all_profiles

# 前端测试
cd frontend && npm run test
```

### 📊 性能指标

- **解析速度**：支持大文件（50MB+）快速解析
- **内存使用**：优化的内存管理
- **准确性**：与官方解析工具高度一致（误差 < 0.3%）
- **兼容性**：支持 StarRocks 3.x 版本的 Profile 格式

### 🤝 贡献指南

1. Fork 项目
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 打开 Pull Request

### 📄 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

### 🙏 致谢

- [StarRocks](https://github.com/StarRocks/starrocks) - 优秀的 OLAP 引擎
- [Vue.js](https://vuejs.org/) - 渐进式 JavaScript 框架
- [Rust](https://www.rust-lang.org/) - 安全高效的编程语言

---

<div align="center">

**Made with ❤️ for StarRocks Community**

</div>
