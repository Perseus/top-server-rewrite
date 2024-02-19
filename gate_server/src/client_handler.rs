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
}

pub enum ClientGateCommands {
    Disconnect(u32),
}

impl ClientHandler {
    pub fn new(connection: Arc<Mutex<TcpConnection<Player>>>) -> ClientHandler {
        ClientHandler {
            connection,
            gate_comm_tx: None,
            groupserver_handler: None,
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

#[async_trait]
impl ConnectionHandler<Player, ClientGateCommands> for ClientHandler {
    fn on_connect(
        id: u32,
        connection_type: String,
        stream: tokio::net::TcpStream,
        socket: std::net::SocketAddr,
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

        Arc::new(RwLock::new(ClientHandler::new(Arc::new(Mutex::new(
            tcp_connection,
        )))))
    }

    async fn on_connected(
        handler: Arc<RwLock<Self>>,
        gate_comm_tx: mpsc::Sender<ClientGateCommands>,
    ) -> anyhow::Result<()> {
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

            println!("{}", "Client channel closed");
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
            Some(Command::None) => {
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

                println!("Player logged in: {:?}", updated_player);
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
}
