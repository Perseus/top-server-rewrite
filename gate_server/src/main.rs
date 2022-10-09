mod config;
mod gate_server;

use std::path::Path;

use config::parse_config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let mut gate_server = gate_server::GateServer::new("./config.yaml");
  
    gate_server.start().await?;
    gate_server.start_accepting_connections().await?;

    Ok(())
}