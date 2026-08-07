use anyhow::Result;

use crate::config::Config;
use crate::server::Server;

mod config;
mod server;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let config = Config::load(&args[1])?;
    dbg!(config.bind);
    dbg!(config.port);
    let server = Server::new();
    server.run().await?;
    Ok(())
}
