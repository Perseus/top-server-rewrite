mod config;
mod gate_server;

use std::path::Path;

use config::parse_config;

#[tokio::main]
async fn main() {
  let gate_server = gate_server::GateServer::start();
  
}