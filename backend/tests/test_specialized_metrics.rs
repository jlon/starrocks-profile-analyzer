use starrocks_profile_analyzer::parser::composer::ProfileComposer;
use std::fs;
use std::path::Path;

#[test]
fn test_specialized_metrics_parsing() {
    let profile_path = Path::new("../profiles/profile5.txt").to_string_lossy().to_string();
    let content = fs::read_to_string(profile_path)
        .expect("Failed to read profile file");
    
    let mut composer = ProfileComposer::new();
    let profile = composer.parse(&content)
        .expect("Failed to parse profile");
    
    assert!(profile.execution_tree.is_some(), "execution_tree should exist");
    
    let tree = profile.execution_tree.unwrap();
    
    // 检查是否有节点有 specialized metrics
    let nodes_with_specialized: Vec<_> = tree.nodes.iter()
        .filter(|n| !matches!(n.metrics.specialized, starrocks_profile_analyzer::models::OperatorSpecializedMetrics::None))
        .collect();
    
    println!("Total nodes: {}", tree.nodes.len());
    println!("Nodes with specialized metrics: {}", nodes_with_specialized.len());
    
    for node in tree.nodes.iter() {
        println!("\nNode: {}", node.operator_name);
        println!("  Specialized: {:?}", node.metrics.specialized);
    }
    
    // 验证拓扑名称与Operator匹配: 应存在OLAP_SCAN节点而非CONNECTOR_SCAN
    let has_olap_scan = tree.nodes.iter().any(|n| n.operator_name == "OLAP_SCAN");
    assert!(has_olap_scan, "Execution tree should contain OLAP_SCAN node derived from CONNECTOR_SCAN");

    let has_connector_scan = tree.nodes.iter().any(|n| n.operator_name == "CONNECTOR_SCAN");
    assert!(!has_connector_scan, "Execution tree should not expose CONNECTOR_SCAN after normalization");

    // 目前可能没有解析，这个测试会显示现状
}
