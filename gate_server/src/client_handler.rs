use std::{
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use common_utils::{
    network::{connection_handler::ConnectionHandler, tcp_connection::TcpConnection},
    packet::{
        auth::{
            ClientDisconnectedForGroupPacket, ClientLoginErrorPacket, ClientLogoutFromGroupPacket,
            ClientMapCrashPacket,
        },
        commands::*,
        init::InitClientPacket,
        BasePacket, PacketReader, PacketWriter, TypedPacket,
    },
};
use tokio::sync::{mpsc, Mutex, RwLock};

use crate::{
    gameserver::GameServerList,
    gameserver_handler::GameServerHandler,
    gate_commands::GateCommands,
    gate_server::{GateServer, SUPPORTED_CLIENT_VERSION},
    groupserver_handler::{self, GroupServerHandler},
    metrics::CONNECTIONS_COUNTER,
    player::Player,
};

pub struct ClientHandler {
    connection: Arc<Mutex<TcpConnection<Player>>>,
    gate_comm_tx: Option<mpsc::Sender<ClientGateCommands>>,
    groupserver_handler: Option<Arc<RwLock<GroupServerHandler>>>,
    gameserver_list: Arc<RwLock<GameServerList>>,
}

pub enum ClientGateCommands {
    Disconnect(u32),
}

impl ClientHandler {
    pub fn new(
        connection: Arc<Mutex<TcpConnection<Player>>>,
        gameserver_list: Arc<RwLock<GameServerList>>,
    ) -> ClientHandler {
        ClientHandler {
            connection,
            gate_comm_tx: None,
            groupserver_handler: None,
            gameserver_list,
        }
    }

    pub fn get_connection(&self) -> Arc<Mutex<TcpConnection<Player>>> {
        self.connection.clone()
    }

    pub fn set_groupserver_handler_ref(
        &mut self,
        groupserver_handler: Arc<RwLock<GroupServerHandler>>,
    ) {
        self.groupserver_handler = Some(groupserver_handler);
    }
}

impl ClientHandler {
    pub fn on_connect(
        id: u32,
        connection_type: String,
        stream: tokio::net::TcpStream,
        socket: std::net::SocketAddr,
        gameserver_list: Arc<RwLock<GameServerList>>,
    ) -> Arc<RwLock<Self>> {
        let mut tcp_connection = TcpConnection::<Player>::new(
            id,
            connection_type,
            stream,
            socket,
            format!("client.{}", id),
            colored::Color::Blue,
        );

        CONNECTIONS_COUNTER.inc();

        // initialize the "player", which will contain any client-specific data
        let player = Player::new();
        tcp_connection.set_application_context(player);

        Arc::new(RwLock::new(ClientHandler::new(
            Arc::new(Mutex::new(tcp_connection)),
            gameserver_list,
        )))
    }

    pub async fn on_connected(
        handler: Arc<RwLock<Self>>,
        gate_comm_tx: mpsc::Sender<ClientGateCommands>,
    ) -> anyhow::Result<()> {
        println!("on connected for client");
        // channel where we get data from the client (recv)
        // separate thread pulling data from the client TCP connection, where a thread consumes it, converts it into a BasePacket, and then pushes it to recv_tx, and can be consumed through recv_rx
        let (recv_tx, mut recv_rx) = mpsc::channel(100);

        // channel used to send data to the client
        // push messages to _send_tx, they are received by a separate thread consuming from send_rx, and then sent to the client
        let (_send_tx, send_rx) = mpsc::channel(100);
        let channel_for_init_packet = _send_tx.clone();
        {
            let mut locked_handler = handler.write().await;
            locked_handler.gate_comm_tx = Some(gate_comm_tx);
        }

        let binding = handler.clone();
        let client_handler = binding.read().await;
        let client_connection = client_handler.connection.clone();
        let _send_tx_for_processor = _send_tx.clone();
        tokio::spawn(async move {
            let mut connection = client_connection.lock().await;
            let application_context = connection.get_application_context();
            if application_context.is_none() {
                return Err(anyhow::anyhow!("No application context").context("on_connected"));
            }

            let chapstr = application_context.unwrap().get_chapstr();
            let client_init_packet = InitClientPacket::new(chapstr).to_base_packet().unwrap();
            channel_for_init_packet
                .send(client_init_packet)
                .await
                .unwrap();

            connection
                .start_processing(recv_tx, send_rx, _send_tx_for_processor)
                .await;

            Ok(())
        });

        let binding = handler.clone();
        tokio::spawn(async move {
            let readable_handler = binding.read().await;
            while let Some(packet) = recv_rx.recv().await {
                readable_handler.on_data(packet);
            }

            println!("Client channel closed");
        });

        Ok(())
    }

    fn on_data(&self, mut packet: BasePacket) {
        // TODO: handle wpe code
        let wpe_high = packet.reverse_read_char();
        let wpe_low = packet.reverse_read_char();

        // remove the packet counter data
        packet.remove_range(8..12);

        // remove the 2 characters used for the wpe code
        packet.discard_last(2);

        if wpe_high.is_none() || wpe_low.is_none() {
            todo!("WPE code not found");
        }

        match packet.read_cmd() {
            Some(Command::CTGTLogin) => {
                self.handle_user_login(packet);
            }

            Some(Command::CTGTSelectCharacter) => {
                self.handle_select_character(packet);
            }

            Some(Command::None) => {
                let raw_cmd = packet.get_raw_cmd();
                if (1..=500).contains(&raw_cmd) {
                    self.send_packet_to_gameserver(packet);
                    return;
                }
                self.handle_disconnect();
            }
            Some(cmd) => {
                todo!("Command not handled: {:?}", cmd)
            }

            None => {
                todo!("None condition")
            }
        }
    }

    // fn on_error(&mut self, _stream: tokio::net::TcpListener, _error: &str) {
    //     todo!()
    // }
}

impl ClientHandler {
    fn send_packet_to_gameserver(&self, mut packet: BasePacket) {
        let connection = self.connection.clone();
        tokio::spawn(async move {
            let connection = connection.lock().await;
            let player = connection.get_application_context().unwrap();

            if let Some(game_server) = player.get_current_game_server() {
                let game_server = game_server.read().await;
                let connection = game_server.get_connection();
                let connection = connection.read().await;
                if connection.send_data(packet).await.is_err() {
                    connection.logger.error(&format!(
                        "Error sending packet to game server: {} for player: {}",
                        connection.get_id(),
                        player.get_login_id()
                    ));
                }
            }
        });
    }

    fn handle_user_login(&self, mut packet: BasePacket) {
        let connection = self.connection.clone();
        let gpserver_handler = self.groupserver_handler.clone().unwrap();

        // let gpserver_handler = self.groupserver_handler.unwrap().as_ref();

        tokio::spawn(async move {
            let mut conn = connection.lock().await;
            conn.logger.debug("Handling user login");
            let application_context = conn.get_application_context();

            if application_context.is_none() {
                conn.logger.error("No player data found");
                conn.close(Duration::ZERO).await;
                return;
            }

            let player = application_context.unwrap();

            // if this connection is already associated with a player, then we should not allow another login
            if player.get_account_id() > 0 {
                conn.logger.error("Player already logged in");
                let error_pkt = ClientMapCrashPacket::new(
                    "Login error - You are already logged in".to_string(),
                )
                .to_base_packet()
                .unwrap();

                conn.send_data(error_pkt).await.unwrap();
                conn.close(Duration::from_millis(100)).await;
                return;
            }

            // if the packet is older than 30 seconds, we should not allow the login
            let time_since_packet_creation = packet.get_time_since_creation();
            if time_since_packet_creation > Duration::from_secs(30) {
                let err_pkt = ClientLoginErrorPacket::new(ErrorCode::ErrNetworkException)
                    .to_base_packet()
                    .unwrap();
                conn.send_data(err_pkt).await.unwrap();
                conn.close(Duration::from_millis(100)).await;
                return;
            }

            // if the client version is not supported, we should not allow the login
            let client_version = packet.reverse_read_short().unwrap();
            if client_version != SUPPORTED_CLIENT_VERSION {
                let err_pkt = ClientLoginErrorPacket::new(ErrorCode::ErrClientVersionMismatch)
                    .to_base_packet()
                    .unwrap();
                conn.send_data(err_pkt).await.unwrap();
                conn.close(Duration::from_millis(100)).await;
                return;
            }

            let ip_addr = conn.get_ip_as_u32();
            if ip_addr.is_err() {
                let err_pkt = ClientLoginErrorPacket::new(ErrorCode::ErrNetworkException)
                    .to_base_packet()
                    .unwrap();
                conn.send_data(err_pkt).await.unwrap();
                conn.close(Duration::from_millis(100)).await;
                return;
            }

            let ip_addr_as_u32 = ip_addr.unwrap();

            // wpe version check
            // TODO: remove this, feel like its useless
            packet.reverse_read_short().unwrap();
            // TODO: do this conditionally based on if the wpe version was correct
            packet.discard_last(4);
            packet.build_packet().unwrap();

            let mut gpserver_packet = packet.duplicate();

            gpserver_packet.write_cmd(Command::GTTGPUserLogin).unwrap();
            gpserver_packet
                .write_string(player.get_chapstr().as_str())
                .unwrap();
            gpserver_packet.write_long(ip_addr_as_u32).unwrap();
            gpserver_packet.write_long(conn.get_id()).unwrap();
            gpserver_packet.write_short(916).unwrap();

            let gpserver_handler = gpserver_handler.read().await;
            if let Ok(mut result) = gpserver_handler.sync_rpc(gpserver_packet).await {
                if !result.has_data() {
                    let err_pkt = ClientLoginErrorPacket::new(ErrorCode::ErrNetworkException)
                        .to_base_packet()
                        .unwrap();

                    conn.logger
                        .error("Did not get a response from group server");
                    conn.send_data(err_pkt).await.unwrap();
                    conn.close(Duration::from_millis(100)).await;
                    return;
                }

                let error_code = result.read_short();
                conn.logger
                    .debug(format!("Error code: {:?}", error_code).as_str());
                if error_code.is_some() && error_code.unwrap() != 0 {
                    result.inspect();
                    let mut err_pkt = result.duplicate();
                    err_pkt.write_cmd(Command::GTTCLogin).unwrap();
                    err_pkt.reset_header();
                    err_pkt.build_packet().unwrap();

                    conn.logger.error(
                        format!("Error code from group server: {}", error_code.unwrap()).as_str(),
                    );
                    conn.send_data(err_pkt).await.unwrap();
                    conn.close(Duration::from_millis(100)).await;
                    return;
                }

                let player_group_addr = result.reverse_read_long().unwrap();
                let player_login_id = result.reverse_read_long().unwrap();
                let player_act_id = result.reverse_read_long().unwrap();

                let by_password = result.reverse_read_char().unwrap();
                let comm_key_length = result.reverse_read_short().unwrap();
                let comm_text_key = result.reverse_read_bytes(comm_key_length as usize).unwrap();
                let mut updated_player = Player::new();
                updated_player.set_chapstr(player.get_chapstr().clone());
                updated_player.set_logged_in_context(
                    player_group_addr,
                    player_login_id,
                    player_act_id,
                    comm_key_length,
                    String::from_utf8_lossy(comm_text_key).to_string(),
                    ip_addr_as_u32,
                );

                conn.set_application_context(updated_player);
                result.discard_last(12 + 1 + 2 + (comm_key_length as usize) + 2);

                let mut client_pkt = result.duplicate();
                client_pkt.write_cmd(Command::GTTCLogin).unwrap();
                client_pkt.write_char(by_password).unwrap();

                // TODO: communication encryption config
                client_pkt.write_long(0).unwrap();
                client_pkt.write_long(0x3214).unwrap();
                client_pkt.reset_header();
                client_pkt.build_packet().unwrap();

                conn.send_data(client_pkt).await.unwrap();

                conn.logger.debug("User login successful");
                // player.
            } else {
                conn.logger
                    .error("Error sending login packet to group server");
            }
        });
    }

    fn handle_user_logout(&self) {}

    fn handle_disconnect(&self) {
        let connection = self.connection.clone();
        let gpserver_conn = self.groupserver_handler.clone().unwrap();
        let gate_comm_tx = self.gate_comm_tx.clone().unwrap();
        tokio::spawn(async move {
            let mut conn = connection.lock().await;
            let player = conn.get_application_context().unwrap();
            if !player.is_active() {
                let mut err_pkt = BasePacket::new();
                err_pkt.write_cmd(Command::None).unwrap();
                err_pkt
                    .write_short(ErrorCode::ErrClientNotLoggedIn as u16)
                    .unwrap();

                conn.send_data(err_pkt).await.unwrap();
                conn.close(Duration::from_millis(100)).await;
                return;
            }

            let grp_disconnect_pkt = ClientDisconnectedForGroupPacket::new(
                player.get_account_id(),
                player.get_ip_addr(),
                "-27".to_string(),
            )
            .to_base_packet()
            .unwrap();

            let gpserver = gpserver_conn.read().await;

            gpserver.send_data(grp_disconnect_pkt).await.unwrap();

            let grp_logout_pkt =
                ClientLogoutFromGroupPacket::new(conn.get_id(), player.get_group_addr())
                    .to_base_packet()
                    .unwrap();

            gpserver.sync_rpc(grp_logout_pkt).await.unwrap();

            gate_comm_tx
                .send(ClientGateCommands::Disconnect(conn.get_id()))
                .await
                .unwrap();
        });
    }

    fn handle_select_character(&self, mut packet: BasePacket) {
        /**
         * 1. Check if the player is on the character selection screen
         *  (is_active should be true)
         *  (group_addr should not be 0)
         *
         * 2. Make a sync RPC to the GroupServer
         *    
         * 3. Extract data from response (map, world id etc)
         *
         * 4. If the map isnt found in any running game servers, send error
         *
         * 5. If the number of players in that gameserver is greater than some amount, send error
         *
         * 6.
         */
        let connection = self.connection.clone();
        let gpserver_handler = self.groupserver_handler.clone().unwrap();
        let gameserver_list = self.gameserver_list.clone();

        tokio::spawn(async move {
            let mut connection = connection.lock().await;
            let player = connection.get_application_context().unwrap();
            let mut begin_ply_pkt_for_group = packet.duplicate();
            let connection_id = connection.get_id();
            let logger = &connection.logger;
            begin_ply_pkt_for_group.reset_header();
            begin_ply_pkt_for_group
                .write_cmd(Command::GTTGPUserBeginPlay)
                .unwrap();
            begin_ply_pkt_for_group
                .write_long(connection.get_id())
                .unwrap();
            begin_ply_pkt_for_group
                .write_long(player.get_group_addr())
                .unwrap();

            let gpserver_handler = gpserver_handler.read().await;
            let mut result = gpserver_handler
                .sync_rpc(begin_ply_pkt_for_group)
                .await
                .unwrap();

            result.inspect_with_logger(&connection.logger);

            if !result.has_data() {
                connection.logger.error("No data from group server");
                return;
            }

            let err_code = result.read_short().unwrap();
            if err_code != 0 {
                logger.error("Got an error from GroupServer in handle_select_character");
            }

            let player_password = result.read_string().unwrap();
            let player_db_id = result.read_long().unwrap();
            let player_world_id = result.read_long().unwrap();
            let player_map_name = result.read_string().unwrap();
            let garner_winner = result.read_short().unwrap();

            let player = connection.get_application_context_mut().unwrap();
            player.set_begin_play_context(
                player_password,
                player_db_id,
                player_world_id,
                garner_winner,
            );

            let game_servers = gameserver_list.read().await;
            if let Some(game_server) = game_servers
                .find_game_server_with_map(player_map_name.clone())
                .await
            {
                let mut kick_character_pkt = BasePacket::new();
                kick_character_pkt
                    .write_cmd(Command::GTTGMKickCharacter)
                    .unwrap();
                kick_character_pkt.write_long(player_db_id).unwrap();
                kick_character_pkt.build_packet().unwrap();
                let responses = game_servers.send_to_all_game_servers_sync(packet).await;

                if responses.is_err() {
                    connection
                        .logger
                        .error("Error sending kick character packet");
                    return;
                }

                if let Ok(()) = GameServerHandler::enter_map(
                    game_server.clone(),
                    connection_id,
                    player,
                    player.get_account_id(),
                    player_db_id,
                    player_world_id,
                    player_map_name,
                    -1,
                    0,
                    0,
                    0,
                    garner_winner,
                )
                .await
                {}
            } else {
                println!("Map not found for player, {}", player_map_name);
                todo!("handle gameserver for map not found");
            }
        });
    }
}
