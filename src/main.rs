use anyhow::Result;

use crate::args::Args;
use crate::config::Config;
use crate::server::Server;

mod args;
mod config;
mod server;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::new();
    let config = Config::load(args.input_file)?;
    let server = Server::new(config.bind.as_ref(), config.port);
    server.init_logging();
    server.run().await?;
    Ok(())
}
