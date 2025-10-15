extern crate starrocks_profile_analyzer;

use starrocks_profile_analyzer::analyze_profile;
use std::fs;
use std::time::Instant;

fn main() {
    // 读取测试Profile文件
    let profile_text = fs::read_to_string("../test_profile.txt")
        .expect("Failed to read test profile");

    println!("开始解析StarRocks Profile...");
    let start_time = Instant::now();

    // 解析并分析Profile
    match analyze_profile(&profile_text) {
        Ok(result) => {
            let elapsed = start_time.elapsed();
            println!("✅ Profile分析成功，耗时: {:?}", elapsed);

            // 输出分析结果
            println!("\n=== 分析结果统计 ===");
            println!("🔥 发现热点数量: {}", result.hotspots.len());
