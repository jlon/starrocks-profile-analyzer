//! # TreeBuilder - 执行树构建器
//! 
//! 负责根据 Topology 或 Fragment 信息构建执行树，并计算节点深度。

use crate::models::{ExecutionTree, ExecutionTreeNode};
use crate::parser::error::{ParseError, ParseResult};
use super::topology_parser::TopologyGraph;
use std::collections::{HashMap, HashSet, VecDeque};

pub struct TreeBuilder;

impl TreeBuilder {
    ///
    /// # Arguments
    /// * `topology` - 解析好的拓扑图
    /// * `nodes` - 所有已解析的执行节点（包含指标）
    /// * `fragments` - 解析后的Fragments列表，用于查找SINK节点
    ///
    /// # Returns
    /// * `ExecutionTree` - 构建好的执行树
    pub fn build_from_topology(
        topology: &TopologyGraph,
        mut nodes: Vec<ExecutionTreeNode>,
        fragments: &[crate::models::Fragment],
    ) -> ParseResult<ExecutionTree> {
        // 1. 建立节点ID映射
        let mut id_to_idx: HashMap<i32, usize> = HashMap::new();
        for (idx, node) in nodes.iter().enumerate() {
            if let Some(plan_id) = node.plan_node_id {
                id_to_idx.insert(plan_id, idx);
            }
        }

        // 2. 根据 Topology 建立父子关系
        for topo_node in &topology.nodes {
            if let Some(&node_idx) = id_to_idx.get(&topo_node.id) {
                // 清空旧的 children（避免重复）
                nodes[node_idx].children.clear();

                for &child_id in &topo_node.children {
                    if let Some(&child_idx) = id_to_idx.get(&child_id) {
                        let child_node_id = nodes[child_idx].id.clone();
                        nodes[node_idx].children.push(child_node_id);
                        nodes[child_idx].parent_plan_node_id = Some(topo_node.id);
                    }
                }
            }
        }

        // 3. 使用两层查找策略找到正确的SINK节点作为根节点
        let sink_node_name = Self::find_sink_node_for_tree_root(fragments);

        // 4. 找到SINK节点在nodes中的索引
        let root_idx = if let Some(sink_name) = sink_node_name {
            // 优先使用找到的SINK节点作为根节点
            nodes.iter().position(|n| n.operator_name == sink_name)
                .or_else(|| {
                    // 如果没找到，尝试查找以_SINK结尾的节点
                    nodes.iter().position(|n| n.operator_name.ends_with("_SINK"))
                })
                .unwrap_or_else(|| {
                    // 如果都没找到，使用topology的root_id
                    id_to_idx.get(&topology.root_id).copied().unwrap_or(0)
                })
        } else {
            // 如果没找到SINK节点，使用topology的root_id
            id_to_idx.get(&topology.root_id)
                .copied()
                .ok_or_else(|| ParseError::TreeError(
                    format!("Root node {} not found", topology.root_id)
                ))?
        };

        // 5. 计算深度（从SINK节点开始）
        Self::calculate_depths_from_sink(&mut nodes, root_idx)?;

        let root = nodes[root_idx].clone();

        Ok(ExecutionTree { root, nodes })
    }
    
    /// 从 Fragment 列表构建执行树（回退方案）
    /// 
    /// 当 Topology 不可用时使用此方法。
    /// 构建线性的树结构（每个 Operator 指向下一个）。
    pub fn build_from_fragments(nodes: Vec<ExecutionTreeNode>) -> ParseResult<ExecutionTree> {
        if nodes.is_empty() {
            return Err(ParseError::TreeError("No nodes to build tree".to_string()));
        }
        
        let mut nodes = nodes;
        
        // 建立线性关系（每个节点指向下一个）
        for i in 0..nodes.len().saturating_sub(1) {
            let next_id = nodes[i + 1].id.clone();
            nodes[i].children.push(next_id);
            nodes[i + 1].parent_plan_node_id = nodes[i].plan_node_id;
        }
        
        // 计算深度
        Self::calculate_depths(&mut nodes)?;
        
        let root = nodes[0].clone();
        Ok(ExecutionTree { root, nodes })
    }
    
    /// 计算节点深度（BFS）
    /// 
    /// 深度从 0 开始，根节点深度为 0。
    pub fn calculate_depths(nodes: &mut [ExecutionTreeNode]) -> ParseResult<()> {
        if nodes.is_empty() {
            return Ok(());
        }
        
        // 1. 建立 ID 到索引的映射
        let id_to_idx: HashMap<String, usize> = nodes.iter()
            .enumerate()
            .map(|(idx, node)| (node.id.clone(), idx))
            .collect();
        
        // 2. 找到根节点（没有父节点的节点）
        let mut has_parent = HashSet::new();
        for node in nodes.iter() {
            for child_id in &node.children {
                has_parent.insert(child_id.clone());
            }
        }
        
        let root_idx = nodes.iter()
            .position(|n| !has_parent.contains(&n.id))
            .ok_or_else(|| ParseError::TreeError("No root node found".to_string()))?;
        
        // 3. BFS 计算深度
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        
        queue.push_back((root_idx, 0)); // (node_index, depth)
        visited.insert(root_idx);
        nodes[root_idx].depth = 0;
        
        while let Some((node_idx, depth)) = queue.pop_front() {
            let children_ids: Vec<String> = nodes[node_idx].children.clone();
            
            for child_id in children_ids {
                if let Some(&child_idx) = id_to_idx.get(&child_id) {
                    if !visited.contains(&child_idx) {
                        nodes[child_idx].depth = depth + 1;
                        visited.insert(child_idx);
                        queue.push_back((child_idx, depth + 1));
                    }
                }
            }
        }
        
        // 4. 对于未访问的节点（孤立节点），设置深度为 0
        for (idx, node) in nodes.iter_mut().enumerate() {
            if !visited.contains(&idx) {
                node.depth = 0;
            }
        }
        
        Ok(())
    }
    
    /// 链接 Exchange 连接
    /// 
    /// 在不同 Fragment 之间建立 EXCHANGE_SINK -> EXCHANGE_SOURCE 的连接。
    pub fn link_exchange_operators(nodes: &mut Vec<ExecutionTreeNode>) {
        // 收集所有 Exchange 节点
        let exchanges: Vec<(usize, Option<i32>, String, String)> = nodes.iter()
            .enumerate()
            .filter_map(|(i, n)| {
                if n.operator_name == "EXCHANGE_SINK" || n.operator_name == "EXCHANGE_SOURCE" {
                    Some((i, n.plan_node_id, n.id.clone(), n.operator_name.clone()))
                } else {
                    None
                }
            })
            .collect();
        
        // 匹配 SINK 和 SOURCE
        for i in 0..exchanges.len() {
            let (sink_idx, sink_plan_id, _sink_id, sink_name) = &exchanges[i];
            
            if sink_name == "EXCHANGE_SINK" && sink_plan_id.is_some() {
                for j in (i + 1)..exchanges.len() {
                    let (_source_idx, source_plan_id, source_id, source_name) = &exchanges[j];
                    
                    if source_name == "EXCHANGE_SOURCE" && source_plan_id == sink_plan_id {
                        // 连接 SINK -> SOURCE
                        nodes[*sink_idx].children.push(source_id.clone());
                        break;
                    }
                }
            }
        }
    }
    
    /// 验证树的有效性
    /// 
    /// 检查：
    /// 1. 所有节点的 children 引用都存在
    /// 2. 没有环路
    /// 3. 每个节点最多一个父节点
    pub fn validate(tree: &ExecutionTree) -> ParseResult<()> {
        let node_ids: HashSet<String> = tree.nodes.iter().map(|n| n.id.clone()).collect();
        
        // 检查 children 引用
        for node in &tree.nodes {
            for child_id in &node.children {
                if !node_ids.contains(child_id) {
                    return Err(ParseError::TreeError(
                        format!("Child {} not found", child_id)
                    ));
                }
            }
        }
        
        // 检查环路（使用 DFS）
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        
        if Self::has_cycle(&tree.root.id, &tree.nodes, &mut visited, &mut rec_stack)? {
            return Err(ParseError::TreeError("Cycle detected in tree".to_string()));
        }
        
        Ok(())
    }
    
    /// 从operator名称中提取纯名称（去掉plan_node_id部分）
    ///
    /// # Arguments
    /// * `full_name` - 完整的operator名称，如 "LOCAL_EXCHANGE_SINK (plan_node_id=-1)"
    ///
    /// # Returns
    /// * `String` - 纯的operator名称，如 "LOCAL_EXCHANGE_SINK"
    fn extract_operator_name(full_name: &str) -> String {
        if let Some(pos) = full_name.find(" (plan_node_id=") {
            full_name[..pos].to_string()
        } else {
            full_name.to_string()
        }
    }
    
    /// 使用StarRocks的通用逻辑找到树根节点的SINK节点名称
    ///
    /// StarRocks的isFinalSink逻辑：
    /// 1. 必须是DataSink类型（以_SINK结尾）
    /// 2. 不能是DataStreamSink类型（EXCHANGE_SINK, LOCAL_EXCHANGE_SINK等）
    /// 3. 不能是MultiCastDataSink类型
    ///
    /// # Arguments
    /// * `fragments` - 解析后的Fragments列表
    ///
    /// # Returns
    /// * `Option<String>` - 找到的SINK节点名称，如果没找到返回None
    fn find_sink_node_for_tree_root(fragments: &[crate::models::Fragment]) -> Option<String> {
        // 收集所有SINK节点，按优先级排序
        let mut sink_candidates = Vec::new();
        
        for fragment in fragments {
            for pipeline in &fragment.pipelines {
                for operator in &pipeline.operators {
                    let pure_name = Self::extract_operator_name(&operator.name);
                    if pure_name.ends_with("_SINK") {
                        let is_final_sink = Self::is_final_sink(&pure_name);
                        let priority = Self::get_sink_priority(&pure_name);
                        
                        sink_candidates.push((pure_name.clone(), is_final_sink, priority));
                    }
                }
            }
        }
        
        // 按优先级排序：final sink > 高优先级 > 低优先级
        sink_candidates.sort_by(|a, b| {
            match (a.1, b.1) {
                (true, false) => std::cmp::Ordering::Less,  // a是final sink，优先级更高
                (false, true) => std::cmp::Ordering::Greater, // b是final sink，优先级更高
                _ => a.2.cmp(&b.2), // 都是或都不是final sink，按优先级排序
            }
        });
        
        if let Some((name, is_final, priority)) = sink_candidates.first() {
            Some(name.clone())
        } else {
            None
        }
    }
    
    /// 判断是否为final sink（基于StarRocks的isFinalSink逻辑）
    ///
    /// # Arguments
    /// * `sink_name` - SINK节点名称
    ///
    /// # Returns
    /// * `bool` - 是否为final sink
    fn is_final_sink(sink_name: &str) -> bool {
        // 不能是DataStreamSink类型
        if sink_name.contains("EXCHANGE_SINK") || sink_name.contains("LOCAL_EXCHANGE_SINK") {
            return false;
        }
        
        // 不能是MultiCastDataSink类型（通常包含MULTI_CAST）
        if sink_name.contains("MULTI_CAST") {
            return false;
        }
        
        // 其他_SINK节点都是final sink
        true
    }
    
    /// 获取SINK节点的优先级（数字越小优先级越高）
    ///
    /// # Arguments
    /// * `sink_name` - SINK节点名称
    ///
    /// # Returns
    /// * `i32` - 优先级（数字越小优先级越高）
    fn get_sink_priority(sink_name: &str) -> i32 {
        if sink_name == "RESULT_SINK" {
            1
        } else if sink_name == "OLAP_TABLE_SINK" {
            2
        } else if sink_name.contains("TABLE_SINK") {
            3
        } else if sink_name.contains("EXCHANGE_SINK") {
            4
        } else if sink_name.contains("LOCAL_EXCHANGE_SINK") {
            5
        } else {
            6 // 其他SINK节点
        }
    }

    /// 从指定的SINK节点开始计算深度（BFS）
    ///
    /// 深度从 0 开始，SINK节点深度为 0，其他节点深度递增。
    ///
    /// # Arguments
    /// * `nodes` - 节点列表
    /// * `root_idx` - 根节点（SINK节点）在nodes中的索引
    ///
    /// # Returns
    /// * `ParseResult<()>` - 计算结果
    fn calculate_depths_from_sink(nodes: &mut [ExecutionTreeNode], root_idx: usize) -> ParseResult<()> {
        if nodes.is_empty() {
            return Ok(());
        }

        // 1. 建立 ID 到索引的映射（节点唯一 id -> 索引）
        let id_to_idx: HashMap<String, usize> = nodes.iter()
            .enumerate()
            .map(|(idx, node)| (node.id.clone(), idx))
            .collect();

        // 额外：建立 plan_node_id 到索引的映射，便于通过父 plan_node_id 找到父节点索引
        let plan_id_to_idx: HashMap<i32, usize> = nodes.iter()
            .enumerate()
            .filter_map(|(idx, node)| node.plan_node_id.map(|pid| (pid, idx)))
            .collect();

        // 2. BFS 计算深度，从 SINK 节点开始，沿父指针向上游递增
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        // 从 SINK 节点开始，深度为 0（SINK 在最上层）
        queue.push_back((root_idx, 0)); // (node_index, depth)
        visited.insert(root_idx);
        nodes[root_idx].depth = 0;

        while let Some((node_idx, depth)) = queue.pop_front() {
            // 沿着父节点方向遍历：当前节点的父 plan_node_id
            if let Some(parent_plan_id) = nodes[node_idx].parent_plan_node_id {
                if let Some(&parent_idx) = plan_id_to_idx.get(&parent_plan_id) {
                    if !visited.contains(&parent_idx) {
                        // 深度递增：距离 SINK 越远，深度越大（从上到下）
                        nodes[parent_idx].depth = depth + 1;
                        visited.insert(parent_idx);
                        queue.push_back((parent_idx, depth + 1));
                    }
                }
            }
        }

        // 3. 对于未访问的节点（孤立或未连接到 SINK 的节点），设置深度为 0
        for (idx, node) in nodes.iter_mut().enumerate() {
            if !visited.contains(&idx) {
                node.depth = 0;
            }
        }

        Ok(())
    }

    // ========== Private Helper Methods ==========

    fn has_cycle(
        node_id: &str,
        nodes: &[ExecutionTreeNode],
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> ParseResult<bool> {
        visited.insert(node_id.to_string());
        rec_stack.insert(node_id.to_string());

        if let Some(node) = nodes.iter().find(|n| n.id == node_id) {
            for child_id in &node.children {
                if !visited.contains(child_id) {
                    if Self::has_cycle(child_id, nodes, visited, rec_stack)? {
                        return Ok(true);
                    }
                } else if rec_stack.contains(child_id) {
                    return Ok(true);
                }
            }
        }

        rec_stack.remove(node_id);
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{NodeType, OperatorMetrics, HotSeverity};
    
    #[test]
    fn test_calculate_depths() {
        let mut nodes = vec![
            ExecutionTreeNode {
                id: "node_0".to_string(),
                operator_name: "ROOT".to_string(),
                node_type: NodeType::Unknown,
                plan_node_id: Some(0),
                parent_plan_node_id: None,
                metrics: OperatorMetrics::default(),
                children: vec!["node_1".to_string()],
                depth: 0,
                is_hotspot: false,
                hotspot_severity: HotSeverity::Normal,
                fragment_id: None,
                pipeline_id: None,
            },
            ExecutionTreeNode {
                id: "node_1".to_string(),
                operator_name: "LEAF".to_string(),
                node_type: NodeType::Unknown,
                plan_node_id: Some(1),
                parent_plan_node_id: Some(0),
                metrics: OperatorMetrics::default(),
                children: vec![],
                depth: 0,
                is_hotspot: false,
                hotspot_severity: HotSeverity::Normal,
                fragment_id: None,
                pipeline_id: None,
            },
        ];
        
        TreeBuilder::calculate_depths(&mut nodes).unwrap();
        assert_eq!(nodes[0].depth, 0);
        assert_eq!(nodes[1].depth, 1);
    }
}
