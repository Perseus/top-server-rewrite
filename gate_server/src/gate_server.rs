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
    gameserver::{self, GameServer, GameServerList},
    gameserver_handler::{GameServerGateCommands, GameServerHandler, GameServerRejectionReason},
    group_server::GroupServer,
    groupserver_handler::{GateGroupCommands, GroupServerHandler},
    player::Player,
};

// TODO: figure out how to set this from config at runtime
pub const SUPPORTED_CLIENT_VERSION: u16 = 136;

pub const MAXIMUM_MAPS_PER_GAMESERVER: u16 = 100;

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

pub type PlayerConnection = Arc<RwLock<TcpConnection<Player>>>;
pub type PlayerConnectionMap = Arc<RwLock<HashMap<u32, PlayerConnection>>>;

pub struct GateServer {
    name: String,
    config: GateServerConfig,
    client_connections: PlayerConnectionMap,

    // Atomic counter to give unique IDs to players. We need this to be a U32 because
    // other services store U32 as the player's "gate address" which is required to communicate to
    // specific players directly
    player_num_counter: Arc<AtomicU32>,
    group_handler: Arc<RwLock<GroupServerHandler>>,

    game_server_counter: Arc<AtomicU32>,
    game_servers: Arc<RwLock<GameServerList>>,
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
        let client_connections = Arc::new(RwLock::new(HashMap::new()));

        GateServer {
            name: config.get_name(),
            config,
            player_num_counter: Arc::new(AtomicU32::new(1)),
            game_server_counter: Arc::new(AtomicU32::new(1)),
            group_handler: Arc::new(RwLock::new(GroupServerHandler::new(
                client_connections.clone(),
            ))),
            client_connections,
            game_servers: Arc::new(RwLock::new(GameServerList::new())),
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
        gameserver_list: Arc<RwLock<GameServerList>>,
    ) {
        let mut map = client_conn_map.write().await;
        // relaxed ordering is fine, this is just to give a unique ID to the player
        // there aren't any other
        let id = player_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let socket_addr = stream.peer_addr().unwrap();
        let handler = ClientHandler::on_connect(
            id,
            "Client".to_string(),
            stream,
            socket_addr,
            gameserver_list,
        );

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
        let groupserver_handler = self.group_handler.clone();
        let (client_to_gate_tx, mut client_to_gate_rx) = mpsc::channel::<ClientGateCommands>(100);
        let client_tcp_connections_for_client_commands_handler = client_tcp_connections.clone();
        let gameserver_list = self.game_servers.clone();

        tokio::spawn(async move {
            let client_tcp_connections = client_tcp_connections_for_client_commands_handler;
            while let Some(command) = client_to_gate_rx.recv().await {
                match command {
                    ClientGateCommands::Disconnect(id) => {
                        let mut map = client_tcp_connections.write().await;
                        if let Some(connection) = map.remove(&id) {
                            let mut connection = connection.write().await;
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
                    gameserver_list.clone(),
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

        let (group_to_gate_tx, mut group_to_gate_rx) = mpsc::channel::<GateGroupCommands>(100);

        let player_map = self.client_connections.clone();
        let gate_server_name = self.config.get_name();
        let gp_handler = self.group_handler.clone();
        tokio::spawn(async move {
            GroupServerHandler::start_connection_loop(
                gp_handler,
                player_map,
                gate_server_name,
                ip,
                port,
                group_to_gate_tx,
            )
            .await;
        });

        tokio::spawn(async move {
            while let Some(command) = group_to_gate_rx.recv().await {
                match command {}
            }
        });
    }

    async fn start_game_server_connection_handler(&mut self) {
        let (ip, port) = self.config.get_ip_and_port_for_game_server();

        let connect_addr = format!("{}:{}", ip, port);

        let (tcp_stream_tx, mut tcp_stream_rx) = tokio::sync::mpsc::channel::<TcpStream>(100);
        let counter = self.game_server_counter.clone();

        /*
         * Start the TCP server, hand it off to a separate task
         */
        tokio::spawn(async move {
            TcpServer::start(connect_addr, tcp_stream_tx).await.unwrap();
        });

        println!(
            "{} {}",
            "Ready to accept game server connections at port".green(),
            port.to_string().green().underline(),
        );

        let game_server_list = self.game_servers.clone();
        let (game_to_gate_tx, mut game_to_gate_rx) = mpsc::channel::<GameServerGateCommands>(100);

        let gate_server_name = self.name.clone();
        let player_list = self.client_connections.clone();

        tokio::spawn(async move {
            let gate_server_name = gate_server_name;
            let game_server_list = game_server_list.clone();
            let player_list = player_list;
            while let Some(stream) = tcp_stream_rx.recv().await {
                let id = counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let gate_server_name = gate_server_name.clone();

                GameServerHandler::handle_incoming_connection(
                    id,
                    stream,
                    "GameServer".to_string(),
                    game_server_list.clone(),
                    game_to_gate_tx.clone(),
                    gate_server_name,
                    player_list.clone(),
                );
            }
        });

        let game_server_list = self.game_servers.clone();
        let player_list = self.client_connections.clone();
        let group_handler = self.group_handler.clone();

        tokio::spawn(async move {
            let player_list = player_list;
            let group_handler = group_handler;
            while let Some(command) = game_to_gate_rx.recv().await {
                match command {
                    GameServerGateCommands::SendPacketToClient(player_id, packet) => {
                        let player_list = player_list.clone();
                        tokio::spawn(async move {
                            let player_list = player_list.read().await;
                            let player = player_list.get(&player_id).unwrap();
                            let player = player.read().await;

                            player
                                .logger
                                .debug("Sending packet from GameServer to client");
                            packet.inspect_with_logger(&player.logger);

                            if player.send_data(packet).await.is_err() {
                                player.logger.error("Failed to send packet to client");
                            }
                        });
                    }

                    GameServerGateCommands::SendPacketToGroupServer(packet) => {
                        let gp_handler = group_handler.clone();
                        tokio::spawn(async move {
                            let gp_handler = gp_handler.write().await;
                            let _ = gp_handler.send_data(packet).await;
                        });
                    }
                }
            }
        });
    }

    pub async fn start(&mut self) -> anyhow::Result<()> {
        self.start_group_server_connection_handler().await;
        self.start_game_server_connection_handler().await;
        let client_connection_handle = self.start_client_connection_handler().await;

        client_connection_handle.await?;
        Ok(())
    }
}

mod tests {}
