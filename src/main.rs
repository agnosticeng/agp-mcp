//! `agp-mcp` is a Model Context Protocol (MCP) server for ClickHouse.
//!
//! This crate provides a dual-transport server (stdio and HTTP) that allows
//! AI models to interact with ClickHouse databases via a secure proxy.

use std::env;

use clap::Parser;
use rmcp::{
    ServiceExt,
    transport::streamable_http_server::{
        StreamableHttpService, session::local::LocalSessionManager,
    },
};
use tracing_subscriber::EnvFilter;

mod service;
mod utils;

use crate::service::agp::AGPService;
use crate::utils::clickhouse::ClickHouseClient;

/// CLI arguments for the `agp-mcp` server.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The URL of the ClickHouse AGP proxy.
    #[arg(long, env = "PROXY_URL")]
    url: String,

    /// Whether to run in HTTP transport mode (default is stdio).
    #[arg(long)]
    http: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let args = Args::parse();
    let ch_client = ClickHouseClient::new(&args.url, None)?;

    if args.http {
        run_streamable_http(ch_client).await
    } else {
        run_stdio(ch_client).await
    }
}

async fn run_stdio(ch_client: ClickHouseClient) -> anyhow::Result<()> {
    let mcp = AGPService::new(ch_client);
    let transport = (tokio::io::stdin(), tokio::io::stdout());

    let service = mcp.serve(transport).await.inspect_err(|e| {
        tracing::error!("serving error: {:?}", e);
    })?;

    service.waiting().await.inspect_err(|e| {
        tracing::error!("waiting error: {:?}", e);
    })?;

    Ok(())
}

async fn run_streamable_http(ch_client: ClickHouseClient) -> anyhow::Result<()> {
    tracing::info!("Running Streamable HTTP server");

    let service = StreamableHttpService::new(
        move || Ok(AGPService::new(ch_client.clone())),
        LocalSessionManager::default().into(),
        Default::default(),
    );

    let bind = env::var("HTTP_BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1:8001".into());

    let cors = tower_http::cors::CorsLayer::permissive();
    let router = axum::Router::new()
        .nest_service("/mcp", service)
        .layer(cors);
    let tcp_listener = tokio::net::TcpListener::bind(&bind).await?;

    tracing::info!("MCP server started at http://{}/mcp", bind);
    tracing::info!("Press Ctrl+C to shutdown");

    axum::serve(tcp_listener, router)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to install Ctrl+C handler");
        })
        .await?;

    Ok(())
}
