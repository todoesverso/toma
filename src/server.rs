use anyhow::{Context, Result};
use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::{debug, error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::handlers::RequestHandler;

pub struct Server {
    bind: String,
    port: u16,
}

impl Server {
    pub fn new(bind: &str, port: u16) -> Self {
        Self {
            bind: bind.to_string(),
            port,
        }
    }

    pub fn init_logging(&self, debug: bool) {
        // Accept log level as parameter
        let level = if debug {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        };
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| format!("toma={}", level).into()),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();
    }

    pub async fn run(self) -> Result<()> {
        let addr: SocketAddr = format!("{}:{}", self.bind, self.port).parse()?;

        info!("Starting server on http://{}", addr);

        let listener = TcpListener::bind(addr)
            .await
            .context(format!("Failed to bind to socket {addr}"))?;

        loop {
            let (stream, remote_addr) = listener
                .accept()
                .await
                .context("Failed to accept connection")?;

            debug!("Accepted connection from {}", remote_addr);
            let io = TokioIo::new(stream);

            tokio::task::spawn(async move {
                let rh = RequestHandler::new();
                if let Err(err) = http1::Builder::new().serve_connection(io, rh).await {
                    error!("Error serving connection: {:?}", err);
                }
            });
        }
    }
}
