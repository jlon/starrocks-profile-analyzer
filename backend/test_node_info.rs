#!/usr/bin/env rust-script
//! 测试NodeInfo实现是否正确解析profile

use std::fs;

fn main() {
    // 测试profile2.txt
    println!("=== Testing profile2.txt ===\n");
    test_profile("../profiles/profile2.txt", vec![
        ("RESULT_SINK", 3.56),
        ("EXCHANGE", 45.73),
        ("SCHEMA_SCAN", 50.75),
    ]);
    
    println!("\n=== Testing profile5.txt ===\n");
    test_profile("../profiles/profile5.txt", vec![
        ("OLAP_TABLE_SINK", 35.73),
        ("PROJECT", 5.64),
        ("TABLE_FUNCTION", 59.07),
    ]);
}

fn test_profile(path: &str, expected: Vec<(&str, f64)>) {
    let profile_text = fs::read_to_string(path)
        .expect(&format!("Failed to read {}", path));
    
    // 使用analyze_profile函数
    match starrocks_profile_analyzer::analyze_profile(&profile_text) {
        Ok(result) => {
            println!("✅ Successfully parsed {}", path);
            
            if let Some(ref tree) = result.execution_tree {
                println!("Nodes found: {}", tree.nodes.len());
                
                for (node_name, expected_pct) in expected {
                    if let Some(node) = tree.nodes.iter()
                        .find(|n| n.operator_name == node_name) 
                {
                    if let Some(actual_pct) = node.time_percentage {
                        let diff = (actual_pct - expected_pct).abs();
                        if diff < 1.0 {
                            println!("✅ {}: {:.2}% (expected {:.2}%, diff {:.2}%)", 
                                node_name, actual_pct, expected_pct, diff);
                        } else {
                            println!("❌ {}: {:.2}% (expected {:.2}%, diff {:.2}%)", 
                                node_name, actual_pct, expected_pct, diff);
                        }
                    } else {
                        println!("❌ {}: No percentage calculated", node_name);
                    }
                    } else {
                        println!("❌ {}: Node not found", node_name);
                    }
                }
            } else {
                println!("❌ No execution tree found");
            }
        }
        Err(e) => {
            println!("❌ Failed to parse {}: {}", path, e);
        }
    }
}

