use colored::{Color, Colorize};
use common_utils::{
    network::{connection_handler::ConnectionHandler, tcp_connection::*, tcp_server::*},
    packet::{
        init::{InitGroupPacket, PlayerForGroupServer, SyncPlayerListToGroupServerPacket},
        *,
    },
};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::{
    select,
    sync::{mpsc, Mutex, RwLockReadGuard},
    task::JoinSet,
    time::{interval, sleep},
};

use crate::{
    gate_server::{PlayerConnection, PlayerConnectionMap},
    group_server::GroupServer,
};

pub enum GateGroupCommands {}

pub struct GroupServerHandler {}

impl GroupServerHandler {
    pub fn new() -> Self {
        GroupServerHandler {}
    }

    pub async fn sync_player_list(
        group_connection: Arc<Mutex<TcpConnection<GroupServer>>>,
        player_map: RwLockReadGuard<'_, HashMap<u32, PlayerConnection>>,
        gate_server_name: String,
    ) {
        let mut interval = interval(Duration::from_millis(100));
        loop {
            let _ = interval.tick().await;
            let mut connection = group_connection.lock().await;
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

impl ConnectionHandler<GroupServer, GateGroupCommands> for GroupServerHandler {
    fn on_connect(
        id: String,
        stream: tokio::net::TcpStream,
        socket: std::net::SocketAddr,
    ) -> TcpConnection<GroupServer> {
        let mut connection = TcpConnection::new(
            id,
            stream,
            socket,
            "GroupServer".to_string(),
            colored::Color::Magenta,
        );
        let group_server = GroupServer::new();

        connection.set_application_context(group_server);
        connection
    }

    fn on_connected(connection: Arc<Mutex<TcpConnection<GroupServer>>>) -> anyhow::Result<()> {
        let (recv_tx, mut recv_rx) = mpsc::channel(100);
        let (send_tx, send_rx) = mpsc::channel(100);

        let mut group_conn = connection.clone();
        tokio::spawn(async move {
            let mut connection = group_conn.lock().await;
            println!("took lock");
            connection
                .start_processing(recv_tx, send_rx, send_tx.clone())
                .await;

            println!("gonna drop lock");
        });

        tokio::spawn(async move {
            loop {
                match recv_rx.recv().await {
                    Some(packet) => {
                        GroupServerHandler::on_data(packet);
                    }
                    None => {
                        println!("{}", "GroupServer channel closed".red());
                        break;
                    }
                }
            }
        });

        group_conn = connection.clone();
        tokio::spawn(async move {
            let init_group_packet = InitGroupPacket::new(103, "GateServer".to_string())
                .to_base_packet()
                .unwrap();

            let mut connection = group_conn.lock().await;
            println!("took lock");
            let mut response = connection.sync_rpc(init_group_packet).await.unwrap();

            let err = response.read_short();
            if (err.is_some() && err.unwrap() == 501) || !response.has_data() {
                println!("{}", "Group server rejected connection".red());
                sleep(Duration::from_secs(5)).await;
                connection.close().await.unwrap();
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

            println!("gonna drop lock");
        });

        Ok(())
    }

    fn on_data(packet: BasePacket) {
        println!("Received packet: {:?}", packet);
    }
}
