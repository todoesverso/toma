use anyhow::Result;

use crate::args::Args;
use crate::config::Config;
use crate::server::Server;

mod args;
mod config;
mod handlers;
mod server;
mod service;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::new();
    let config = Config::load(args.input_file)?;
    dbg!(&config);
    let server = Server::new(config.bind.as_ref(), config.port);
    server.init_logging(args.debug);
    server.run(config.services).await?;
    Ok(())
}
