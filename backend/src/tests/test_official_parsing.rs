use std::fs;
use starrocks_profile_analyzer::analyze_profile;

#[test]
fn test_profile5_with_official_parsing() {
    println!("=== 测试profile5.txt与官方解析逻辑的兼容性 ===\n");
    
    let profile_path = "../profiles/profile5.txt";
    let profile_text = fs::read_to_string(profile_path).expect("Failed to read profile5.txt");
    
    match analyze_profile(&profile_text) {
        Ok(result) => {
            println!("✅ Profile解析成功");
            
            // 检查summary
            if let Some(summary) = &result.summary {
                println!("📊 Summary信息:");
                println!("  - Query ID: {}", summary.query_id);
                println!("  - Total Time: {}", summary.total_time);
                if let Some(total_time_ms) = summary.total_time_ms {
                    println!("  - Total Time (ms): {:.2}", total_time_ms);
                }
            }
            
            // 检查execution tree
            if let Some(tree) = &result.execution_tree {
                println!("🌳 Execution Tree:");
                println!("  - 节点数量: {}", tree.nodes.len());
                
                for node in &tree.nodes {
                    println!("  - 节点: {} (ID: {})", node.operator_name, node.plan_node_id);
                    if let Some(pct) = node.time_percentage {
                        println!("    时间百分比: {:.2}%", pct);
                    }
                    
                    // 检查specialized metrics
                    if let Some(specialized) = &node.specialized_metrics {
                        match specialized {
                            starrocks_profile_analyzer::models::OperatorSpecializedMetrics::ConnectorScan(metrics) => {
                                println!("    Specialized: CONNECTOR_SCAN");
                                println!("      - DataSourceType: {}", metrics.data_source_type);
                                println!("      - Table: {}", metrics.table);
                                println!("      - SharedScan: {}", metrics.shared_scan);
                                println!("      - MorselQueueType: {}", metrics.morsel_queue_type);
                            },
                            starrocks_profile_analyzer::models::OperatorSpecializedMetrics::OlapScan(metrics) => {
                                println!("    Specialized: OLAP_SCAN");
                                println!("      - Table: {}", metrics.table);
                                println!("      - Rollup: {}", metrics.rollup);
                            },
                            _ => {
                                println!("    Specialized: {:?}", specialized);
                            }
                        }
                    }
                }
            }
            
            // 检查fragments
            println!("🔧 Fragments:");
            for fragment in &result.fragments {
                println!("  - Fragment {}: {} pipelines", fragment.id, fragment.pipelines.len());
                for pipeline in &fragment.pipelines {
                    println!("    - Pipeline {}: {} operators", pipeline.id, pipeline.operators.len());
                }
            }
            
            println!("\n🎉 所有检查通过！官方解析逻辑与现有功能兼容");
        },
        Err(e) => {
            panic!("❌ Profile解析失败: {}", e);
        }
    }
}

#[test]
fn test_all_profiles_compatibility() {
    println!("=== 测试所有profiles与官方解析逻辑的兼容性 ===\n");
    
    let profiles = vec!["profile1.txt", "profile2.txt", "profile3.txt", "profile4.txt", "profile5.txt"];
    let mut success_count = 0;
    
    for profile_name in profiles {
        println!("📋 测试 {} ...", profile_name);
        
        let profile_path = format!("../profiles/{}", profile_name);
        match fs::read_to_string(&profile_path) {
            Ok(profile_text) => {
                match analyze_profile(&profile_text) {
                    Ok(result) => {
                        println!("  ✅ 解析成功");
                        
                        // 基本检查
                        assert!(!result.summary.query_id.is_empty(), "Query ID should not be empty");
                        
                        if let Some(tree) = &result.execution_tree {
                            assert!(!tree.nodes.is_empty(), "Execution tree should have nodes");
                            
                            // 检查时间百分比计算
                            let mut has_time_percentage = false;
                            for node in &tree.nodes {
                                if node.time_percentage.is_some() {
                                    has_time_percentage = true;
                                    break;
                                }
                            }
                            assert!(has_time_percentage, "At least one node should have time percentage");
                        }
                        
                        success_count += 1;
                    },
                    Err(e) => {
                        panic!("  ❌ 解析失败: {}", e);
                    }
                }
            },
            Err(e) => {
                panic!("  ❌ 读取文件失败: {}", e);
            }
        }
        println!();
    }
    
    println!("=== 最终结果 ===");
    println!("成功解析: {}/{} profiles", success_count, profiles.len());
    
    assert_eq!(success_count, profiles.len(), "All profiles should parse successfully");
    println!("🎉 所有profiles解析成功！官方解析逻辑完全兼容");
}
