pub mod parser;
pub mod models;
pub mod analyzer;
pub mod api;
pub mod constants;
pub mod static_files;

pub use models::*;
pub use analyzer::hotspot_detector::HotSpotDetector;
pub use analyzer::suggestion_engine::SuggestionEngine;
pub use parser::ProfileComposer;

pub fn analyze_profile(profile_text: &str) -> Result<ProfileAnalysisResponse, String> {
    let mut composer = ProfileComposer::new();
    let profile = composer.parse(profile_text)
        .map_err(|e| format!("解析Profile失败: {:?}", e))?;

    let hotspots = HotSpotDetector::analyze(&profile);
    let conclusion = SuggestionEngine::generate_conclusion(&hotspots, &profile);
    let suggestions = SuggestionEngine::generate_suggestions(&hotspots);
    let performance_score = SuggestionEngine::calculate_performance_score(&hotspots, &profile);
    let execution_tree = profile.execution_tree.clone();
    let summary = Some(profile.summary.clone());

    Ok(ProfileAnalysisResponse {
        hotspots,
        conclusion,
        suggestions,
        performance_score,
        execution_tree,
        summary,
    })
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::fs;

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
                        println!("  - 节点: {} (ID: {:?})", node.operator_name, node.plan_node_id);
                        if let Some(pct) = node.time_percentage {
                            println!("    时间百分比: {:.2}%", pct);
                        }
                        
                        // 检查metrics
                        println!("    Metrics:");
                        if let Some(time_ns) = node.metrics.operator_total_time {
                            let time_ms = time_ns as f64 / 1_000_000.0;
                            println!("      - OperatorTotalTime: {:.3}ms", time_ms);
                        }
                        if let Some(rows) = node.metrics.pull_row_num {
                            println!("      - PullRowNum: {}", rows);
                        }
                        if let Some(rows) = node.metrics.push_row_num {
                            println!("      - PushRowNum: {}", rows);
                        }
                    }
                }
                
                // 检查hotspots
                println!("🔥 Hotspots:");
                for hotspot in &result.hotspots {
                    println!("  - {}: {:?}", hotspot.node_path, hotspot.severity);
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
        
        for profile_name in &profiles {
            println!("📋 测试 {} ...", profile_name);
            
            let profile_path = format!("../profiles/{}", profile_name);
            match fs::read_to_string(&profile_path) {
                Ok(profile_text) => {
                    match analyze_profile(&profile_text) {
                        Ok(result) => {
                            println!("  ✅ 解析成功");
                            
                            // 基本检查
                            if let Some(summary) = &result.summary {
                                assert!(!summary.query_id.is_empty(), "Query ID should not be empty");
                            }
                            
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

    #[test]
    fn test_validate_all_profiles_regression() {
        println!("=== 严格验证所有profiles与官方图片的一致性 ===\n");
        
        let test_cases = vec![
            ("profile1.txt", vec![
            ]),
            ("profile2.txt", vec![
                ("RESULT_SINK", 3.56),
                ("EXCHANGE", 45.73),
                ("SCHEMA_SCAN", 50.75),
            ]),
            ("profile3.txt", vec![
                ("OLAP_SCAN", 99.97),
            ]),
            ("profile4.txt", vec![
                ("RESULT_SINK", 97.43),
                ("MERGE_EXCHANGE", 2.64),
            ]),
            ("profile5.txt", vec![
                ("OLAP_TABLE_SINK", 35.73),
                ("PROJECT", 5.64),
                ("TABLE_FUNCTION", 59.07),
            ]),
        ];
        
        let mut total_tests = 0;
        let mut passed_tests = 0;
        let mut failed_tests = 0;
        
        for (filename, expected_nodes) in test_cases {
            if expected_nodes.is_empty() {
                println!("⚠️  {} - 需要手动从PNG提取期望值", filename);
                continue;
            }
            
            println!("📋 Testing {} ...", filename);
            
            let profile_path = format!("../profiles/{}", filename);
            match fs::read_to_string(&profile_path) {
                Ok(profile_text) => {
                    match analyze_profile(&profile_text) {
                        Ok(result) => {
                            if let Some(ref tree) = result.execution_tree {
                                for (node_name, expected_pct) in &expected_nodes {
                                    total_tests += 1;
                                    
                                    if let Some(node) = tree.nodes.iter().find(|n| n.operator_name == *node_name) {
                                        if let Some(actual_pct) = node.time_percentage {
                                            let diff = (actual_pct - expected_pct).abs();
                                            
                                            if diff < 1.0 {
                                                println!("  ✅ {}: {:.2}% (expected {:.2}%, diff {:.2}%)", 
                                                    node_name, actual_pct, expected_pct, diff);
                                                passed_tests += 1;
                                            } else {
                                                println!("  ❌ {}: {:.2}% (expected {:.2}%, diff {:.2}%) - FAILED", 
                                                    node_name, actual_pct, expected_pct, diff);
                                                failed_tests += 1;
                                            }
                                        } else {
                                            println!("  ❌ {}: No percentage calculated", node_name);
                                            failed_tests += 1;
                                        }
                                    } else {
                                        println!("  ❌ {}: Node not found", node_name);
                                        failed_tests += 1;
                                    }
                                }
                            } else {
                                println!("  ❌ No execution tree found");
                                failed_tests += expected_nodes.len();
                                total_tests += expected_nodes.len();
                            }
                        }
                        Err(e) => {
                            println!("  ❌ Parse error: {}", e);
                            failed_tests += expected_nodes.len();
                            total_tests += expected_nodes.len();
                        }
                    }
                }
                Err(e) => {
                    println!("  ❌ Read error: {}", e);
                    failed_tests += expected_nodes.len();
                    total_tests += expected_nodes.len();
                }
            }
            println!();
        }
        
        println!("=== 最终结果 ===");
        println!("总测试数: {}", total_tests);
        println!("通过: {} ({:.1}%)", passed_tests, 
            if total_tests > 0 { passed_tests as f64 * 100.0 / total_tests as f64 } else { 0.0 });
        println!("失败: {} ({:.1}%)", failed_tests,
            if total_tests > 0 { failed_tests as f64 * 100.0 / total_tests as f64 } else { 0.0 });
        
        if failed_tests > 0 {
            println!("\n⚠️  警告: 有{}个测试失败，需要进一步优化解析逻辑", failed_tests);
            panic!("Regression test failed: {} tests failed", failed_tests);
        } else if total_tests == 0 {
            println!("\n⚠️  警告: 没有足够的测试数据，请从PNG图片中提取期望值");
            panic!("No test data available");
        } else {
            println!("\n🎉 所有测试通过！");
        }
    }
}
