pub mod parser;
pub mod models;
pub mod analyzer;
pub mod api;

pub use models::*;
pub use parser::*;
pub use analyzer::*;

use analyzer::hotspot_detector::HotSpotDetector;
use analyzer::suggestion_engine::SuggestionEngine;
use parser::advanced_parser::AdvancedStarRocksProfileParser;

pub fn analyze_profile(profile_text: &str) -> Result<AnalysisResult, String> {
    // 1. 使用高级解析器解析Profile文本
    let profile = AdvancedStarRocksProfileParser::parse_advanced(profile_text)
        .map_err(|e| format!("解析Profile失败: {}", e))?;

    // 2. 检测热点
    let hotspots = HotSpotDetector::analyze(&profile);

    // 3. 生成结论
    let conclusion = SuggestionEngine::generate_conclusion(&hotspots, &profile);

    // 4. 生成建议
    let suggestions = SuggestionEngine::generate_suggestions(&hotspots);

    // 5. 计算性能评分
    let performance_score = SuggestionEngine::calculate_performance_score(&hotspots, &profile);

    Ok(AnalysisResult {
        hotspots,
        conclusion,
        suggestions,
        performance_score,
    })
}
