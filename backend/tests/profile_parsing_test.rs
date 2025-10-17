//! # Profile 解析集成测试
//! 
//! 使用真实的 Profile 文件测试完整的解析流程

use starrocks_profile_analyzer::parser::ProfileComposer;
use std::fs;
use std::path::PathBuf;

fn get_test_profile_path(filename: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // 回到项目根目录
    path.push("profiles");
    path.push(filename);
    path
}

#[test]
fn test_parse_test_profile() {
    let path = get_test_profile_path("test_profile.txt");
    
    if !path.exists() {
        println!("Skipping test: profile file not found at {:?}", path);
        return;
    }
    
    let profile_text = fs::read_to_string(&path).unwrap();
    let mut composer = ProfileComposer::new();
    
    let profile = composer.parse(&profile_text).expect("Failed to parse profile");
    
    // 验证 Summary
    assert!(!profile.summary.query_id.is_empty());
    assert!(!profile.summary.start_time.is_empty());
    assert!(!profile.summary.total_time.is_empty());
    assert_eq!(profile.summary.query_state, "Finished");
    
    // 验证时间解析
    if let Some(total_ms) = profile.summary.total_time_ms {
        assert!(total_ms > 0);
    }
    
    // 验证内存解析
    if let Some(peak_mem) = profile.summary.query_peak_memory {
        assert!(peak_mem > 0);
    }
    
    println!("✓ test_profile.txt parsed successfully");
    println!("  Query ID: {}", profile.summary.query_id);
    println!("  Total Time: {}", profile.summary.total_time);
    println!("  Query State: {}", profile.summary.query_state);
}

#[test]
fn test_parse_profile1() {
    let path = get_test_profile_path("profile1.txt");
    
    if !path.exists() {
        println!("Skipping test: profile file not found at {:?}", path);
        return;
    }
    
    let profile_text = fs::read_to_string(&path).unwrap();
    let mut composer = ProfileComposer::new();
    
    let profile = composer.parse(&profile_text).expect("Failed to parse profile1");
    
    // 验证基本信息
    assert!(!profile.summary.query_id.is_empty());
    assert_eq!(profile.summary.query_state, "Finished");
    
    println!("✓ profile1.txt parsed successfully");
    println!("  Query ID: {}", profile.summary.query_id);
}

#[test]
fn test_parse_all_profiles() {
    let profiles_dir = get_test_profile_path("");
    
    if !profiles_dir.exists() {
        println!("Skipping test: profiles directory not found");
        return;
    }
    
    let mut composer = ProfileComposer::new();
    let mut success_count = 0;
    let mut total_count = 0;
    
    // 遍历所有 profile 文件
    if let Ok(entries) = fs::read_dir(&profiles_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("txt") {
                    total_count += 1;
                    let filename = path.file_name().unwrap().to_str().unwrap();
                    
                    println!("\nTesting: {}", filename);
                    
                    match fs::read_to_string(&path) {
                        Ok(content) => {
                            match composer.parse(&content) {
                                Ok(profile) => {
                                    println!("  ✓ Parsed successfully");
                                    println!("    Query ID: {}", profile.summary.query_id);
                                    println!("    Query State: {}", profile.summary.query_state);
                                    
                                    if let Some(tree) = &profile.execution_tree {
                                        println!("    Nodes: {}", tree.nodes.len());
                                    }
                                    
                                    // Note: bottlenecks field has been removed from Profile struct
                                    // Use HotspotDetector directly if needed
                                    // if let Some(bottlenecks) = &profile.bottlenecks {
                                    //     println!("    Bottlenecks: {}", bottlenecks.len());
                                    // }
                                    
                                    success_count += 1;
                                },
                                Err(e) => {
                                    println!("  ✗ Parse failed: {}", e);
                                }
                            }
                        },
                        Err(e) => {
                            println!("  ✗ Read failed: {}", e);
                        }
                    }
                }
            }
        }
    }
    
    println!("\n========================================");
    println!("Total: {}, Success: {}, Failed: {}", 
        total_count, success_count, total_count - success_count);
    println!("========================================");
    
    if total_count > 0 {
        let success_rate = (success_count as f64 / total_count as f64) * 100.0;
        println!("Success Rate: {:.1}%", success_rate);
        
        // 至少要有 80% 的成功率
        assert!(success_rate >= 80.0, 
            "Success rate too low: {:.1}%", success_rate);
    }
}

#[test]
fn test_execution_tree_structure() {
    let path = get_test_profile_path("test_profile.txt");
    
    if !path.exists() {
        println!("Skipping test: profile file not found");
        return;
    }
    
    let profile_text = fs::read_to_string(&path).unwrap();
    let mut composer = ProfileComposer::new();
    let profile = composer.parse(&profile_text).unwrap();
    
    if let Some(tree) = profile.execution_tree {
        // 验证树结构
        assert!(!tree.nodes.is_empty(), "Execution tree should have nodes");
        
        // 验证根节点
        assert!(!tree.root.operator_name.is_empty());
        
        // 验证节点深度
        for node in &tree.nodes {
            // 深度应该是合理的 (0-10)
            assert!(node.depth <= 10, "Node depth too large: {}", node.depth);
        }
        
        println!("✓ Execution tree structure validated");
        println!("  Total nodes: {}", tree.nodes.len());
        println!("  Root operator: {}", tree.root.operator_name);
    }
}

#[test]
fn test_hotspot_detection() {
    let path = get_test_profile_path("test_profile.txt");
    
    if !path.exists() {
        println!("Skipping test: profile file not found");
        return;
    }
    
    let profile_text = fs::read_to_string(&path).unwrap();
    let mut composer = ProfileComposer::new();
    let profile = composer.parse(&profile_text).unwrap();
    
    if let Some(tree) = profile.execution_tree {
        // 检查是否有热点标记
        let hotspot_count = tree.nodes.iter().filter(|n| n.is_hotspot).count();
        
        println!("✓ Hotspot detection completed");
        println!("  Total nodes: {}", tree.nodes.len());
        println!("  Hotspot nodes: {}", hotspot_count);
    }
    
    // Note: bottlenecks field has been removed from Profile struct
    // Use HotspotDetector directly if needed
    // if let Some(bottlenecks) = profile.bottlenecks {
    //     println!("  Bottlenecks identified: {}", bottlenecks.len());
    //     
    //     for (i, bn) in bottlenecks.iter().take(3).enumerate() {
    //         println!("    #{} {} - {:.1}%", 
    //             i + 1, bn.operator_name, bn.time_percentage);
    //     }
    // }
}

#[test]
fn test_metrics_extraction() {
    let path = get_test_profile_path("test_profile.txt");
    
    if !path.exists() {
        println!("Skipping test: profile file not found");
        return;
    }
    
    let profile_text = fs::read_to_string(&path).unwrap();
    let mut composer = ProfileComposer::new();
    let profile = composer.parse(&profile_text).unwrap();
    
    // 验证 Execution 指标
    assert!(!profile.execution.metrics.is_empty(), 
        "Execution metrics should not be empty");
    
    println!("✓ Metrics extracted successfully");
    println!("  Execution metrics: {}", profile.execution.metrics.len());
    println!("  Planner details: {}", profile.planner.details.len());
    
    // 打印一些关键指标
    for (key, value) in profile.execution.metrics.iter().take(5) {
        println!("    {}: {}", key, value);
    }
}

