#!/usr/bin/env rust-script
//! 测试所有profiles的解析

use std::fs;
use std::path::Path;

fn main() {
    let profiles_dir = "../profiles";
    
    // 读取所有.txt文件
    let mut profiles = Vec::new();
    if let Ok(entries) = fs::read_dir(profiles_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("txt") {
                    profiles.push(path);
                }
            }
        }
    }
    
    profiles.sort();
    
    println!("=== Testing {} profiles ===\n", profiles.len());
    
    let mut success_count = 0;
    let mut fail_count = 0;
    
    for profile_path in profiles {
        let filename = profile_path.file_name().unwrap().to_str().unwrap();
        print!("Testing {:<30} ... ", filename);
        
        match fs::read_to_string(&profile_path) {
            Ok(profile_text) => {
                match starrocks_profile_analyzer::analyze_profile(&profile_text) {
                    Ok(result) => {
                        if let Some(ref tree) = result.execution_tree {
                            println!("✅ OK ({} nodes)", tree.nodes.len());
                            success_count += 1;
                        } else {
                            println!("❌ No execution tree");
                            fail_count += 1;
                        }
                    }
                    Err(e) => {
                        println!("❌ Parse error: {}", e);
                        fail_count += 1;
                    }
                }
            }
            Err(e) => {
                println!("❌ Read error: {}", e);
                fail_count += 1;
            }
        }
    }
    
    println!("\n=== Summary ===");
    println!("Total: {}", success_count + fail_count);
    println!("Success: {} ({}%)", success_count, 
        if success_count + fail_count > 0 { 
            success_count * 100 / (success_count + fail_count) 
        } else { 
            0 
        });
    println!("Failed: {}", fail_count);
}

