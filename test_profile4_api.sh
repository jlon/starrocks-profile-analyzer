#!/bin/bash

echo "🧪 测试 profile4.txt 的API解析"
echo "================================"

# 读取profile4.txt
PROFILE_TEXT=$(cat profiles/profile4.txt)

# 创建JSON payload
JSON_PAYLOAD=$(jq -n --arg text "$PROFILE_TEXT" '{profile_text: $text}')

# 发送请求
echo "📤 发送请求到 http://localhost:3030/analyze ..."
RESPONSE=$(curl -s -X POST http://localhost:3030/analyze \
  -H "Content-Type: application/json" \
  -d "$JSON_PAYLOAD")

# 检查是否成功
SUCCESS=$(echo "$RESPONSE" | jq -r '.success')

if [ "$SUCCESS" = "true" ]; then
    echo "✅ 解析成功！"
    echo ""
    echo "📊 执行树节点："
    echo "$RESPONSE" | jq -r '.data.execution_tree.nodes[] | "  - \(.operator_name) (plan_node_id=\(.plan_node_id // "null")): \(.time_percentage // 0)%"'
    
    echo ""
    echo "🎯 关键节点验证："
    
    # RESULT_SINK
    RESULT_SINK_PCT=$(echo "$RESPONSE" | jq -r '.data.execution_tree.nodes[] | select(.operator_name=="RESULT_SINK") | .time_percentage')
    echo "  RESULT_SINK: ${RESULT_SINK_PCT}% (期望: 97.43%)"
    
    # MERGE_EXCHANGE
    MERGE_EXCHANGE_PCT=$(echo "$RESPONSE" | jq -r '.data.execution_tree.nodes[] | select(.operator_name=="MERGE_EXCHANGE") | .time_percentage')
    echo "  MERGE_EXCHANGE: ${MERGE_EXCHANGE_PCT}% (期望: 2.64%)"
    
    echo ""
    echo "📈 Summary信息："
    echo "$RESPONSE" | jq -r '.data.summary | "  Query ID: \(.query_id)\n  Total Time: \(.total_time)\n  Query State: \(.query_state)"'
    
else
    echo "❌ 解析失败！"
    echo "$RESPONSE" | jq -r '.error'
fi

