use async_trait::async_trait;
use colored::{Color, Colorize};
use common_utils::{
    network::{
        connection_handler::ConnectionHandler, tcp_client::TcpClient, tcp_connection::*,
        tcp_server::*,
    },
    packet::{
        init::{InitGroupPacket, PlayerForGroupServer, SyncPlayerListToGroupServerPacket},
        *,
    },
};
use std::{collections::HashMap, net::Ipv4Addr, sync::Arc, time::Duration};
use tokio::{
    select,
    sync::{mpsc, Mutex, MutexGuard, RwLock, RwLockReadGuard},
    task::JoinSet,
    time::{interval, sleep},
};

use crate::{
    gate_server::{PlayerConnection, PlayerConnectionMap},
    group_server::{self, GroupServer},
};

pub enum GateGroupCommands {}

#[derive(Debug)]
pub struct GroupServerHandler {
    connection: Arc<RwLock<Option<TcpConnection<GroupServer>>>>,
    gate_commands_tx: Option<mpsc::Sender<GateGroupCommands>>,
    should_sync_player_list: bool,
}

impl GroupServerHandler {
    pub fn new() -> Self {
        GroupServerHandler {
            connection: Arc::new(RwLock::new(None)),
            gate_commands_tx: None,
            should_sync_player_list: true,
        }
    }

    pub async fn sync_rpc(&self, packet: BasePacket) -> anyhow::Result<BasePacket> {
        let mut write_lock = self.connection.write().await;
        let connection = write_lock.as_mut().unwrap();
        connection.sync_rpc(packet).await
    }

    pub async fn send_data(&self, packet: BasePacket) -> anyhow::Result<()> {
        let mut write_lock = self.connection.write().await;
        let connection = write_lock.as_mut().unwrap();
        connection.send_data(packet).await
    }

    async fn is_tcp_connection_established(&self) -> bool {
        self.connection.read().await.is_some()
    }

    async fn connect(&self, ip: Ipv4Addr, port: u16) {
        let mut client = TcpClient::new(1, "GroupServer".to_string(), ip, port);
        let stream = client
            .connect::<GroupServerHandler, GroupServer, GateGroupCommands>(5)
            .await
            .unwrap();

        let socket_addr = stream.peer_addr().unwrap();

        let mut connection = TcpConnection::new(
            1,
            "GroupServer".to_string(),
            stream,
            socket_addr,
            "GroupServer".to_string(),
            Color::Magenta,
        );

        let group_server = GroupServer::new();
        connection.set_application_context(group_server);

        let group_connection = self.connection.clone();

        let mut write_lock = group_connection.write().await;
        write_lock.replace(connection);
    }

    async fn send_init_packet(&self, gate_server_name: String) -> anyhow::Result<()> {
        let group_conn = self.connection.clone();
        let init_group_packet = InitGroupPacket::new(103, gate_server_name)
            .to_base_packet()
            .unwrap();

        let mut write_lock = group_conn.write().await;
        let connection = write_lock.as_mut().unwrap();

        let response = connection.sync_rpc(init_group_packet).await;
        if response.is_err() {
            return Err(anyhow::anyhow!(
                "Failed to send init packet to group server"
            ));
        }

        let mut response = response.unwrap();

        let err = response.read_short();
        if (err.is_some() && err.unwrap() == 501) || !response.has_data() {
            println!("{}", "Group server rejected connection".red());
            sleep(Duration::from_secs(5)).await;
            connection.close(Duration::ZERO).await;
            return Err(anyhow::anyhow!("Group server rejected connection"));
        } else {
            connection.logger.debug("Marking connection as connected");
            connection.mark_connected();
        }

        connection.logger.info(
            format!(
                "Connected to group server at {}",
                connection.get_socket_addr().to_string()
            )
            .as_str(),
        );

        Ok(())
    }

    pub async fn start_connection_loop(
        handler: Arc<RwLock<Self>>,
        player_map: PlayerConnectionMap,
        gate_server_name: String,
        ip: Ipv4Addr,
        port: u16,
        group_to_gate_tx: mpsc::Sender<GateGroupCommands>,
    ) {
        let player_map = player_map.clone();

        loop {
            let readable_handler = handler.read().await;
            if !readable_handler.is_tcp_connection_established().await {
                readable_handler.connect(ip, port).await;

                drop(readable_handler);

                GroupServerHandler::on_connected(handler.clone(), group_to_gate_tx.clone())
                    .await
                    .unwrap();

                let readable_handler = handler.read().await;

                if (readable_handler
                    .send_init_packet(gate_server_name.clone())
                    .await)
                    .is_err()
                {
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            } else {
                let read_lock = readable_handler.connection.read().await;
                let connection = read_lock.as_ref().unwrap();

                if !connection.is_ready() {
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }

                drop(read_lock);

                if readable_handler.should_sync_player_list {
                    drop(readable_handler);
                    let mut writable_handler = handler.write().await;
                    let player_map = player_map.clone();
                    let player_list = player_map.read().await;

                    writable_handler
                        .sync_player_list(player_list, gate_server_name.clone())
                        .await;

                    sleep(Duration::from_secs(1)).await;
                    continue;
                }

                sleep(Duration::from_secs(1)).await;
            }
        }
    }

    pub async fn sync_player_list(
        &mut self,
        player_map: RwLockReadGuard<'_, HashMap<u32, PlayerConnection>>,
        gate_server_name: String,
    ) {
        let player_count = player_map.len() as u32;
        let mut join_set = JoinSet::new();
        let player_map_copy = player_map.clone();
        let mut player_list: Vec<PlayerForGroupServer> = vec![];

        for (player_id, player_conn) in player_map_copy.iter() {
            let player_id = *player_id;
            let player_conn = player_conn.clone();
            let connection = player_conn.clone();

            join_set.spawn(async move {
                let player_conn = connection.lock().await;
                let player = player_conn.get_application_context().unwrap();
                PlayerForGroupServer {
                    player_pointer: player_id,
                    login_id: player.get_login_id(),
                    account_id: player.get_account_id(),
                }
            });
        }

        while let Some(res) = join_set.join_next().await {
            if res.is_ok() {
                player_list.push(res.unwrap());
            }
        }

        let packet =
            SyncPlayerListToGroupServerPacket::new(player_count, gate_server_name, player_list);
        let built_packet = packet.to_base_packet().unwrap();

        let mut writable_connection = self.connection.write().await;
        let connection = writable_connection.as_mut().unwrap();

        if (connection.sync_rpc(built_packet).await).is_ok() {
            connection.logger.info(
                format!(
                    "Synced player list to group server, {} players",
                    player_count
                )
                .as_str(),
            );

            self.should_sync_player_list = false;
        }
    }
}

#[async_trait]
impl ConnectionHandler<GroupServer, GateGroupCommands> for GroupServerHandler {
    // This is not used, because we're directly creating the handler in the GateServer
    // which is then passed to the clients etc.
    fn on_connect(
        id: u32,
        server_type: String,
        stream: tokio::net::TcpStream,
        socket: std::net::SocketAddr,
    ) -> Arc<RwLock<Self>> {
        let handler = Arc::new(RwLock::new(Self::new()));
        let mut connection = TcpConnection::new(
            id,
            server_type,
            stream,
            socket,
            "GroupServer".to_string(),
            Color::Magenta,
        );
        let group_server = GroupServer::new();
        connection.set_application_context(group_server);

        handler
    }

    async fn on_connected(
        handler: Arc<RwLock<Self>>,
        parent_comm_tx: mpsc::Sender<GateGroupCommands>,
    ) -> anyhow::Result<()> {
        // receive data from the GroupServer
        let (recv_tx, mut recv_rx) = mpsc::channel(100);

        // send data to the GroupServer
        let (send_tx, send_rx) = mpsc::channel(100);
        {
            let binding = handler.clone();
            let mut writable_handler = binding.write().await;
            writable_handler.gate_commands_tx = Some(parent_comm_tx);
        }

        {
            let binding = handler.clone();
            let group_handler = binding.read().await;

            let group_conn = group_handler.connection.clone();
            tokio::spawn(async move {
                let mut write_lock = group_conn.write().await;
                let connection = write_lock.as_mut().unwrap();

                connection
                    .start_processing(recv_tx, send_rx, send_tx.clone())
                    .await;
            })
            .await
            .unwrap();
        }
        {
            let binding = handler.clone();
            tokio::spawn(async move {
                loop {
                    match recv_rx.recv().await {
                        Some(packet) => {
                            let group_handler = binding.read().await;
                            group_handler.on_data(packet);
                        }
                        None => {
                            println!("{}", "GroupServer channel closed".red());
                            break;
                        }
                    }
                }
            });
        }

        Ok(())
    }

    fn on_data(&self, packet: BasePacket) {
        let conn = self.connection.clone();
        tokio::spawn(async move {
            let read_lock = conn.read().await;
            let connection = read_lock.as_ref().unwrap();
            connection.logger.debug("Received packet");
            packet.inspect_with_logger(&connection.logger);
        });
    }
}
