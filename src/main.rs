use std::{net::SocketAddr, sync::Arc};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use rmcp::{
    transport::streamable_http_server::{
        session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
    },
    ServiceExt,
};
use tracing_subscriber::EnvFilter;

use mcp_network_proxy::{http_client::HttpClient, proxy::NetworkProxy};

#[derive(Debug, Parser)]
#[command(version, about = "MCP HTTP egress proxy", long_about = None)]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Serve over stdio (default).
    Stdio,
    /// Serve over Streamable HTTP.
    Http {
        /// Address to bind, e.g. 127.0.0.1:8080.
        #[arg(long, default_value = "127.0.0.1:8080")]
        bind: SocketAddr,
        /// HTTP path to mount the MCP service at.
        #[arg(long, default_value = "/mcp")]
        path: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let http = Arc::new(HttpClient::new()?);
    let cli = Cli::parse();

    match cli.cmd.unwrap_or(Cmd::Stdio) {
        Cmd::Stdio => run_stdio(http).await,
        Cmd::Http { bind, path } => run_http(http, bind, path).await,
    }
}

async fn run_stdio(http: Arc<HttpClient>) -> Result<()> {
    tracing::info!("starting mcp-network-proxy on stdio");
    let proxy = NetworkProxy::new(http);
    let server = proxy
        .serve((tokio::io::stdin(), tokio::io::stdout()))
        .await
        .context("starting stdio MCP server")?;
    server.waiting().await?;
    Ok(())
}

async fn run_http(http: Arc<HttpClient>, bind: SocketAddr, path: String) -> Result<()> {
    let ct = tokio_util::sync::CancellationToken::new();
    let service = StreamableHttpService::new(
        {
            let http = http.clone();
            move || Ok(NetworkProxy::new(http.clone()))
        },
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default().with_cancellation_token(ct.child_token()),
    );

    let app = axum::Router::new().nest_service(&path, service);
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .with_context(|| format!("binding to {bind}"))?;
    tracing::info!(%bind, %path, "mcp-network-proxy listening");

    axum::serve(listener, app)
        .with_graceful_shutdown({
            let ct = ct.clone();
            async move {
                let _ = tokio::signal::ctrl_c().await;
                ct.cancel();
            }
        })
        .await
        .context("axum serve")?;
    Ok(())
}

fn init_tracing() {
    // Always log to stderr — stdio mode uses stdout for the JSON-RPC stream.
    let _ = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .try_init();
}
