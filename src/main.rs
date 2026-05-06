use std::error::Error;
use std::sync::Arc;

use clap::Parser;
use rmcp::ServiceExt;
use tracing_subscriber::EnvFilter;

mod service;
mod utils;

use crate::service::agp::AGPService;
use crate::utils::clickhouse::ClickHouseClient;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long, env = "PROXY_URL")]
    url: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let args = Args::parse();

    let ch_client = ClickHouseClient::new(&args.url, None)?;
    let agp_service = Arc::new(AGPService {
        client: Arc::new(ch_client),
    });

    agp_service
        .serve((tokio::io::stdin(), tokio::io::stdout()))
        .await
        .inspect_err(|e| {
            tracing::error!("serving error: {:?}", e);
        })?
        .waiting()
        .await
        .inspect_err(|e| {
            tracing::error!("waiting error: {:?}", e);
        })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_args_parsing() {
        let args = Args::try_parse_from(&["agp-mcp", "--url", "http://localhost:8123"]).unwrap();
        assert_eq!(args.url, "http://localhost:8123");
    }

    #[test]
    fn test_args_missing_url() {
        let result = Args::try_parse_from(&["agp-mcp"]);
        assert!(result.is_err());
    }
}
