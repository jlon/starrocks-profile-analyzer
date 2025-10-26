use std::fs;

fn main() {
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
                match starrocks_profile_analyzer::analyze_profile(&profile_text) {
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
        std::process::exit(1);
    } else if total_tests == 0 {
        println!("\n⚠️  警告: 没有足够的测试数据，请从PNG图片中提取期望值");
        std::process::exit(1);
    } else {
        println!("\n🎉 所有测试通过！");
    }
}
