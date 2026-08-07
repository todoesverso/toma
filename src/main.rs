use anyhow::Result;

use crate::server::Server;

mod server;

#[tokio::main]
async fn main() -> Result<()> {
    let server = Server::new();
    server.run().await?;
    Ok(())
}
