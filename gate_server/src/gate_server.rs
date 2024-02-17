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
    net::TcpStream,
    sync::*,
    task::{JoinHandle, JoinSet},
    time::sleep,
};

use crate::{
    client_handler::{ClientGateCommands, ClientHandler},
    config::{self, GateServerConfig},
    group_server::GroupServer,
    groupserver_handler::{GateGroupCommands, GroupServerHandler},
    player::Player,
};

// TODO: figure out how to set this from config at runtime
pub const SUPPORTED_CLIENT_VERSION: u16 = 136;

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
    group_handler: Option<Arc<RwLock<GroupServerHandler>>>,
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
            player_num_counter: Arc::new(AtomicU32::new(1)),
            client_connections: Arc::new(RwLock::new(HashMap::new())),
            group_handler: None,
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

    /**
     * Register a new client connection with the GateServer
     */
    async fn register_client_connection(
        client_conn_map: PlayerConnectionMap,
        player_counter: Arc<AtomicU32>,
        stream: TcpStream,
        groupserver_handler: Arc<RwLock<GroupServerHandler>>,
        client_to_gate_tx: mpsc::Sender<ClientGateCommands>,
    ) {
        let mut map = client_conn_map.write().await;
        // relaxed ordering is fine, this is just to give a unique ID to the player
        // there aren't any other
        let id = player_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let socket_addr = stream.peer_addr().unwrap();
        let handler = ClientHandler::on_connect(id, "Client".to_string(), stream, socket_addr);

        {
            let handler = handler.clone();
            let mut handler = handler.write().await;
            handler.set_groupserver_handler_ref(groupserver_handler);
        }

        let connection = handler.read().await.get_connection().clone();

        // TODO: handle the cases where
        // 1. the counter overflows (unlikely to happen in normal conditions, but if we get a DoS, it can happen)
        let inserted = map.insert(id.clone(), connection);
        let connection = map.get(&id).unwrap();

        if let Ok(()) = ClientHandler::on_connected(handler, client_to_gate_tx).await {
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

    /**
     * Start a TCP server which listens for incoming connections from
     * game clients
     */
    async fn start_client_connection_handler(&mut self) -> JoinHandle<()> {
        let (ip, port) = self.config.get_server_ip_and_port_for_client();
        let connect_addr = format!("{}:{}", ip, port);

        /*
         * This channel will allow us to receive the TcpStreams that get created when an incoming
         * connection from the client is established.
         *
         * We want to "own" the TcpStreams/connections as part of the GateServer, and since we are
         * offloading the handling of the connections to a separate task, we need a way to get this data back
         *
         * I would've liked to use something like a generator here, but I don't think that's possible
         * in the way that things are currently done.
         *
         * Another way is to use stream iterators, but from the Tokio docs, the way to do it is pretty much
         * the same as what I'm doing here, so I'm not sure if it's worth it.
         */
        let (tcp_stream_tx, mut tcp_stream_rx) = tokio::sync::mpsc::channel::<TcpStream>(100);

        /*
         * Start the TCP server, hand it off to a separate task
         */
        tokio::spawn(async move {
            TcpServer::start(connect_addr, tcp_stream_tx).await.unwrap();
        });

        let client_tcp_connections = self.client_connections.clone();

        println!(
            "{} {}",
            "Ready to accept client connections at port".green(),
            port.to_string().green().underline(),
        );

        let player_counter = self.player_num_counter.clone();
        let groupserver_handler = self.group_handler.clone().unwrap();
        let (client_to_gate_tx, mut client_to_gate_rx) = mpsc::channel::<ClientGateCommands>(100);
        let client_tcp_connections_for_client_commands_handler = client_tcp_connections.clone();

        tokio::spawn(async move {
            let client_tcp_connections = client_tcp_connections_for_client_commands_handler;
            while let Some(command) = client_to_gate_rx.recv().await {
                match command {
                    ClientGateCommands::Disconnect(id) => {
                        let mut map = client_tcp_connections.write().await;
                        if let Some(connection) = map.remove(&id) {
                            let mut connection = connection.lock().await;
                            connection.close(Duration::ZERO).await;
                        }
                    }
                }
            }
        });
        /*
         * The only thing that needs to be mutated when a new client conencts
         * is the client_connections map, so we wrap it in a Arc<RwLock<>> to allow
         * for concurrent access and then pass it to this task, which can call
         * a static method on GateServer so that the logic remains in the same module
         */
        tokio::spawn(async move {
            let player_counter = player_counter;
            let groupserver_handler = groupserver_handler;
            while let Some(stream) = tcp_stream_rx.recv().await {
                GateServer::register_client_connection(
                    client_tcp_connections.clone(),
                    player_counter.clone(),
                    stream,
                    groupserver_handler.clone(),
                    client_to_gate_tx.clone(),
                )
                .await;
            }
        })
    }

    /**
     * Establishes a connection with the GroupServer
     */
    async fn start_group_server_connection_handler(&mut self) {
        let (ip, port) = self.config.get_server_ip_and_port_for_group_server();
        let connect_addr = format!("{}:{}", ip, port);

        let mut group_server_client = TcpClient::new(1, "GroupServer".to_string(), ip, port);
        let (group_to_gate_tx, group_to_gate_rx) = mpsc::channel::<GateGroupCommands>(100);
        if let Ok(group_handler) = group_server_client
            .connect::<GroupServerHandler, GroupServer, GateGroupCommands>(5)
            .await
        {
            self.group_handler = Some(group_handler);
            let player_list = self.client_connections.read().await;
            if let Err(err) = GroupServerHandler::on_connected(
                self.group_handler.clone().unwrap(),
                group_to_gate_tx,
            )
            .await
            {
                println!(
                    "{} {}",
                    "Failed to start group server connection handler".red(),
                    err.to_string().red().underline(),
                );
                panic!()
            } else {
                println!("{}", "Connected to group server".green());
            }

            self.group_handler
                .clone()
                .unwrap()
                .read()
                .await
                .sync_player_list(player_list, "GateServer".to_string())
                .await;

            // TODO: handle data coming in from groupserver on `group_to_gate_rx`
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
        self.start_group_server_connection_handler().await;
        let client_connection_handle = self.start_client_connection_handler().await;

        client_connection_handle.await?;
        Ok(())
    }
}

mod tests {}
