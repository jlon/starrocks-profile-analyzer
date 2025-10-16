pub mod parser;
pub mod models;
pub mod analyzer;
pub mod api;

pub use models::*;
pub use parser::*;
pub use analyzer::hotspot_detector::HotSpotDetector;
pub use analyzer::suggestion_engine::SuggestionEngine;
pub use parser::advanced_parser::AdvancedStarRocksProfileParser;
pub use parser::starrocks::StarRocksProfileParser;

pub fn analyze_profile(profile_text: &str) -> Result<ProfileAnalysisResponse, String> {
    let profile = AdvancedStarRocksProfileParser::parse_advanced(profile_text)
        .map_err(|e| format!("解析Profile失败: {}", e))?;

    let hotspots = HotSpotDetector::analyze(&profile);
    let conclusion = SuggestionEngine::generate_conclusion(&hotspots, &profile);
    let suggestions = SuggestionEngine::generate_suggestions(&hotspots);
    let performance_score = SuggestionEngine::calculate_performance_score(&hotspots, &profile);
    let execution_tree = profile.execution_tree.clone();
    let summary = Some(profile.summary.clone());

    Ok(ProfileAnalysisResponse {
        hotspots,
        conclusion,
        suggestions,
        performance_score,
        execution_tree,
        summary,
    })
}
