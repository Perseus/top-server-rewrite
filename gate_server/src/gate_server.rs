use std::path::Path;
use colored::Colorize;
use tokio::{net::TcpListener, io::AsyncReadExt, io::AsyncWriteExt};

use crate::config::{GateServerConfig, self};

pub struct GateServer {
  config: GateServerConfig,
  server: Option<TcpListener>
}

impl GateServer {
  pub fn new(path: &str) -> Self {
    Self::print_gate_start();

    println!("{} {}", "Loading GateServer config from".green(), path.blue().italic());
    let config = config::parse_config(Path::new(path));

    GateServer {
      config,
      server: None
    }
  }

  fn print_gate_start() {
    println!("{}", r"------------------------------------------------------------------------------------------------------------------------".blue());
    println!("{}", r"
     _______      ___   .___________. _______         _______. _______ .______     ____    ____  _______ .______      
    /  _____|    /   \  |           ||   ____|       /       ||   ____||   _  \    \   \  /   / |   ____||   _  \     
    |  |  __     /  ^  \ `---|  |----`|  |__         |   (----`|  |__   |  |_)  |    \   \/   /  |  |__   |  |_)  |    
    |  | |_ |   /  /_\  \    |  |     |   __|         \   \    |   __|  |      /      \      /   |   __|  |      /     
    |  |__| |  /  _____  \   |  |     |  |____    .----)   |   |  |____ |  |\  \----.  \    /    |  |____ |  |\  \----.
    \______|  /__/     \__\  |__|     |_______|   |_______/    |_______|| _| `._____|   \__/     |_______|| _| `._____|
    ".blue());
    println!("{}", r"                                                                                                                  v0.1.0".red());
    println!("{}", r"                                                                                                              by Perseus".red());
    println!("{}", r"------------------------------------------------------------------------------------------------------------------------".blue());
  }

  pub async fn start(&mut self) -> anyhow::Result<()> {
    let (ip, port) = self.config.get_server_ip_and_port();
    let connect_addr = format!("{}:{}", ip, port);

    let server = match TcpListener::bind(&connect_addr).await {
      Ok(listener) => listener,
      Err(err) => panic!("Unable to start the server at {}, error - {:?}", connect_addr, err),
    };

    self.server = Some(server);
    Ok(())
  }

  pub async fn start_accepting_connections(self) -> anyhow::Result<()> {
    let server = self.server.unwrap();

    loop {
      let (mut socket, _) = server.accept().await?;
    }

  }


}


mod tests {
  #[tokio::test]
  async fn it_should_start_a_tcp_server_at_the_given_port() {
    let mut gate_server = super::GateServer::new("./tests/test_gate_config.yaml");
    let (gate_server_ip, gate_server_port) = gate_server.config.get_server_ip_and_port();
    gate_server.start().await.unwrap();

    tokio::spawn(async move {
      gate_server.start_accepting_connections().await.unwrap();
    });

    let connect_addr = format!("{}:{}", gate_server_ip, gate_server_port);

    match tokio::net::TcpStream::connect(connect_addr).await {
      Ok(conn) => conn,
      Err(err) => panic!("Unable to connect to the TCP stream, error - {:?}", err)
    };

  }
}