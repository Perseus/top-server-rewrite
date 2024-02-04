use colored::Colorize;
use common_utils::{
    network::{tcp_connection::TcpConnection, tcp_server::TcpServer},
    packet::BasePacket,
};
use std::{collections::HashMap, path::Path, sync::Arc};
use tokio::{
    sync::*,
    task::{JoinHandle, JoinSet},
};

use crate::{
    client_handler::ClientHandler,
    config::{self, GateServerConfig},
    player::Player,
};

pub struct GateServer {
    config: GateServerConfig,
    client_comm_channels: Arc<Mutex<HashMap<String, mpsc::Sender<BasePacket>>>>,
    group_comm_channel: Option<mpsc::Sender<BasePacket>>,
}

impl GateServer {
    pub fn new(path: &str) -> Self {
        Self::print_gate_start();

        println!(
            "{} {}",
            "Loading GateServer config from".green(),
            path.blue().italic()
        );
        let config = config::parse_config(Path::new(path));

        GateServer {
            config,
            client_comm_channels: Arc::new(Mutex::new(HashMap::new())),
            group_comm_channel: None,
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

    async fn start_client_connection_handler(&mut self) -> JoinHandle<()> {
        let (ip, port) = self.config.get_server_ip_and_port_for_client();
        let connect_addr = format!("{}:{}", ip, port);

        let (client_comm_channel_tracker_tx, mut client_comm_channel_tracker_rx) =
            tokio::sync::mpsc::channel::<(String, mpsc::Sender<BasePacket>)>(100);

        tokio::spawn(async move {
            TcpServer::start::<ClientHandler, Player>(connect_addr, client_comm_channel_tracker_tx)
                .await
                .unwrap();
        });

        let client_comm_channels = self.client_comm_channels.clone();

        println!(
            "{} {}",
            "Ready to accept client connections at port".green(),
            port
        );

        tokio::spawn(async move {
            while let Some((id, client_comm_channel)) = client_comm_channel_tracker_rx.recv().await
            {
                println!("{} {}", "Adding client comm channel for id".green(), id);
                client_comm_channels
                    .lock()
                    .await
                    .insert(id, client_comm_channel);
            }
        })
    }

    async fn start_group_server_connection_handler(&mut self) -> JoinHandle<()> {
        let (ip, port) = self.config.get_server_ip_and_port_for_group_server();
        let connect_addr = format!("{}:{}", ip, port);
    }

    pub async fn start(&mut self) -> anyhow::Result<()> {
        let handle = self.start_client_connection_handler().await;

        handle.await?;
        Ok(())
    }

    pub fn do_something(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

mod tests {}
