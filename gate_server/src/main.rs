mod client_handler;
mod config;
mod gate_commands;
mod gate_server;
mod group_server;
mod groupserver_handler;
mod metrics;
mod player;
mod web_server;

use std::thread;

use web_server::start_webserver;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut gate_server = gate_server::GateServer::new("./config.yaml");

    thread::spawn(|| {
        start_webserver();
    });

    gate_server.start().await?;

    Ok(())
}
