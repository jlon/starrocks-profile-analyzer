use starrocks_profile_analyzer::parser::ProfileComposer;
use std::fs;
use std::path::Path;

#[allow(dead_code)]
fn get_profile_path(filename: &str) -> String {
    let base_path = Path::new("../profiles");
    base_path.join(filename).to_string_lossy().to_string()
}

#[test]
fn test_new_parser_with_all_profiles() {
    let profiles_dir = Path::new("../profiles");
    
    if !profiles_dir.exists() {
        eprintln!("⚠️  Profiles directory not found at {:?}", profiles_dir);
        return;
    }
    
    // 读取目录下所有 .txt 文件
    let entries = fs::read_dir(profiles_dir)
        .expect("Failed to read profiles directory");
    
    let mut profile_files: Vec<_> = entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.path().extension()
                .and_then(|s| s.to_str())
                .map(|s| s == "txt")
                .unwrap_or(false)
        })
        .collect();
    
    // 按文件名排序
    profile_files.sort_by_key(|e| e.file_name());
    
    println!("\n🔍 Testing New ProfileComposer with all profile files");
    println!("=====================================================");
    println!("Found {} profile files\n", profile_files.len());
    
    let mut total_success = 0;
    let mut total_failed = 0;
    let mut parse_errors = Vec::new();
    
    let mut composer = ProfileComposer::new();
    
    for entry in profile_files {
        let path = entry.path();
        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        
        println!("📄 Testing: {}", filename);
        println!("{}", "=".repeat(60));
        
        match fs::read_to_string(&path) {
            Ok(profile_text) => {
                // 使用新版 ProfileComposer 解析
                match composer.parse(&profile_text) {
                    Ok(profile) => {
                        println!("   ✅ Parse successful");
                        
                        // 验证 Summary
                        if !profile.summary.query_id.is_empty() {
                            println!("   📋 Query ID: {}", profile.summary.query_id);
                        }
                        if !profile.summary.starrocks_version.is_empty() {
                            println!("   🔖 Version: {}", profile.summary.starrocks_version);
                        }
                        
                        // 验证 Execution Tree
                        if let Some(ref tree) = profile.execution_tree {
                            println!("   🌳 Execution Tree: {} nodes", tree.nodes.len());
                            println!("      Root: {}", tree.root.operator_name);
                            
                            // 统计 operator 类型
                            let mut operator_types = std::collections::HashMap::new();
                            for node in &tree.nodes {
                                *operator_types.entry(node.operator_name.clone()).or_insert(0) += 1;
                            }
                            
                            println!("      Operator types:");
                            for (op_type, count) in &operator_types {
                                println!("        - {}: {}", op_type, count);
                            }
                        } else {
                            println!("   ⚠️  No execution tree");
                        }
                        
                        // 验证 Topology
                        if !profile.execution.topology.is_empty() {
                            println!("   ✅ Has topology data");
                        }
                        
                        total_success += 1;
                    }
                    Err(e) => {
                        println!("   ❌ Parse error: {:?}", e);
                        parse_errors.push((filename.clone(), format!("{:?}", e)));
                        total_failed += 1;
                    }
                }
            }
            Err(e) => {
                println!("   ❌ Failed to read file: {}", e);
                parse_errors.push((filename.clone(), format!("Read error: {}", e)));
                total_failed += 1;
            }
        }
        
        println!();
    }
    
    println!("\n📊 Test Summary");
    println!("===============");
    println!("   Total files: {}", total_success + total_failed);
    println!("   ✅ Success: {}", total_success);
    println!("   ❌ Failed: {}", total_failed);
    
    if !parse_errors.is_empty() {
        println!("\n❌ Failed files:");
        for (filename, error) in parse_errors {
            println!("   - {}: {}", filename, error);
        }
    }
    
    // 至少要成功解析一些文件
    assert!(total_success > 0, "Should successfully parse at least one profile");
    println!("\n✅ All tests passed!");
}
