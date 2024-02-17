use async_trait::async_trait;
use colored::{Color, Colorize};
use common_utils::{
    network::{connection_handler::ConnectionHandler, tcp_connection::*, tcp_server::*},
    packet::{
        init::{InitGroupPacket, PlayerForGroupServer, SyncPlayerListToGroupServerPacket},
        *,
    },
};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{
    select,
    sync::{mpsc, Mutex, MutexGuard, RwLock, RwLockReadGuard},
    task::JoinSet,
    time::{interval, sleep},
};

use crate::{
    gate_server::{PlayerConnection, PlayerConnectionMap},
    group_server::GroupServer,
};

pub enum GateGroupCommands {}

#[derive(Debug)]
pub struct GroupServerHandler {
    connection: Arc<Mutex<TcpConnection<GroupServer>>>,
    gate_commands_tx: Option<mpsc::Sender<GateGroupCommands>>,
}

impl GroupServerHandler {
    pub fn new(connection: Arc<Mutex<TcpConnection<GroupServer>>>) -> Self {
        GroupServerHandler {
            connection: connection.clone(),
            gate_commands_tx: None,
        }
    }

    pub async fn sync_rpc(&self, packet: BasePacket) -> anyhow::Result<BasePacket> {
        self.connection.lock().await.sync_rpc(packet).await
    }

    pub async fn send_data(&self, packet: BasePacket) -> anyhow::Result<()> {
        self.connection.lock().await.send_data(packet).await
    }

    pub async fn sync_player_list(
        &self,
        player_map: RwLockReadGuard<'_, HashMap<u32, PlayerConnection>>,
        gate_server_name: String,
    ) {
        let mut interval = interval(Duration::from_millis(100));
        loop {
            let _ = interval.tick().await;
            let mut connection = self.connection.lock().await;
            if !connection.is_connected() {
                continue;
            }

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

            if (connection.sync_rpc(built_packet).await).is_ok() {
                connection.logger.info(
                    format!(
                        "Synced player list to group server, {} players",
                        player_count
                    )
                    .as_str(),
                );
            }

            break;
        }
    }
}

#[async_trait]
impl ConnectionHandler<GroupServer, GateGroupCommands> for GroupServerHandler {
    fn on_connect(
        id: u32,
        server_type: String,
        stream: tokio::net::TcpStream,
        socket: std::net::SocketAddr,
    ) -> Arc<RwLock<Self>> {
        let prefix = format!("{}.{}", server_type, id);
        let mut connection = TcpConnection::new(
            id,
            server_type,
            stream,
            socket,
            prefix,
            colored::Color::Magenta,
        );
        let group_server = GroupServer::new();
        connection.set_application_context(group_server);
        let (gate_commands_tx, _gate_commands_rx) = mpsc::channel::<GateGroupCommands>(100);
        let handler = GroupServerHandler::new(Arc::new(Mutex::new(connection)));
        Arc::new(RwLock::new(handler))
    }

    async fn on_connected(
        handler: Arc<RwLock<Self>>,
        parent_comm_tx: mpsc::Sender<GateGroupCommands>,
    ) -> anyhow::Result<()> {
        let (recv_tx, mut recv_rx) = mpsc::channel(100);
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
                let mut connection = group_conn.lock().await;
                connection
                    .start_processing(recv_tx, send_rx, send_tx.clone())
                    .await;
            });
        }
        {
            let binding = handler.clone();
            tokio::spawn(async move {
                let group_handler = binding.read().await;

                loop {
                    match recv_rx.recv().await {
                        Some(packet) => {
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

        let binding = handler.clone();
        tokio::spawn(async move {
            let group_handler = binding.read().await;
            let group_conn = group_handler.connection.clone();
            let init_group_packet = InitGroupPacket::new(103, "GateServer".to_string())
                .to_base_packet()
                .unwrap();

            let mut connection = group_conn.lock().await;
            let mut response = connection.sync_rpc(init_group_packet).await.unwrap();

            let err = response.read_short();
            if (err.is_some() && err.unwrap() == 501) || !response.has_data() {
                println!("{}", "Group server rejected connection".red());
                sleep(Duration::from_secs(5)).await;
                connection.close(Duration::ZERO).await;
            } else {
                connection.mark_connected();
            }

            connection.logger.info(
                format!(
                    "Connected to group server at {}",
                    connection.get_socket_addr().to_string()
                )
                .as_str(),
            );
        });

        Ok(())
    }

    fn on_data(&self, packet: BasePacket) {
        let conn = self.connection.clone();
        tokio::spawn(async move {
            let conn = conn.lock().await;
            conn.logger.debug("Received packet");
            packet.inspect_with_logger(&conn.logger);
        });
    }
}
