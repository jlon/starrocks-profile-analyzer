use starrocks_profile_analyzer::parser::core::fragment_parser::FragmentParser;
use std::fs;
use std::path::Path;

#[test]
fn test_operator_has_common_metrics() {
    let profile_path = Path::new("../profiles/profile1.txt").to_string_lossy().to_string();
    let content = fs::read_to_string(profile_path)
        .expect("Failed to read profile file");
    
    let fragments = FragmentParser::extract_all_fragments(&content);
    
    println!("Total fragments: {}", fragments.len());
    
    for (i, fragment) in fragments.iter().enumerate() {
        println!("\nFragment {}:", i);
        println!("  Fragment ID: {}", fragment.id);
        println!("  Pipelines: {}", fragment.pipelines.len());
        
        for (p_idx, pipeline) in fragment.pipelines.iter().enumerate() {
            println!("    Pipeline {}: {} operators", p_idx, pipeline.operators.len());
            
            for (o_idx, operator) in pipeline.operators.iter().enumerate() {
                println!("      Operator {}: {}", o_idx, operator.name);
                println!("        Plan node ID: {:?}", operator.plan_node_id);
                println!("        Common metrics: {} entries", operator.common_metrics.len());
                println!("        Unique metrics: {} entries", operator.unique_metrics.len());
                
                if !operator.unique_metrics.is_empty() {
                    println!("        Unique metrics:");
                    for (key, value) in operator.unique_metrics.iter().take(5) {
                        println!("          {} = {}", key, value);
                    }
                }
            }
        }
    }
    
    // Assert that at least one operator has common metrics
    let has_metrics = fragments.iter()
        .flat_map(|f| &f.pipelines)
        .flat_map(|p| &p.operators)
        .any(|op| !op.common_metrics.is_empty());
    
    assert!(has_metrics, "At least one operator should have common_metrics");
}
