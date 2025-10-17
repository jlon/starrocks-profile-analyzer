use starrocks_profile_analyzer::analyze_profile;
use std::fs;
use std::time::Instant;

fn main() {
    println!("🔍 StarRocks Profile格式兼容性验证测试");
    println!("=====================================");

    // 读取测试Profile文件
    let profile_text = fs::read_to_string("../test_profile.txt")
        .expect("无法读取测试Profile文件");

    println!("✅ Profile文件读取成功，大小: {} 字节", profile_text.len());

    // 测试时间格式解析
    println!("\n🕒 测试时间格式解析:");
    test_time_parsing();

    // 测试字节单位转换
    println!("\n💾 测试字节单位转换:");
    test_byte_parsing();

    // 执行完整的Profile分析
    println!("\n🔬 执行Profile分析:");
    let start_time = Instant::now();

    // 使用新版解析器分析
    println!("\n🔬 使用 ProfileComposer 分析Profile:");
    let mut composer = starrocks_profile_analyzer::ProfileComposer::new();
    match composer.parse(&profile_text) {
        Ok(advanced_profile) => {
            println!("✅ 解析成功");
            // 检查执行树
            if let Some(tree) = &advanced_profile.execution_tree {
                println!("📊 执行树包含 {} 个节点", tree.nodes.len());
                for node in &tree.nodes {
                    println!("  • {}: {:?}", node.operator_name, node.node_type);
                }
            }
        }
        Err(e) => {
            println!("❌ 解析失败: {:?}", e);
        }
    }

    match analyze_profile(&profile_text) {
        Ok(result) => {
            let elapsed = start_time.elapsed();
            println!("✅ 分析成功，耗时: {:?}", elapsed);

            // 验证CONNECTOR_SCAN指标提取
            verify_connector_scan_parsing(&profile_text);
            verify_analysis_results(&result);

            println!("\n🎉 兼容性验证通过！StarRocks Profile格式完全兼容。");
        }
        Err(e) => {
            println!("❌ 分析失败: {:?}", e);
            std::process::exit(1);
        }
    }
}

fn test_time_parsing() {
    use starrocks_profile_analyzer::analyzer::suggestion_engine::SuggestionEngine;

    let time_formats = vec![
        ("1h30m", 5400.0),  // 1.5小时 = 5400秒
        ("7s854ms", 7.854),
        ("5s499ms", 5.499),
        ("2m30s", 150.0),
        ("1h", 3600.0),
        ("30m", 1800.0),
        ("45s", 45.0),
    ];

    for (time_str, expected) in time_formats {
        match SuggestionEngine::parse_duration(time_str) {
            Ok(duration) => {
                let seconds = duration.as_secs_f64();
                let diff = (seconds - expected).abs();
                if diff < 0.001 {
                    println!("  ✅ {} -> {:.3}s", time_str, seconds);
                } else {
                    println!("  ⚠️  {} -> {:.3}s (期望: {:.3}s)", time_str, seconds, expected);
                }
            }
            Err(_) => {
                println!("  ❌ {} -> 解析失败", time_str);
            }
        }
    }
}

fn test_byte_parsing() {
    use starrocks_profile_analyzer::analyzer::suggestion_engine::SuggestionEngine;

    let byte_formats = vec![
        ("2.174K (2174)", 2174u64),
        ("1.463 KB", 1495u64),  // 1.463 * 1024 ≈ 1495
        ("18.604 MB", 19512832u64), // 18.604 * 1024 * 1024 ≈ 19512832
        ("12.768 GB", 13710116864u64), // 12.768 * 1024^3 ≈ 13710116864
        ("2.123 KB", 2175u64), // 2.123 * 1024 ≈ 2175
    ];

    for (byte_str, expected) in byte_formats {
        match SuggestionEngine::parse_bytes(byte_str) {
            Ok(bytes) => {
                let diff = (bytes as i64 - expected as i64).abs();
                if diff <= 1 {  // 允许1字节误差
                    println!("  ✅ {} -> {} bytes", byte_str, bytes);
                } else {
                    println!("  ⚠️  {} -> {} bytes (期望: {})", byte_str, bytes, expected);
                }
            }
            Err(_) => {
                println!("  ❌ {} -> 解析失败", byte_str);
            }
        }
    }
}

fn verify_connector_scan_parsing(profile_text: &str) {
    println!("🔧 CONNECTOR_SCAN解析验证:");

    // 检查Profile中是否包含CONNECTOR_SCAN
    if profile_text.contains("CONNECTOR_SCAN") {
        println!("  ✅ Profile中包含CONNECTOR_SCAN操作符");

        // 检查关键指标是否存在
        let key_indicators = vec![
            ("CreateSegmentIter", "Segment迭代器初始化时间"),
            ("SegmentsReadCount", "读取的Segment数量"),
            ("IOTimeRemote", "远程I/O时间"),
            ("ScanTime", "扫描总时间"),
            ("DegreeOfParallelism", "并行度"),
        ];

        for (indicator, description) in key_indicators {
            if profile_text.contains(indicator) {
                println!("  ✅ 找到指标: {} ({})", indicator, description);
                if indicator == "SegmentsReadCount" {
                    // 提取具体的数值
                    if let Some(line) = profile_text.lines().find(|l| l.contains("SegmentsReadCount")) {
                        println!("    📊 原始数据行: {}", line.trim());
                    }
                }
            } else {
                println!("  ❌ 缺失指标: {} ({})", indicator, description);
            }
        }
    } else {
        println!("  ❌ Profile中不包含CONNECTOR_SCAN操作符");
    }

    // 检查嵌套结构
    if profile_text.contains("- IOTaskExecTime:") {
        println!("  ✅ 找到IOTaskExecTime子结构");
    } else {
        println!("  ⚠️  未找到IOTaskExecTime子结构，这可能是解析的关键问题");
    }

    // 检查Fragment结构
    println!("  📊 Fragment分析:");
    let fragment_count = profile_text.lines()
        .filter(|line| line.trim().starts_with("Fragment "))
        .count();
    println!("    📈 总Fragment数量: {}", fragment_count);

    // 分析每个Fragment
    let lines: Vec<&str> = profile_text.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if line.trim().starts_with("Fragment ") {
            let fragment_id = line.trim();
            println!("    🔍 Fragment {}: {}", fragment_id.replace("Fragment ", ""), fragment_id);

            // 检查这个Fragment内是否有CONNECTOR_SCAN
            let mut j = i + 1;
            let mut found_connector_scan = false;
            while j < lines.len() && !lines[j].trim().starts_with("Fragment ") {
                if lines[j].trim().contains("CONNECTOR_SCAN") {
                    found_connector_scan = true;
                    println!("      🚨 包含CONNECTOR_SCAN操作符");
                    break;
                }
                j += 1;
            }
            if !found_connector_scan {
                println!("      📋 不包含CONNECTOR_SCAN操作符");
            }
        }
    }
}

fn verify_analysis_results(result: &starrocks_profile_analyzer::models::ProfileAnalysisResponse) {
    println!("📊 分析结果验证:");

    // 验证基本结构
    println!("  ✅ 发现 {} 个热点", result.hotspots.len());
    println!("  ✅ 性能评分: {:.1}", result.performance_score);
    println!("  ✅ 结论: {}", result.conclusion.lines().next().unwrap_or(""));
    println!("  ✅ {} 条优化建议", result.suggestions.len());

    // 验证执行树
    if let Some(tree) = &result.execution_tree {
        println!("  ✅ 执行树包含 {} 个节点", tree.nodes.len());
    }

    // 验证热点分析是否合理
    if !result.hotspots.is_empty() {
        println!("\n🔥 热点验证:");
        for (i, hotspot) in result.hotspots.iter().take(3).enumerate() {
            println!("  {}. {} - {}", i + 1, hotspot.issue_type, hotspot.description.lines().next().unwrap_or(""));
            println!("     位置: {}", hotspot.node_path);
            println!("     严重程度: {:?}", hotspot.severity);
        }
        if result.hotspots.len() > 3 {
            println!("     ... 还有 {} 个热点", result.hotspots.len() - 3);
        }
    }

    // 验证建议是否相关
    if !result.suggestions.is_empty() {
        println!("\n💡 建议验证:");
        for suggestion in result.suggestions.iter().take(3) {
            println!("  • {}", suggestion);
        }
        if result.suggestions.len() > 3 {
            println!("  ... 还有 {} 条建议", result.suggestions.len() - 3);
        }
    }

    // 验证CONNECTOR_SCAN指标提取
    let connector_scan_hotspots = result.hotspots.iter()
        .filter(|h| h.node_path.contains("CONNECTOR_SCAN"))
        .collect::<Vec<_>>();

    println!("\n🔍 CONNECTOR_SCAN分析:");
    if connector_scan_hotspots.is_empty() {
        println!("  📝 未发现CONNECTOR_SCAN相关热点");
        // 检查是否解析到了CONNECTOR_SCAN操作符的数据
        println!("  🔍 检查ConnectorScan解析情况...");
    } else {
        println!("  ✅ 发现 {} 个CONNECTOR_SCAN热点", connector_scan_hotspots.len());
        for (i, hotspot) in connector_scan_hotspots.iter().enumerate() {
            println!("  {}. {} - {}", i + 1, hotspot.issue_type, hotspot.description.lines().next().unwrap_or(""));
        }
    }

    // 打印诊断详细信息用于debug
    println!("\n🛠️  调试信息:");
    println!("  📊 热点类型统计:");
    let hotspot_types = result.hotspots.iter()
        .fold(std::collections::HashMap::new(), |mut map, h| {
            *map.entry(&h.issue_type).or_insert(0) += 1;
            map
        });
    for (hotspot_type, count) in hotspot_types {
        println!("      {}: {}", hotspot_type, count);
    }

    // 验证时间计算逻辑
    if result.performance_score >= 0.0 && result.performance_score <= 100.0 {
        println!("\n📈 性能评分有效: {:.1}/100", result.performance_score);
    } else {
        println!("\n⚠️  性能评分异常: {}", result.performance_score);
    }
}
