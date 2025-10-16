#!/bin/bash

echo "🚀 StarRocks Profile 分析器 - 启动脚本"
echo "=========================================="

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 保存脚本所在目录
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# 杀死已有的进程
echo "📋 清理已有进程..."
pkill -f "npm run serve" 2>/dev/null
pkill -f "starrocks-profile-analyzer" 2>/dev/null
sleep 2

# 启动后端
echo -e "${YELLOW}📦 启动后端服务...${NC}"
cd "$SCRIPT_DIR"

# 确保已编译
echo "   📝 编译后端..."
cd backend && cargo build --release 2>&1 | grep -E "Finished|error" | head -1
cd "$SCRIPT_DIR"

# 使用预编译的二进制（绝对路径）
BACKEND_BIN="$SCRIPT_DIR/target/release/starrocks-profile-analyzer"
if [ ! -f "$BACKEND_BIN" ]; then
    echo -e "   ${RED}❌ 二进制文件不存在: $BACKEND_BIN${NC}"
    exit 1
fi

$BACKEND_BIN > /tmp/backend.log 2>&1 &
BACKEND_PID=$!
echo "   PID: $BACKEND_PID"

# 等待后端启动
echo "   等待后端初始化..."
sleep 5

# 检查后端是否成功启动
RETRY=0
while [ $RETRY -lt 10 ]; do
    if curl -s http://127.0.0.1:3030/health > /dev/null 2>&1; then
        echo -e "   ${GREEN}✅ 后端已启动${NC}"
        break
    fi
    RETRY=$((RETRY + 1))
    if [ $RETRY -lt 10 ]; then
        echo "   ⏳ 等待后端响应... ($RETRY/10)"
        sleep 2
    fi
done

if [ $RETRY -eq 10 ]; then
    echo -e "   ${RED}❌ 后端启动失败${NC}"
    echo "   查看日志: tail -f /tmp/backend.log"
    exit 1
fi

# 启动前端
echo -e "${YELLOW}🎨 启动前端服务...${NC}"
cd "$SCRIPT_DIR/frontend"
npm run serve > /tmp/frontend.log 2>&1 &
FRONTEND_PID=$!
echo "   PID: $FRONTEND_PID"

# 等待前端启动
echo "   等待前端初始化..."
sleep 10

# 检查前端是否成功启动
RETRY=0
while [ $RETRY -lt 10 ]; do
    if curl -s http://localhost:8080 > /dev/null 2>&1; then
        echo -e "   ${GREEN}✅ 前端已启动${NC}"
        break
    fi
    RETRY=$((RETRY + 1))
    if [ $RETRY -lt 10 ]; then
        echo "   ⏳ 等待前端响应... ($RETRY/10)"
        sleep 2
    fi
done

if [ $RETRY -eq 10 ]; then
    echo -e "   ${RED}❌ 前端启动失败${NC}"
    echo "   查看日志: tail -f /tmp/frontend.log"
    exit 1
fi

echo ""
echo "=========================================="
echo -e "${GREEN}✅ 所有服务已启动！${NC}"
echo ""
echo "📍 访问地址:"
echo "   Frontend: http://localhost:8080"
echo "   Backend:  http://localhost:3030"
echo ""
echo "📖 查看日志:"
echo "   后端: tail -f /tmp/backend.log"
echo "   前端: tail -f /tmp/frontend.log"
echo ""
echo "🛑 停止服务: pkill -f 'cargo run' && pkill -f 'npm run serve'"
echo ""
