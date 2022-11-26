mod config;
mod gate_server;
mod web_server;
mod metrics;

use std::{path::Path, thread};

use config::parse_config;
use web_server::start_webserver;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let mut gate_server = gate_server::GateServer::new("./config.yaml");

    thread::spawn(|| {
      start_webserver();
    });
  
    gate_server.start().await?;
    gate_server.start_accepting_connections().await?;

    Ok(())
}