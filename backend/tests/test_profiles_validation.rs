use std::fs;
use std::path::PathBuf;

use starrocks_profile_analyzer::parser::ProfileComposer;
use starrocks_profile_analyzer::parser::core::fragment_parser::FragmentParser;
use starrocks_profile_analyzer::parser::core::topology_parser::TopologyParser;

fn get_profiles_dir() -> PathBuf {
    // backend crate dir -> repo root -> profiles
    let backend_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    backend_dir.parent().unwrap().join("profiles")
}

fn normalize_op_name(name: &str) -> String {
    // Extract the first token up to whitespace or '(' as the base operator name
    let trimmed = name.trim();
    let mut chars = trimmed.chars();
    let mut base = String::new();
    while let Some(c) = chars.next() {
        if c.is_whitespace() || c == '(' {
            break;
        }
        base.push(c);
    }
    base
}

fn is_final_sink_name(name: &str) -> bool {
    let base = normalize_op_name(name).to_ascii_uppercase();
    if !base.ends_with("_SINK") {
        return false;
    }
    let excluded = ["EXCHANGE_SINK", "LOCAL_EXCHANGE_SINK", "MULTI_CAST", "MULTICAST_SINK", "MULTICAST"];
    !excluded.iter().any(|e| base.contains(e))
}

fn collect_final_sink_candidates(profile_text: &str) -> Vec<String> {
    let mut candidates = Vec::new();
    for line in profile_text.lines() {
        let l = line.trim();
        if l.is_empty() { continue; }
        // Heuristic: lines that look like operator headers
        if l.contains("plan_node_id") || l.ends_with("_SINK") || l.contains("_SINK ") {
            let base = normalize_op_name(l);
            if is_final_sink_name(&base) {
                candidates.push(base);
            }
        }
    }
    // Deduplicate
    candidates.sort();
    candidates.dedup();
    candidates
}

#[test]
fn validate_all_profiles_general_logic() {
    let profiles_dir = get_profiles_dir();
    assert!(profiles_dir.exists(), "profiles directory should exist: {:?}", profiles_dir);

    let mut composer = ProfileComposer::new();

    let mut total = 0usize;
    let mut passed_root_sink = 0usize;
    let mut passed_children_order = 0usize;
    let mut passed_depth = 0usize;
    let mut passed_topology_edges = 0usize;

    for entry in fs::read_dir(&profiles_dir).expect("read_dir profiles failed") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.extension().map(|e| e == "txt").unwrap_or(false) {
            total += 1;
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            println!("\n===== Validating {} =====", name);

            let text = fs::read_to_string(&path).expect("read profile text");
            let profile = composer.parse(&text).expect("parse profile");
            let tree = profile.execution_tree.as_ref().expect("execution tree");

            println!("   Root operator: {}", tree.root.operator_name);
            println!("   Root plan_node_id: {:?}", tree.root.plan_node_id);

            // 1) Final sink root logic
            let sink_candidates = collect_final_sink_candidates(&text);
            println!("   Final sink candidates: {:?}", sink_candidates);
            let root_name_upper = normalize_op_name(&tree.root.operator_name).to_ascii_uppercase();
            let is_root_sink = root_name_upper.ends_with("_SINK");
            let is_root_final_sink = is_final_sink_name(&tree.root.operator_name);
            if !sink_candidates.is_empty() {
                println!("   is_root_sink: {}  is_root_final_sink: {}", is_root_sink, is_root_final_sink);
                assert!(is_root_sink, "Root should be a _SINK when final sinks exist");
                assert!(is_root_final_sink, "Root should be a final sink (not exchange/local/multicast)");
                passed_root_sink += 1;
            } else {
                // 如果没有 final sink 候选，回退为 topology root
                let fragments = FragmentParser::extract_all_fragments(&text);
                let topo_json = profile.execution.topology.clone();
                if !topo_json.is_empty() {
                    let topo = TopologyParser::parse_with_fragments(&topo_json, &text, &fragments)
                        .expect("parse topology json");
                    // 找到 topology root 对应的树节点（按 plan_node_id）
                    let topo_root_id = topo.root_id;
                    let topo_root_node_id_opt = tree.nodes.iter()
                        .find(|n| n.plan_node_id == Some(topo_root_id))
                        .map(|n| n.id.clone());
                    if let Some(root_node_id) = topo_root_node_id_opt {
                        assert_eq!(tree.root.id, root_node_id, "Root should equal topology root when no final sink");
                    }
                }
            }

            // 2) 树结构顺序（children 顺序与 Topology 一致）
            let fragments = FragmentParser::extract_all_fragments(&text);
            let topo_json = profile.execution.topology.clone();
            if !topo_json.is_empty() {
                if let Ok(topo) = TopologyParser::parse_with_fragments(&topo_json, &text, &fragments) {
                    // 构建 plan_node_id -> tree_node_id 映射
                    let mut id_map = std::collections::HashMap::new();
                    for node in &tree.nodes {
                        if let Some(pid) = node.plan_node_id {
                            id_map.insert(pid, node.id.clone());
                        }
                    }
                    // 识别树中的 final sink 节点（用于校验时忽略）
                    use std::collections::HashSet;
                    let sink_ids: HashSet<String> = tree.nodes.iter()
                        .filter(|n| is_final_sink_name(&n.operator_name))
                        .map(|n| n.id.clone())
                        .collect();

                    // 如果有 final sink，校验 root 的第一个孩子应该是原始 Topology root
                    if !sink_candidates.is_empty() {
                        let topo_root_id = topo.root_id;
                        if let Some(topo_root_tree_id) = id_map.get(&topo_root_id) {
                            assert!(tree.root.children.contains(topo_root_tree_id), "With final sink, root should point to original Topology root");
                        }
                    }

                    // 验证每个 topology 节点的 children 顺序在树中得以保持（忽略 sink 节点）
                    let mut order_ok = true;
                    for topo_node in &topo.nodes {
                        if let Some(parent_tree_id) = id_map.get(&topo_node.id) {
                            // 期望的 children 顺序（转换为树节点 id，忽略 sink）
                            let expected_children: Vec<String> = topo_node.children.iter()
                                .filter_map(|cid| id_map.get(cid).cloned())
                                .filter(|cid| !sink_ids.contains(cid))
                                .collect();
                            // 树中实际的 children 顺序（忽略 sink）
                            if let Some(parent_node) = tree.nodes.iter().find(|n| &n.id == parent_tree_id) {
                                let actual = &parent_node.children;
                                let filtered_actual: Vec<String> = actual.iter()
                                    .filter(|aid| expected_children.contains(*aid))
                                    .cloned()
                                    .collect();
                                if filtered_actual != expected_children {
                                    order_ok = false;
                                    println!("   ⚠️ Children order mismatch for plan_node_id={}: expected={:?} actual={:?}", topo_node.id, expected_children, filtered_actual);
                                }
                            }
                        }
                    }
                    assert!(order_ok, "Children order should follow Topology definition");
                    passed_children_order += 1;

                    // 3) 拓扑边关系存在性（父子连线，忽略 sink 边）
                    let mut edges_ok = true;
                    for topo_node in &topo.nodes {
                        if let Some(parent_tree_id) = id_map.get(&topo_node.id) {
                            if let Some(parent_node) = tree.nodes.iter().find(|n| &n.id == parent_tree_id) {
                                for cid in &topo_node.children {
                                    if let Some(child_tree_id) = id_map.get(cid) {
                                        if sink_ids.contains(child_tree_id) { continue; }
                                        if !parent_node.children.contains(child_tree_id) {
                                            edges_ok = false;
                                            println!("   ❌ Missing edge: parent plan_node_id={} -> child plan_node_id={}", topo_node.id, cid);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    assert!(edges_ok, "All Topology edges should be present in the tree (excluding sink elevation)");
                    passed_topology_edges += 1;

                    // 4) 深度正确性
                    let mut depth_ok = true;
                    // 根深度为 0
                    if tree.root.depth != 0 { depth_ok = false; println!("   ❌ Root depth is not 0"); }
                    // 每条边满足 child.depth = parent.depth + 1
                    for parent in &tree.nodes {
                        for child_id in &parent.children {
                            if let Some(child) = tree.nodes.iter().find(|n| &n.id == child_id) {
                                if child.depth != parent.depth + 1 {
                                    depth_ok = false;
                                    println!("   ❌ Depth mismatch: parent {} (d={}) -> child {} (d={})", parent.id, parent.depth, child.id, child.depth);
                                }
                            }
                        }
                    }
                    assert!(depth_ok, "Depth calculation should be consistent (BFS from root)");
                    passed_depth += 1;
                }
            }
        }
    }

    println!("\n==== Summary ====");
    println!("Profiles: {}", total);
    println!("Root final sink checks passed: {}", passed_root_sink);
    println!("Topology children order checks passed: {}", passed_children_order);
    println!("Topology edges checks passed: {}", passed_topology_edges);
    println!("Depth checks passed: {}", passed_depth);
}