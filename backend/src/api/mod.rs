use warp::Filter;
use serde_json::json;
use serde::{Serialize, Deserialize};
use crate::static_files::StaticFiles;
use mime_guess;

#[derive(Deserialize)]
struct AnalyzeRequest {
    profile_text: String,
}

pub async fn start_server(host: String, port: u16) {
    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec!["content-type"])
        .allow_methods(vec!["GET", "POST"]);

    let analyze_profile_json = warp::path("analyze")
        .and(warp::post())
        .and(warp::body::content_length_limit(1024 * 1024 * 50))
        .and(warp::body::json())
        .and_then(handle_analyze_profile);

    use crate::constants::file_limits;
    let analyze_profile_file = warp::path("analyze-file")
        .and(warp::post())
        .and(warp::body::content_length_limit(file_limits::MAX_UPLOAD_SIZE))
        .and(warp::multipart::form().max_length(file_limits::MAX_UPLOAD_SIZE))
        .and_then(handle_analyze_profile_file);

    let health = warp::path("health")
        .and(warp::get())
        .map(|| warp::reply::json(&json!({"status": "ok"})));

    let api_routes = health
        .or(analyze_profile_json)
        .or(analyze_profile_file);

    let static_routes = warp::get()
        .and(warp::path::tail())
        .and_then(serve_static);

    let routes = api_routes
        .or(static_routes)
        .with(cors)
        .with(warp::log("api"));

    let addr: std::net::IpAddr = host.parse().unwrap_or_else(|_| {
        eprintln!("Invalid host address, using 0.0.0.0");
        std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0))
    });

    warp::serve(routes)
        .run((addr, port))
        .await;
}

#[derive(Serialize)]
struct AnalyzeResponse {
    success: bool,
    error: Option<String>,
    data: Option<crate::models::ProfileAnalysisResponse>,
}

async fn handle_analyze_profile_file(mut form: warp::multipart::FormData) -> Result<impl warp::Reply, warp::Rejection> {
    use futures::TryStreamExt;
    use bytes::Buf;
    
    let mut profile_text = String::new();
    
    while let Some(part) = form.try_next().await.map_err(|_| warp::reject::reject())? {
        if part.name() == "file" {
            let data = part.stream().try_fold(Vec::new(), |mut acc, chunk| async move {
                let chunk_bytes = chunk.chunk();
                acc.extend_from_slice(chunk_bytes);
                Ok(acc)
            }).await.map_err(|_| warp::reject::reject())?;
            
            profile_text = String::from_utf8(data).map_err(|_| warp::reject::reject())?;
            break;
        }
    }
    
    if profile_text.is_empty() {
        return Ok(warp::reply::json(&json!({
            "success": false,
            "error": "No file provided",
            "data": null
        })));
    }
    
    match crate::analyze_profile(&profile_text) {
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
                error: Some(format!("解析Profile失败: {}", err)),
                data: None,
            };
            Ok(warp::reply::json(&response))
        }
    }
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

async fn serve_static(path: warp::path::Tail) -> Result<impl warp::Reply, warp::Rejection> {
    let path_str = path.as_str();
    
    let file_path = if path_str.is_empty() || path_str == "/" {
        "index.html"
    } else {
        path_str.trim_start_matches('/')
    };
    
    tracing::debug!("Requesting static file: {}", file_path);
    
    match StaticFiles::get(file_path) {
        Some(content) => {
            let mime = mime_guess::from_path(file_path).first_or_octet_stream();
            tracing::debug!("Serving {} with mime type {}", file_path, mime.as_ref());
            
            Ok(warp::http::Response::builder()
                .header("content-type", mime.as_ref())
                .header("cache-control", "public, max-age=86400")
                .body(content.data.to_vec())
                .unwrap())
        }
        None => {
            if !file_path.contains('.') {
                tracing::debug!("SPA fallback: serving index.html for route {}", file_path);
                match StaticFiles::get("index.html") {
                    Some(content) => {
                        Ok(warp::http::Response::builder()
                            .header("content-type", "text/html")
                            .header("cache-control", "no-cache")
                            .body(content.data.to_vec())
                            .unwrap())
                    }
                    None => {
                        tracing::error!("index.html not found in embedded files!");
                        Err(warp::reject::not_found())
                    }
                }
            } else {
                tracing::warn!("Static file not found: {}", file_path);
                Err(warp::reject::not_found())
            }
        }
    }
}
