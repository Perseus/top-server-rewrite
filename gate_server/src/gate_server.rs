use colored::Colorize;
use common_utils::{
    network::{
        connection_handler::*, tcp_client::TcpClient, tcp_connection::TcpConnection, tcp_server::*,
    },
    packet::{init::InitGroupPacket, BasePacket, PacketReader, TypedPacket},
};
use std::{
    collections::HashMap,
    path::Path,
    sync::{atomic::AtomicU32, Arc},
    time::Duration,
};
use tokio::{
    sync::*,
    task::{JoinHandle, JoinSet},
    time::sleep,
};

use crate::{
    client_handler::ClientHandler,
    config::{self, GateServerConfig},
    group_server::GroupServer,
    groupserver_handler::{GateGroupCommands, GroupServerHandler},
    player::Player,
};

/**
 * This maps an ever-growing counter that acts as the player's internal (in-memory) ID to the
 * TcpConnection that is established with the player.
 *
 * This is a RwLock because whenever some other service (Group, Game) needs to send a message to
 * a player, it will send this ID in the message. If we have this in a Mutex, we will essentially be
 * slowing down messages going to all players because we will be locking the entire map.
 *
 * Instead, the only time this gets "write"-locked, is when a new player connects and we need to
 * add their entry to the map.
 */

pub type PlayerConnection = Arc<Mutex<TcpConnection<Player>>>;
pub type PlayerConnectionMap = Arc<RwLock<HashMap<u32, PlayerConnection>>>;

pub struct GateServer {
    config: GateServerConfig,
    client_connections: PlayerConnectionMap,

    // Atomic counter to give unique IDs to players. We need this to be a U32 because
    // other services store U32 as the player's "gate address" which is required to communicate to
    // specific players directly
    player_num_counter: Arc<AtomicU32>,
    group_conn: Option<Arc<Mutex<TcpConnection<GroupServer>>>>,
}

impl GateServer {
    pub fn new(path: &str) -> Self {
        Self::print_gate_start();

        println!(
            "{} {}",
            "Loading GateServer config from".green(),
            path.green().underline(),
        );
        let config = config::parse_config(Path::new(path));

        GateServer {
            config,
            player_num_counter: Arc::new(AtomicU32::new(0)),
            client_connections: Arc::new(RwLock::new(HashMap::new())),
            group_conn: None,
        }
    }

    fn print_gate_start() {
        println!("{}", r"------------------------------------------------------------------------------------------------------------------------".green());
        println!("{}", r"
     _______      ___   .___________. _______         _______. _______ .______     ____    ____  _______ .______      
    /  _____|    /   \  |           ||   ____|       /       ||   ____||   _  \    \   \  /   / |   ____||   _  \     
    |  |  __     /  ^  \ `---|  |----`|  |__         |   (----`|  |__   |  |_)  |    \   \/   /  |  |__   |  |_)  |    
    |  | |_ |   /  /_\  \    |  |     |   __|         \   \    |   __|  |      /      \      /   |   __|  |      /     
    |  |__| |  /  _____  \   |  |     |  |____    .----)   |   |  |____ |  |\  \----.  \    /    |  |____ |  |\  \----.
    \______|  /__/     \__\  |__|     |_______|   |_______/    |_______|| _| `._____|   \__/     |_______|| _| `._____|
    ".green());
        println!("{}", r"                                                                                                                  v0.1.0".green());
        println!("{}", r"                                                                                                              by Perseus".green());
        println!("{}", r"------------------------------------------------------------------------------------------------------------------------".green());
    }

    async fn register_client_connection(
        client_conn_map: PlayerConnectionMap,
        player_counter: Arc<AtomicU32>,
        connection: TcpConnection<Player>,
    ) {
        let mut map = client_conn_map.write().await;
        let id = player_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // TODO: handle the cases where
        // 1. the counter overflows (unlikely to happen in normal conditions, but if we get a DoS, it can happen)
        let inserted = map.insert(id.clone(), Arc::new(Mutex::new(connection)));

        let connection = map.get(&id).unwrap();

        if let Ok(()) = ClientHandler::on_connected(connection.clone()) {
            println!(
                "{} {}",
                "Client connection registered with id".green(),
                id.to_string().green().underline(),
            );
        } else {
            println!(
                "{} {}",
                "Failed to register client connection with id".red(),
                id.to_string().red().underline(),
            );
        }
    }

    async fn start_client_connection_handler(&mut self) -> JoinHandle<()> {
        let (ip, port) = self.config.get_server_ip_and_port_for_client();
        let connect_addr = format!("{}:{}", ip, port);

        let (client_comm_channel_tracker_tx, mut client_comm_channel_tracker_rx) =
            tokio::sync::mpsc::channel::<(String, TcpConnection<Player>)>(100);

        tokio::spawn(async move {
            TcpServer::start::<ClientHandler, Player>(connect_addr, client_comm_channel_tracker_tx)
                .await
                .unwrap();
        });

        let client_tcp_connections = self.client_connections.clone();

        println!(
            "{} {}",
            "Ready to accept client connections at port".green(),
            port.to_string().green().underline(),
        );

        let player_counter = self.player_num_counter.clone();

        tokio::spawn(async move {
            let player_counter = player_counter;
            while let Some((_, player_connection)) = client_comm_channel_tracker_rx.recv().await {
                GateServer::register_client_connection(
                    client_tcp_connections.clone(),
                    player_counter.clone(),
                    player_connection,
                )
                .await;
            }
        })
    }

    async fn start_group_server_connection_handler(&mut self) {
        let (ip, port) = self.config.get_server_ip_and_port_for_group_server();
        let connect_addr = format!("{}:{}", ip, port);

        let mut group_server_client = TcpClient::new("GroupServer".to_string(), ip, port);
        if let Ok(group_conn) = group_server_client
            .connect::<GroupServerHandler, GroupServer, GateGroupCommands>(5)
            .await
        {
            self.group_conn = Some(Arc::new(Mutex::new(group_conn)));
            let player_list = self.client_connections.read().await;
            let group_conn = self.group_conn.clone().unwrap();
            if let Err(err) = GroupServerHandler::on_connected(group_conn.clone()) {
                println!(
                    "{} {}",
                    "Failed to start group server connection handler".red(),
                    err.to_string().red().underline(),
                );
                panic!()
            } else {
                println!("{}", "Connected to group server".green());
            }

            GroupServerHandler::sync_player_list(
                group_conn.clone(),
                player_list,
                "GateServer".to_string(),
            )
            .await;
        } else {
            println!(
                "{} {}",
                "Unable to connect to group server at".red(),
                connect_addr.red(),
            );

            sleep(tokio::time::Duration::from_secs(5)).await;
            panic!()
        }
    }

    pub async fn start(&mut self) -> anyhow::Result<()> {
        let client_connection_handle = self.start_client_connection_handler().await;
        self.start_group_server_connection_handler().await;

        client_connection_handle.await?;
        Ok(())
    }
}

mod tests {}
