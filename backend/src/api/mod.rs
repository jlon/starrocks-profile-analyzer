use warp::Filter;
use serde_json::json;

pub async fn start_server() {
    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec!["content-type"])
        .allow_methods(vec!["GET", "POST"]);

    let analyze_profile = warp::path("analyze")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(handle_analyze_profile);

    let health = warp::path("health")
        .and(warp::get())
        .map(|| warp::reply::json(&json!({"status": "ok"})));

    let routes = health.or(analyze_profile).with(cors).with(warp::log("api"));

    println!("Starting server on http://localhost:3030");
    warp::serve(routes)
        .run(([127, 0, 0, 1], 3030))
        .await;
}

use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct AnalyzeRequest {
    profile_text: String,
}

#[derive(Serialize)]
struct AnalyzeResponse {
    success: bool,
    error: Option<String>,
    data: Option<crate::models::AnalysisResult>,
}

async fn handle_analyze_profile(req: AnalyzeRequest) -> Result<impl warp::Reply, warp::Rejection> {
    match crate::analyze_profile(&req.profile_text) {
        Ok(result) => {
            let response = AnalyzeResponse {
                success: true,
                error: None,
                data: Some(result),
            };
            Ok(warp::reply::json(&response))
        }
        Err(err) => {
            let response = AnalyzeResponse {
                success: false,
                error: Some(err),
                data: None,
            };
            Ok(warp::reply::json(&response))
        }
    }
}
