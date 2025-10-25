#!/usr/bin/env python3
import json
import requests

# 读取profile4.txt
with open('profiles/profile4.txt', 'r') as f:
    profile_text = f.read()

# 发送请求
response = requests.post(
    'http://localhost:3030/analyze',
    json={'profile_text': profile_text}
)

data = response.json()

if data['success']:
    print("✅ 解析成功！\n")
    
    # 打印Top Nodes
    top_nodes = data['data']['summary'].get('top_time_consuming_nodes', [])
    if top_nodes:
        print("📊 Top Most Time-consuming Nodes:")
        for node in top_nodes:
            rank = node['rank']
            name = node['operator_name']
            time = node['total_time']
            pct = node['time_percentage']
            is_most = node['is_most_consuming']
            is_second = node['is_second_most_consuming']
            
            color = "🔴" if is_most else ("🟠" if is_second else "⚪")
            print(f"  {color} {rank}. {name}: {time} ({pct:.2f}%)")
    else:
        print("❌ 没有Top Nodes数据")
else:
    print(f"❌ 解析失败: {data.get('error')}")

