#!/usr/bin/env rust-script
//! 详细验证每个节点的所有指标（时间、行数等）

use std::fs;

fn main() {
    println!("=== 详细验证所有节点的指标 ===\n");
    
    // 定义每个profile的详细期望值（从官方PNG图片中提取）
    let test_cases = vec![
        ("profile2.txt", vec![
            NodeExpectation {
                name: "RESULT_SINK",
                time_percentage: Some(3.56),
                output_rows: Some(11),
                // 从图片中提取其他指标
            },
            NodeExpectation {
                name: "EXCHANGE",
                time_percentage: Some(45.73),
                output_rows: None, // 需要从图片确认
            },
            NodeExpectation {
                name: "SCHEMA_SCAN",
                time_percentage: Some(50.75),
                output_rows: None, // 需要从图片确认
            },
        ]),
        ("profile3.txt", vec![
            NodeExpectation {
                name: "OLAP_SCAN",
                time_percentage: Some(99.97),
                output_rows: None, // 需要从图片确认
            },
        ]),
        ("profile4.txt", vec![
            NodeExpectation {
                name: "RESULT_SINK",
                time_percentage: Some(97.43),
                output_rows: None, // 需要从图片确认
            },
            NodeExpectation {
                name: "MERGE_EXCHANGE",
                time_percentage: Some(2.64),
                output_rows: None, // 需要从图片确认
            },
        ]),
        ("profile5.txt", vec![
            NodeExpectation {
                name: "OLAP_TABLE_SINK",
                time_percentage: Some(35.73),
                output_rows: None, // 需要从图片确认
            },
            NodeExpectation {
                name: "PROJECT",
                time_percentage: Some(5.64),
                output_rows: None, // 需要从图片确认
            },
            NodeExpectation {
                name: "TABLE_FUNCTION",
                time_percentage: Some(59.07),
                output_rows: None, // 需要从图片确认
            },
        ]),
    ];
    
    let mut total_checks = 0;
    let mut passed_checks = 0;
    let mut failed_checks = 0;
    
    for (filename, expectations) in test_cases {
        println!("📋 验证 {} ...", filename);
        
        let profile_path = format!("../profiles/{}", filename);
        match fs::read_to_string(&profile_path) {
            Ok(profile_text) => {
                match starrocks_profile_analyzer::analyze_profile(&profile_text) {
                    Ok(result) => {
                        if let Some(ref tree) = result.execution_tree {
                            for expectation in &expectations {
                                println!("\n  🔍 节点: {}", expectation.name);
                                
                                if let Some(node) = tree.nodes.iter().find(|n| n.operator_name == expectation.name) {
                                    // 验证时间百分比
                                    if let Some(expected_pct) = expectation.time_percentage {
                                        total_checks += 1;
                                        if let Some(actual_pct) = node.time_percentage {
                                            let diff = (actual_pct - expected_pct).abs();
                                            if diff < 1.0 {
                                                println!("    ✅ 时间百分比: {:.2}% (expected {:.2}%, diff {:.2}%)", 
                                                    actual_pct, expected_pct, diff);
                                                passed_checks += 1;
                                            } else {
                                                println!("    ❌ 时间百分比: {:.2}% (expected {:.2}%, diff {:.2}%) - FAILED", 
                                                    actual_pct, expected_pct, diff);
                                                failed_checks += 1;
                                            }
                                        } else {
                                            println!("    ❌ 时间百分比: 未计算");
                                            failed_checks += 1;
                                        }
                                    }
                                    
                                    // 验证输出行数
                                    if let Some(expected_rows) = expectation.output_rows {
                                        total_checks += 1;
                                        // 从metrics中获取输出行数
                                        let actual_rows = node.metrics.pull_row_num.or(node.metrics.push_row_num);
                                        if let Some(rows) = actual_rows {
                                            if rows == expected_rows {
                                                println!("    ✅ 输出行数: {} (expected {})", rows, expected_rows);
                                                passed_checks += 1;
                                            } else {
                                                println!("    ❌ 输出行数: {} (expected {}) - FAILED", rows, expected_rows);
                                                failed_checks += 1;
                                            }
                                        } else {
                                            println!("    ⚠️  输出行数: 未找到");
                                        }
                                    }
                                    
                                    // 显示其他关键指标（用于手动对比）
                                    println!("    📊 其他指标:");
                                    if let Some(time_ns) = node.metrics.operator_total_time {
                                        let time_ms = time_ns as f64 / 1_000_000.0;
                                        println!("       - OperatorTotalTime: {:.3}ms", time_ms);
                                    }
                                    if let Some(rows) = node.metrics.pull_row_num {
                                        println!("       - PullRowNum: {}", rows);
                                    }
                                    if let Some(rows) = node.metrics.push_row_num {
                                        println!("       - PushRowNum: {}", rows);
                                    }
                                    if let Some(bytes) = node.metrics.memory_usage {
                                        println!("       - MemoryUsage: {} bytes", bytes);
                                    }
                                } else {
                                    println!("    ❌ 节点未找到");
                                    failed_checks += expectations.len();
                                    total_checks += expectations.len();
                                }
                            }
                        } else {
                            println!("  ❌ 没有执行树");
                        }
                    }
                    Err(e) => {
                        println!("  ❌ 解析失败: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("  ❌ 读取文件失败: {}", e);
            }
        }
        println!();
    }
    
    println!("\n=== 最终结果 ===");
    println!("总检查项: {}", total_checks);
    println!("通过: {} ({:.1}%)", passed_checks, 
        if total_checks > 0 { passed_checks as f64 * 100.0 / total_checks as f64 } else { 0.0 });
    println!("失败: {} ({:.1}%)", failed_checks,
        if total_checks > 0 { failed_checks as f64 * 100.0 / total_checks as f64 } else { 0.0 });
    
    if failed_checks > 0 {
        println!("\n⚠️  警告: 有{}个检查项失败", failed_checks);
        std::process::exit(1);
    } else {
        println!("\n🎉 所有检查项通过！");
    }
}

#[derive(Debug)]
struct NodeExpectation {
    name: &'static str,
    time_percentage: Option<f64>,
    output_rows: Option<u64>,
}

