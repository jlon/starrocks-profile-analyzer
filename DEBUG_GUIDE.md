# 调试指南 - Failed to fetch 问题排查

## 问题症状
前端点击"开始分析"后出现：`TypeError: Failed to fetch`

## 快速诊断

### 1. 验证后端服务是否运行
```bash
curl http://localhost:3030/health
# 预期返回: {"status":"ok"}
```

### 2. 验证后端API可以接收请求
```bash
curl -X POST http://localhost:3030/analyze \
  -H "Content-Type: application/json" \
  -d '{"profile_text":"Query:\n  Summary:\n     - Query ID: test"}'
# 预期返回: {"success":false,"error":"...","data":null}
```

### 3. 检查前端浏览器控制台日志
打开 http://localhost:8080，按 F12 打开开发者工具，查看 Console 标签

**应该看到的日志**:
```
📤 开始发送请求到: http://localhost:3030/analyze
📝 Profile文本长度: 437 字符
📨 收到响应: 200 OK
✅ 解析成功，收到数据: {...}
✅ 分析完成！
```

**如果看到错误**:
```
❌ API请求失败: {
  name: "TypeError",
  message: "Failed to fetch",
  stack: "..."
}
```

## 常见问题及解决方案

### 问题 1: 后端未运行
**症状**: curl 连接被拒绝
**解决**:
```bash
cd backend
cargo run --bin starrocks-profile-analyzer
```

### 问题 2: 前端未运行
**症状**: 无法访问 http://localhost:8080
**解决**:
```bash
cd frontend
npm run serve
```

### 问题 3: CORS 被阻止
**症状**: 浏览器控制台显示 CORS 错误
**检查**: 确保后端的 CORS 配置包含 `allow_any_origin()`

### 问题 4: 端口被占用
**症状**: 启动服务时提示"port already in use"
**解决**: 杀死占用端口的进程
```bash
# 查找占用 3030 的进程
lsof -i :3030
# 杀死进程 (PID 替换为实际的进程ID)
kill -9 PID
```

### 问题 5: 防火墙阻止
**症状**: 无法连接到 localhost:3030
**检查**: 
- 确保你在本机访问（不要从其他机器访问）
- 或检查本地防火墙设置

## 完整的工作流程检查表

- [ ] 后端编译成功 (`cargo build`)
- [ ] 后端进程运行中 (`ps aux | grep starrocks`)
- [ ] 后端能响应 health 检查 (`curl http://localhost:3030/health`)
- [ ] 后端能处理 POST 请求 (`curl -X POST ...`)
- [ ] 前端编译成功 (`npm run build`)
- [ ] 前端进程运行中 (`ps aux | grep npm`)
- [ ] 前端可以访问 (`curl http://localhost:8080`)
- [ ] 浏览器打开 http://localhost:8080
- [ ] 粘贴 Profile 文本到文本框
- [ ] 打开浏览器开发者工具 (F12)
- [ ] 点击"开始分析"
- [ ] 查看 Console 中的详细日志

## 如果仍然不工作

请提供以下信息：
1. 后端进程的完整启动日志
2. 前端的完整启动日志
3. 浏览器开发者工具 Console 的完整错误信息
4. 网络标签 (Network) 中的请求详情
5. `curl -v http://localhost:3030/analyze` 的完整输出

## 快速重启所有服务

```bash
# 杀死所有相关进程
pkill -f "npm run serve" 2>/dev/null
pkill -f "starrocks-profile-analyzer" 2>/dev/null
sleep 2

# 启动后端
cd backend && cargo run --bin starrocks-profile-analyzer &
sleep 3

# 启动前端
cd ../frontend && npm run serve &
sleep 5

echo "✅ 所有服务已启动，访问 http://localhost:8080"
```
