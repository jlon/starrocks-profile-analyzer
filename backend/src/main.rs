use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "starrocks-profile-analyzer")]
#[command(author = "StarRocks Community")]
#[command(version = "0.1.0")]
#[command(about = "StarRocks Profile Analyzer - Analyze query profiles with embedded web UI", long_about = None)]
struct Args {
    /// Server port
    #[arg(short, long, default_value = "3030")]
    port: u16,

    /// Server host
    #[arg(long, default_value = "0.0.0.0")]
    host: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("StarRocks Profile Analyzer v0.1.0");
    println!("Starting server on http://{}:{}", args.host, args.port);
    println!("Frontend: http://{}:{}", args.host, args.port);
    println!("API: http://{}:{}/health, /analyze, /analyze-file", args.host, args.port);
    println!();

    starrocks_profile_analyzer::api::start_server(args.host, args.port).await;
    Ok(())
}
