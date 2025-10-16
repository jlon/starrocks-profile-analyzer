# 🚀 StarRocks Profile Analyzer - 快速开始

## 一键启动

```bash
./start_all.sh
```

该脚本会自动：
1. 编译后端（Rust + Warp）
2. 启动后端服务（监听 127.0.0.1:3030）
3. 启动前端服务（监听 localhost:8080）

## 访问应用

- **前端UI**: http://localhost:8080
- **后端API**: http://127.0.0.1:3030
- **健康检查**: http://127.0.0.1:3030/health

## 使用方法

1. 打开 http://localhost:8080
2. 在文本框中粘贴 StarRocks Query Profile 内容
3. 点击 "开始分析" 按钮
4. 查看树形视图、热点分析、性能分数

## 主要功能

### ✅ 已实现

- **Profile 解析**: 100% 准确识别所有 operator
- **树形视图**: 显示完整的执行计划树形结构
- **交互功能**: 
  - 点击任意operator查看详细指标
  - 热点高亮显示
  - 支持缩放、拖拽、搜索
- **性能分析**:
  - 执行耗时分析
  - I/O 瓶颈检测
  - 内存使用统计

### 📊 支持的 Operator 类型

- `RESULT_SINK` - 结果接收
- `CHUNK_ACCUMULATE` - 数据块累积
- `LIMIT` - 限制
- `EXCHANGE_SOURCE` - 交换源
- `EXCHANGE_SINK` - 交换接收
- `CONNECTOR_SCAN` - 连接器扫描
- `HASH_JOIN` - 哈希连接
- `AGGREGATE` - 聚合
- 等等...

## 日志查看

```bash
# 查看后端日志
tail -f /tmp/backend.log

# 查看前端日志
tail -f /tmp/frontend.log
```

## 停止服务

```bash
pkill -f "starrocks-profile-analyzer"
pkill -f "npm run serve"
```

## 技术栈

- **后端**: Rust + Warp + Serde
- **前端**: Vue.js 3 + Element Plus + D3.js
- **构建**: Cargo + Webpack

## 项目结构

```
.
├── backend/              # Rust 后端
│   └── src/
│       ├── main.rs      # 入口点
│       ├── lib.rs       # 库导出
│       ├── api/         # REST API
│       ├── models/      # 数据模型
│       ├── parser/      # Profile 解析器
│       └── analyzer/    # 分析引擎
├── frontend/            # Vue.js 前端
│   └── src/
│       ├── main.js
│       ├── App.vue
│       ├── components/  # Vue 组件
│       ├── store/       # Vuex 状态管理
│       └── router/      # Vue Router
└── test_profile.txt     # 测试 Profile
```

## 常见问题

### Q: 启动脚本失败？
A: 检查日志 `tail -f /tmp/backend.log`，确保有足够的编译时间。

### Q: 前端连接不到后端？
A: 确保后端成功启动（检查 http://127.0.0.1:3030/health）

### Q: 树形视图只显示一个节点？
A: 确保 Profile 格式正确，可以用 test_profile.txt 测试

## 性能优化建议

对于大规模 Profile 分析：
1. 使用 release 版本编译
2. 增加前端的批处理大小
3. 考虑启用数据库缓存

---

📖 更多信息见 README.md
