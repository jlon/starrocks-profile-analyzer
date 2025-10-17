use starrocks_profile_analyzer::parser::composer::ProfileComposer;
use std::fs;

#[test]
fn test_metrics_parsing_from_profile() {
    let profile_path = "../profiles/profile1.txt";
    let content = fs::read_to_string(profile_path)
        .expect("Failed to read profile file");
    
    let mut composer = ProfileComposer::new();
    let profile = composer.parse(&content)
        .expect("Failed to parse profile");
    
    // 验证 execution_tree 中的节点有 metrics
    assert!(profile.execution_tree.is_some(), "execution_tree should exist");
    
    let tree = profile.execution_tree.unwrap();
    assert!(!tree.nodes.is_empty(), "Tree should have nodes");
    
    // 找一个有 metrics 的节点
    let nodes_with_metrics: Vec<_> = tree.nodes.iter()
        .filter(|n| n.metrics.operator_total_time.is_some())
        .collect();
    
    println!("Total nodes: {}", tree.nodes.len());
    println!("Nodes with operator_total_time: {}", nodes_with_metrics.len());
    
    // 至少应该有一些节点有 metrics
    assert!(nodes_with_metrics.len() > 0, 
        "At least some nodes should have operator_total_time metrics");
    
    // 打印第一个有 metrics 的节点
    if let Some(node) = nodes_with_metrics.first() {
        println!("\nFirst node with metrics:");
        println!("  Name: {}", node.operator_name);
        println!("  OperatorTotalTime: {:?} ms", node.metrics.operator_total_time);
        println!("  PushChunkNum: {:?}", node.metrics.push_chunk_num);
        println!("  PullChunkNum: {:?}", node.metrics.pull_chunk_num);
        println!("  MemoryUsage: {:?}", node.metrics.memory_usage);
    }
}
