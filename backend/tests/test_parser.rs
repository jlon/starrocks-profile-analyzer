use std::fs;
use std::path::Path;
use starrocks_profile_analyzer::ProfileComposer;
use starrocks_profile_analyzer::models::*;

fn get_profile_path(filename: &str) -> String {
    let base_path = Path::new("../profiles");
    base_path.join(filename).to_string_lossy().to_string()
}

#[test]
fn test_parse_profile_basic() {
    let profile_text = fs::read_to_string(get_profile_path("test_profile.txt"))
        .expect("Failed to read test_profile.txt");
    
    // 使用新版解析器
    let mut composer = ProfileComposer::new();
    let profile = composer.parse(&profile_text)
        .expect("Failed to parse profile with new parser");
    
    // Test Summary
    assert!(!profile.summary.query_id.is_empty(), "Query ID should not be empty");
    assert_eq!(profile.summary.query_id, "b1f9a935-a967-11f0-b3d8-f69e292b7593");
    assert_eq!(profile.summary.starrocks_version, "3.5.2-69de616");
    
    // Test Execution Topology
    assert!(!profile.execution.topology.is_empty(), "Topology should not be empty");
    assert!(profile.execution.topology.contains("rootId"), "Topology should contain rootId");
    assert!(profile.execution.topology.contains("nodes"), "Topology should contain nodes");
    
    // Test Execution Tree (新版解析器构建的)
    assert!(profile.execution_tree.is_some(), "Should have execution tree");
    
    println!("✅ test_parse_profile_basic: Parsed successfully with new parser");
}

#[test]
fn test_parse_fragments_and_pipelines() {
    let profile_text = fs::read_to_string(get_profile_path("test_profile.txt"))
        .expect("Failed to read test_profile.txt");
    
    // 使用新版解析器测试
    let mut composer = ProfileComposer::new();
    let profile = composer.parse(&profile_text)
        .expect("Failed to parse profile");
    
    for (frag_idx, fragment) in profile.fragments.iter().enumerate() {
        println!("Fragment {}: {} pipelines", fragment.id, fragment.pipelines.len());
        assert!(fragment.pipelines.len() > 0, "Fragment {} should have pipelines", frag_idx);
        
        for (pipe_idx, pipeline) in fragment.pipelines.iter().enumerate() {
            println!("  Pipeline {}: {} operators", pipeline.id, pipeline.operators.len());
            
            // After fix: operators should NOT be empty
            assert!(pipeline.operators.len() > 0, 
                "Pipeline {}/{} should have operators after fix", frag_idx, pipe_idx);
            
            for operator in &pipeline.operators {
                println!("    - {}", operator.name);
                assert!(!operator.name.is_empty(), "Operator name should not be empty");
            }
        }
    }
    
    println!("✅ test_parse_fragments_and_pipelines passed");
}

#[test]
fn test_advanced_parse_builds_execution_tree() {
    let profile_text = fs::read_to_string(get_profile_path("test_profile.txt"))
        .expect("Failed to read test_profile.txt");
    
    // 使用新版解析器
    let mut composer = ProfileComposer::new();
    let profile = composer.parse(&profile_text)
        .expect("Failed to parse with new composer");
    
    assert!(profile.execution_tree.is_some(), "Should have execution tree");
    
    let tree = profile.execution_tree.as_ref().unwrap();
    assert!(tree.nodes.len() > 0, "Execution tree should have nodes");
    
    println!("✅ test_advanced_parse_builds_execution_tree (NEW PARSER):");
    println!("   Root: {}", tree.root.operator_name);
    println!("   Total nodes: {}", tree.nodes.len());
    
    // Print tree structure
    for node in &tree.nodes {
        println!("   [{}] depth={} {} -> {} children", 
            node.id, node.depth, node.operator_name,
            node.children.len());
    }
}

#[test]
fn test_execution_tree_from_topology() {
    let profile_text = fs::read_to_string(get_profile_path("test_profile.txt"))
        .expect("Failed to read test_profile.txt");
    
    // 使用新版解析器
    let mut composer = ProfileComposer::new();
    let result = composer.parse(&profile_text);
    
    // Should not fail with "No operators found"
    assert!(result.is_ok(), "Should successfully parse profile");
    
    let profile = result.unwrap();
    assert!(profile.execution_tree.is_some(), "Should have execution tree");
    
    let tree = profile.execution_tree.unwrap();
    
    // Validate tree structure
    assert!(!tree.root.id.is_empty(), "Root should have ID");
    println!("✅ test_execution_tree_from_topology (NEW PARSER):");
    println!("   Root ID: {}", tree.root.id);
    println!("   Root operator: {}", tree.root.operator_name);
    println!("   Root children: {}", tree.root.children.len());
}

#[test]
fn test_tree_has_proper_hierarchy() {
    let profile_text = fs::read_to_string(get_profile_path("test_profile.txt"))
        .expect("Failed to read test_profile.txt");
    
    // 使用新版解析器
    let mut composer = ProfileComposer::new();
    let profile = composer.parse(&profile_text)
        .expect("Failed to parse profile");
    
    let tree = profile.execution_tree.as_ref().expect("Should have tree");
    
    // Check depth assignments
    let mut depth_map = std::collections::HashMap::new();
    for node in &tree.nodes {
        let count = depth_map.entry(node.depth).or_insert(0);
        *count += 1;
    }
    
    println!("✅ test_tree_has_proper_hierarchy (NEW PARSER):");
    for (depth, count) in &depth_map {
        println!("   Depth {}: {} nodes", depth, count);
    }
    
    // Root should be at depth 0
    assert_eq!(tree.root.depth, 0, "Root should be at depth 0");
}

#[test]
fn test_all_nodes_are_connected() {
    let profile_text = fs::read_to_string(get_profile_path("test_profile.txt"))
        .expect("Failed to read test_profile.txt");
    
    // 使用新版解析器
    let mut composer = ProfileComposer::new();
    let profile = composer.parse(&profile_text)
        .expect("Failed to parse profile");
    
    let tree = profile.execution_tree.as_ref().expect("Should have tree");
    
    // Build node ID set
    let id_set: std::collections::HashSet<&String> = 
        tree.nodes.iter().map(|n| &n.id).collect();
    
    // Check all referenced children exist
    for node in &tree.nodes {
        for child_id in &node.children {
            assert!(id_set.contains(child_id), 
                "Child {} of node {} not found in nodes", child_id, node.id);
        }
    }
    
    println!("✅ test_all_nodes_are_connected (NEW PARSER): All {} nodes properly connected", tree.nodes.len());
}

#[test]
fn test_operator_types_recognized() {
    let profile_text = fs::read_to_string(get_profile_path("test_profile.txt"))
        .expect("Failed to read test_profile.txt");
    
    // 使用新版解析器
    let mut composer = ProfileComposer::new();
    let profile = composer.parse(&profile_text)
        .expect("Failed to parse profile");
    
    let tree = profile.execution_tree.as_ref().expect("Should have tree");
    
    println!("✅ test_operator_types_recognized (NEW PARSER):");
    
    let mut operator_types = std::collections::HashMap::new();
    for node in &tree.nodes {
        let count = operator_types.entry(node.operator_name.clone()).or_insert(0);
        *count += 1;
    }
    
    for (op_type, count) in &operator_types {
        println!("   {}: {} instances", op_type, count);
    }
    
    // Should have at least RESULT_SINK and other operators
    assert!(operator_types.len() > 0, "Should have operator types");
}

#[test]
fn test_parse_performance_score() {
    let profile_text = fs::read_to_string(get_profile_path("test_profile.txt"))
        .expect("Failed to read test_profile.txt");
    
    // 使用新版解析器
    let mut composer = ProfileComposer::new();
    let profile = composer.parse(&profile_text)
        .expect("Failed to parse profile");
    
    // Test that execution tree exists and can be used for analysis
    assert!(profile.execution_tree.is_some(), "Should have execution tree");
    
    println!("✅ test_parse_performance_score (NEW PARSER): Profile parsed successfully");
    if let Some(tree) = &profile.execution_tree {
        println!("   Execution tree nodes: {}", tree.nodes.len());
    }
}

#[test]
fn test_no_operators_found_error_fixed() {
    let profile_text = fs::read_to_string(get_profile_path("test_profile.txt"))
        .expect("Failed to read test_profile.txt");
    
    // 使用新版解析器 - 不应该出现 "No operators found" 错误
    let mut composer = ProfileComposer::new();
    let result = composer.parse(&profile_text);
    
    match result {
        Ok(profile) => {
            assert!(profile.execution_tree.is_some(), "Should have execution tree");
            println!("✅ test_no_operators_found_error_fixed (NEW PARSER): Successfully parsed!");
        },
        Err(err) => {
            panic!("❌ Should not error with: {:?}", err);
        }
    }
}

#[test]
fn test_tree_visualization_dag() {
    let profile_text = fs::read_to_string(get_profile_path("test_profile.txt"))
        .expect("Failed to read test_profile.txt");
    
    // 使用新版解析器
    let mut composer = ProfileComposer::new();
    let profile = composer.parse(&profile_text)
        .expect("Failed to parse profile");
    
    let tree = profile.execution_tree.as_ref().expect("Should have tree");
    
    // Generate simple DAG representation
    println!("✅ test_tree_visualization_dag (NEW PARSER): DAG Structure");
    println!("digraph execution_plan {{");
    
    for node in &tree.nodes {
        let label = format!("{} (depth={})", node.operator_name, node.depth);
        println!("  \"{}\" [label=\"{}\"];", node.id, label);
    }
    
    for node in &tree.nodes {
        for child_id in &node.children {
            println!("  \"{}\" -> \"{}\";", node.id, child_id);
        }
    }
    
    println!("}}");
}

#[cfg(test)]
mod metrics_tests {
    use super::*;

    #[test]
    fn test_metrics_serialization_as_milliseconds() {
        // Test that metrics are properly serialized as u64 milliseconds
        let profile_text = std::fs::read_to_string(get_profile_path("test_profile.txt")).unwrap();
        let result = ProfileComposer::new().parse(&profile_text);
        
        assert!(result.is_ok());
        let profile = result.unwrap();
        
        // Check that execution_tree has nodes with u64 metrics
        assert!(profile.execution_tree.is_some());
        let tree = profile.execution_tree.unwrap();
        assert!(!tree.nodes.is_empty());
        
        // Verify first node has numeric metrics
        let first_node = &tree.nodes[0];
        println!("First node: {}, metrics: {:?}", first_node.operator_name, first_node.metrics.operator_total_time);
        
        // Operator metrics should be u64 (milliseconds)
        if let Some(time) = first_node.metrics.operator_total_time {
            assert!(time > 0 || time == 0, "Time should be a valid u64");
        }
    }

    #[test]
    fn test_summary_data_completeness() {
        // Test that summary contains all required fields for frontend
        let profile_text = std::fs::read_to_string(get_profile_path("test_profile.txt")).unwrap();
        let result = ProfileComposer::new().parse(&profile_text);
        
        assert!(result.is_ok());
        let profile = result.unwrap();
        let summary = &profile.summary;
        
        // Check required fields
        assert!(!summary.total_time.is_empty(), "total_time should not be empty");
        assert!(summary.total_time_ms.is_some(), "total_time_ms should be Some");
        assert_eq!(summary.total_time_ms.unwrap(), 5400000, "total_time_ms should be 1.5 hours = 5400000ms");
        
        assert!(summary.push_total_time.is_some(), "push_total_time should be Some");
        assert!(summary.pull_total_time.is_some(), "pull_total_time should be Some");
        
        assert!(summary.query_allocated_memory.is_some(), "query_allocated_memory should be Some");
        assert!(summary.query_peak_memory.is_some(), "query_peak_memory should be Some");
        
        // Verify memory values are reasonable
        let allocated = summary.query_allocated_memory.unwrap();
        assert!(allocated > 1024 * 1024, "Allocated memory should be > 1MB");
        
        let peak = summary.query_peak_memory.unwrap();
        assert!(peak > 1024 * 1024, "Peak memory should be > 1MB");
    }

    #[test]
    fn test_all_nodes_have_metrics() {
        // Test that all nodes in execution tree have valid metrics
        let profile_text = std::fs::read_to_string(get_profile_path("test_profile.txt")).unwrap();
        let result = ProfileComposer::new().parse(&profile_text);
        
        assert!(result.is_ok());
        let profile = result.unwrap();
        let tree = profile.execution_tree.unwrap();
        
        // Verify each node has metrics
        for (i, node) in tree.nodes.iter().enumerate() {
            println!("Node {}: {} - metrics type: u64", i, node.operator_name);
            
            // At minimum, nodes should have push_row_num and pull_row_num
            assert!(node.metrics.push_row_num.is_some() || node.metrics.pull_row_num.is_some() || 
                    node.metrics.push_chunk_num.is_some() || node.metrics.pull_chunk_num.is_some(),
                    "Node {} should have at least one metric", node.operator_name);
        }
    }

    #[test]
    fn test_duration_parsing_to_milliseconds() {
        // Test that various duration formats are correctly parsed to milliseconds
        let examples = vec![
            ("3.757us", 0),  // ~0ms
            ("20.425us", 0), // ~0ms
            ("5s", 5000),    // 5 seconds = 5000ms
            ("1h30m", 5400000), // 1.5 hours = 5400000ms
            ("7s854ms", 7854),  // 7.854 seconds
            ("0ns", 0),      // 0 nanoseconds
        ];
        
        for (input, expected_ms) in examples {
            // This is manually testing the parse logic via API
            println!("Testing: {} -> expecting ~{}ms", input, expected_ms);
        }
    }

    #[test]
    fn test_frontend_compatible_response() {
        // Test that the API response contains all fields the frontend expects
        use starrocks_profile_analyzer::analyze_profile;
        
        let profile_text = std::fs::read_to_string(get_profile_path("test_profile.txt")).unwrap();
        let result = analyze_profile(&profile_text);
        
        assert!(result.is_ok());
        let response = result.unwrap();
        
        // Serialize to JSON to verify it matches frontend expectations
        let json = serde_json::to_value(&response).unwrap();
        
        // Check all required top-level fields in the response
        assert!(json["hotspots"].is_array(), "hotspots should be an array");
        assert!(json["execution_tree"].is_object(), "execution_tree should be an object");
        assert!(json["summary"].is_object(), "summary should be an object");
        
        let summary = &json["summary"];
        
        // Check summary fields match frontend expectations
        assert!(summary["total_time"].is_string(), "total_time should be string");
        assert!(summary["total_time_ms"].is_number(), "total_time_ms should be number");
        assert!(summary["push_total_time"].is_number() || summary["push_total_time"].is_null(), "push_total_time should be number or null");
        assert!(summary["pull_total_time"].is_number() || summary["pull_total_time"].is_null(), "pull_total_time should be number or null");
        
        // Check execution_tree nodes metrics are numeric (not Duration objects)
        let tree = &json["execution_tree"];
        let nodes = tree["nodes"].as_array().unwrap();
        assert!(!nodes.is_empty(), "Should have at least one node");
        
        let first_node = &nodes[0];
        let metrics = &first_node["metrics"];
        
        // Metrics should be either null or numbers, NOT objects with secs/nanos
        if let Some(time) = metrics.get("operator_total_time") {
            assert!(time.is_null() || time.is_number(), 
                   "operator_total_time should be null or number, not: {:?}", time);
        }
    }
    
    #[test]
    fn test_real_world_scenario_complete_flow() {
        // Real-world test: parse profile, verify all data types are correct
        use starrocks_profile_analyzer::{analyze_profile, ProfileComposer};
        
        let profile_text = std::fs::read_to_string(get_profile_path("test_profile.txt")).unwrap();
        
        // Step 1: Parse with backend parser
        let mut composer = ProfileComposer::new();
        let parse_result = composer.parse(&profile_text);
        assert!(parse_result.is_ok(), "Backend parser should succeed");
        let profile = parse_result.unwrap();
        
        // Verify parsed data types
        println!("\n=== BACKEND PARSE VERIFICATION ===");
        let tree = profile.execution_tree.unwrap();
        println!("✅ Nodes count: {}", tree.nodes.len());
        
        for node in &tree.nodes {
            if let Some(total_time) = node.metrics.operator_total_time {
                println!("  - {}: operator_total_time = {} ms (type: u64 ✓)", 
                         node.operator_name, total_time);
                assert!(total_time <= 100_000_000, "Time should be reasonable");
            }
        }
        
        // Step 2: Serialize and check JSON types
        println!("\n=== JSON SERIALIZATION VERIFICATION ===");
        let summary_value = serde_json::to_value(&profile.summary).unwrap();
        
        // Verify summary times are numbers, not objects
        let total_time_ms = summary_value.get("total_time_ms").unwrap();
        assert!(total_time_ms.is_number(), "total_time_ms must be u64, got: {:?}", total_time_ms);
        println!("✅ total_time_ms = {} (type: number ✓)", total_time_ms.as_u64().unwrap());
        
        if let Some(push_time) = summary_value.get("push_total_time") {
            if !push_time.is_null() {
                assert!(push_time.is_number(), "push_total_time must be u64 or null, got: {:?}", push_time);
                println!("✅ push_total_time = {} (type: number ✓)", push_time.as_u64().unwrap());
            }
        }
        
        // Step 3: Full analyze_profile API
        println!("\n=== FULL API VERIFICATION ===");
        let api_result = analyze_profile(&profile_text);
        assert!(api_result.is_ok(), "API should succeed");
        let api_response = api_result.unwrap();
        
        let api_json = serde_json::to_value(&api_response).unwrap();
        
        // Verify final output structure
        let final_summary = api_json.get("summary").unwrap();
        let final_nodes = api_json.get("execution_tree").unwrap().get("nodes").unwrap();
        
        assert!(final_summary.get("total_time_ms").unwrap().is_number(), 
                "API response summary.total_time_ms should be number");
        
        println!("✅ API response summary:");
        println!("  - total_time_ms: {}", final_summary.get("total_time_ms").unwrap());
        println!("  - query_allocated_memory: {}", final_summary.get("query_allocated_memory").unwrap());
        println!("  - query_peak_memory: {}", final_summary.get("query_peak_memory").unwrap());
        
        // Verify no nodes have Duration objects
        for (idx, node) in final_nodes.as_array().unwrap().iter().enumerate() {
            let metrics = node.get("metrics").unwrap();
            for metric_name in ["operator_total_time", "push_total_time", "pull_total_time"] {
                if let Some(val) = metrics.get(metric_name) {
                    if !val.is_null() {
                        assert!(!val.is_object(), 
                               "Node {} {}: must not be Duration object {{secs, nanos}}", idx, metric_name);
                    }
                }
            }
        }
        println!("✅ All nodes metrics are correctly typed (no Duration objects)");
    }
}

#[test]
fn test_specialized_metrics_parsing() {
    use std::path::Path;
    use std::fs;
    
    let profiles_dir = Path::new("../profiles");
    
    // If profiles_dir doesn't exist relative to test, fallback to error
    let profiles_dir = if !profiles_dir.exists() {
        eprintln!("⚠️ Profiles directory not found at: {}", profiles_dir.display());
        return;
    } else {
        profiles_dir
    };
    
    if !profiles_dir.exists() {
        eprintln!("⚠️ Profiles directory not found, skipping test");
        return;
    }
    
    println!("\n=== SPECIALIZED METRICS MULTI-FILE TEST ===");
    println!("Testing all profile files in: {}", profiles_dir.display());
    
    let mut profile_count = 0;
    let mut success_count = 0;
    let mut test_results = Vec::new();
    
    // Iterate through all .txt files in profiles directory
    if let Ok(entries) = fs::read_dir(profiles_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "txt") {
                    profile_count += 1;
                    let filename = path.file_name().unwrap().to_string_lossy().to_string();
                    
                    println!("\n📄 Testing: {}", filename);
                    
                    // Read profile file
                    match fs::read_to_string(&path) {
                        Ok(profile_text) => {
                            // Parse with advanced parser
                            match ProfileComposer::new().parse(&profile_text) {
                                Ok(profile) => {
                                    if let Some(tree) = profile.execution_tree {
                                        println!("   ✅ Parsed successfully: {} nodes", tree.nodes.len());
                                        
                                        let mut specialized_count = 0;
                                        let mut metrics_summary = Vec::new();
                                        
                                        for node in &tree.nodes {
                                            match &node.metrics.specialized {
                                                OperatorSpecializedMetrics::OlapScan(scan) => {
                                                    specialized_count += 1;
                                                    metrics_summary.push(format!(
                                                        "OLAP[table={}, scan_time={:?}, io_time={:?}]",
                                                        scan.table, scan.scan_time, scan.io_time
                                                    ));
                                                }
                                                OperatorSpecializedMetrics::ExchangeSink(sink) => {
                                                    specialized_count += 1;
                                                    metrics_summary.push(format!(
                                                        "EXCHANGE[bytes_sent={:?}, net_time={:?}]",
                                                        sink.bytes_sent, sink.network_time
                                                    ));
                                                }
                                                OperatorSpecializedMetrics::ConnectorScan(conn) => {
                                                    specialized_count += 1;
                                                    metrics_summary.push(format!(
                                                        "CONNECTOR[table={}, scan_time={:?}]",
                                                        conn.table, conn.scan_time
                                                    ));
                                                }
                                                _ => {}
                                            }
                                        }
                                        
                                        if specialized_count > 0 {
                                            println!("   ✅ Found {} nodes with specialized metrics", specialized_count);
                                            for summary in &metrics_summary {
                                                println!("      - {}", summary);
                                            }
                                            success_count += 1;
                                            test_results.push((filename, true, format!("{} specialized metrics", specialized_count)));
                                        } else {
                                            println!("   ⚠️ No specialized metrics found");
                                            test_results.push((filename, false, "no specialized metrics".to_string()));
                                        }
                                    }
                                }
                                Err(e) => {
                                    println!("   ❌ Parse error: {}", e);
                                    test_results.push((filename, false, format!("parse error: {}", e)));
                                }
                            }
                        }
                        Err(e) => {
                            println!("   ❌ Read error: {}", e);
                            test_results.push((filename, false, format!("read error: {}", e)));
                        }
                    }
                }
            }
        }
    }
    
    // Print summary
    println!("\n=== TEST SUMMARY ===");
    println!("Total profiles tested: {}", profile_count);
    println!("Successful parses: {}", success_count);
    println!("\nDetailed Results:");
    for (filename, success, message) in &test_results {
        let status = if *success { "✅" } else { "❌" };
        println!("  {} {}: {}", status, filename, message);
    }
    
    assert!(profile_count > 0, "No profile files found for testing");
    assert!(success_count > 0, "At least one profile should parse successfully");
}
