use std::{mem::zeroed, net::SocketAddr, sync::Arc};

use async_trait::async_trait;
use common_utils::{
    network::{connection_handler::ConnectionHandler, tcp_connection::TcpConnection},
    packet::{commands::Command, BasePacket, PacketReader, PacketWriter, TypedPacket},
};
use tokio::{
    net::TcpStream,
    sync::{mpsc, Mutex, RwLock},
};

use crate::{
    gameserver::{GameServer, GameServerList},
    gate_server::{PlayerConnection, PlayerConnectionMap},
    group_server::GroupServer,
    groupserver_handler::GroupServerHandler,
    packets::{
        self,
        gameserver::{GameServerEnterMapPacket, GameServerEnterMapResultPacket},
    },
    player::Player,
};

#[derive(Debug)]
pub struct GameServerHandler {
    id: u32,
    connection: Arc<RwLock<TcpConnection<GameServer>>>,
    game_server_list: Arc<RwLock<GameServerList>>,
    gate_server_name: String,
    gate_comm_tx: Option<Arc<mpsc::Sender<GameServerGateCommands>>>,
    client_list: PlayerConnectionMap,
}

impl GameServerHandler {
    pub fn new(
        id: u32,
        connection: Arc<RwLock<TcpConnection<GameServer>>>,
        game_server_list: Arc<RwLock<GameServerList>>,
        gate_comm_tx: Option<Arc<mpsc::Sender<GameServerGateCommands>>>,
        gate_server_name: String,
        client_list: PlayerConnectionMap,
    ) -> Self {
        GameServerHandler {
            id,
            connection,
            game_server_list,
            gate_comm_tx,
            gate_server_name,
            client_list,
        }
    }
}

pub enum GameServerGateCommands {
    SendPacketToClient(u32, BasePacket),
    SendPacketToGroupServer(BasePacket),
}

pub enum GameServerRejectionReason {
    DuplicateName,
    OverlappingMapList,
}

impl Drop for GameServerHandler {
    fn drop(&mut self) {
        println!("GameServerHandler: Dropped");
    }
}

impl GameServerHandler {
    async fn on_connected(
        handler: Arc<RwLock<Self>>,
        gate_comm_tx: mpsc::Sender<GameServerGateCommands>,
    ) -> anyhow::Result<()> {
        let (data_recv_tx, mut data_recv_rx) = mpsc::channel(100);
        let (data_send_tx, data_send_rx) = mpsc::channel(100);

        {
            let mut handler = handler.write().await;
            handler.gate_comm_tx = Some(Arc::new(gate_comm_tx));
        }

        let incoming_packet_processor = handler.clone();
        tokio::spawn(async move {
            let handler = incoming_packet_processor.read().await;
            let connection = handler.connection.clone();
            let mut connection = connection.write().await;

            connection
                .start_processing(data_recv_tx, data_send_rx, data_send_tx.clone())
                .await;
        })
        .await
        .unwrap();

        tokio::spawn(async move {
            loop {
                match data_recv_rx.recv().await {
                    None => {
                        println!(
                            "GameServerHandler: Data receiver stopped due to connection closing"
                        );
                        break;
                    }
                    Some(packet) => {
                        GameServerHandler::on_data(handler.clone(), packet);
                    }
                }
            }

            // what to do on disconnect?
        });

        Ok(())
    }

    fn on_data(handler: Arc<RwLock<Self>>, mut packet: BasePacket) {
        let cmd = packet.read_cmd();
        match cmd {
            Some(Command::GMTGTInit) => {
                GameServerHandler::on_game_server_init(handler, packet);
            }

            Some(Command::GMTGTMapEntry) => {
                GameServerHandler::on_map_entry(handler, packet);
            }

            Some(Command::GMTCEnterMap) => GameServerHandler::on_enter_map_result(handler, packet),

            Some(Command::None) => {
                let raw_cmd = packet.get_raw_cmd();
                if (500..=1000).contains(&raw_cmd) {
                    GameServerHandler::send_to_client(handler, packet);
                }
            }

            _ => {
                println!(
                    "GameServerHandler: Unhandled command: {}",
                    packet.get_raw_cmd()
                );
            }
        }
    }

    pub fn handle_incoming_connection(
        id: u32,
        stream: TcpStream,
        server_type: String,
        game_server_list: Arc<RwLock<GameServerList>>,
        gate_comm_tx: mpsc::Sender<GameServerGateCommands>,
        gate_server_name: String,
        player_list: PlayerConnectionMap,
    ) {
        tokio::spawn(async move {
            // Initialize the GameServer handler
            let socket_addr = stream.peer_addr().unwrap();
            let mut tcp_connection = TcpConnection::new(
                id,
                server_type,
                stream,
                socket_addr,
                format!("GameServer{}", id),
                colored::Color::BrightYellow,
            );

            let gameserver = GameServer::new();
            tcp_connection.set_application_context(gameserver);
            let handler = Arc::new(RwLock::new(Self::new(
                id,
                Arc::new(RwLock::new(tcp_connection)),
                game_server_list.clone(),
                Some(Arc::new(gate_comm_tx.clone())),
                gate_server_name,
                player_list,
            )));

            // Start listening to data coming in from the connection
            GameServerHandler::on_connected(handler.clone(), gate_comm_tx.clone())
                .await
                .unwrap();
        });
    }

    fn on_game_server_init(handler: Arc<RwLock<Self>>, mut packet: BasePacket) {
        tokio::spawn(async move {
            let readable_handler = handler.clone();
            let readable_handler = readable_handler.read().await;
            let init_packet =
                packets::gameserver::GameServerInitPacket::from_base_packet(packet).unwrap();

            let id = readable_handler.id;
            let game_server_list = readable_handler.game_server_list.clone();
            let connection = readable_handler.connection.clone();

            let mut connection = connection.write().await;
            let new_gameserver_data = connection.get_application_context_mut().unwrap();

            new_gameserver_data.set_name(init_packet.game_server_name.clone());
            new_gameserver_data.set_map_list(init_packet.map_list.clone());

            let mut game_server_list = game_server_list.write().await;
            let mut has_duplicate_name = false;
            let mut has_overlapping_map_list = false;

            /*
               Whenever a GameServer connects to the GateServer,
               we need to ensure that the name of the GameServer is unique
               and that it is not already running any of the maps that are
               running on some other GameServer.

               If so, we reject that connection
            */

            let rejection_reason = game_server_list
                .check_for_duplicate_name_or_overlapping_maps(new_gameserver_data)
                .await;

            connection.mark_connected();

            // drop the reference, so that the lock is released
            drop(connection);

            match rejection_reason {
                Ok(_) => {}
                Err(reason) => match reason {
                    GameServerRejectionReason::DuplicateName => {
                        has_duplicate_name = true;
                    }
                    GameServerRejectionReason::OverlappingMapList => {
                        has_overlapping_map_list = true;
                    }
                },
            }

            if has_duplicate_name {
                GameServerHandler::reject_connection(
                    handler.clone(),
                    GameServerRejectionReason::DuplicateName,
                )
                .await;
                return;
            }

            if has_overlapping_map_list {
                GameServerHandler::reject_connection(
                    handler.clone(),
                    GameServerRejectionReason::OverlappingMapList,
                )
                .await;
                return;
            }

            GameServerHandler::acknowledge_init(handler.clone()).await;
            game_server_list.add_game_server(handler).await;
        });
    }

    pub async fn acknowledge_init(handler: Arc<RwLock<Self>>) {
        let handler = handler.read().await;
        let mut packet = BasePacket::new();
        packet.write_cmd(Command::GTTGMInitAcknowledge).unwrap();
        packet.write_short(0).unwrap();
        packet.write_string(&handler.gate_server_name).unwrap();
        packet.build_packet().unwrap();

        let connection = handler.connection.read().await;
        if (connection.send_data(packet).await).is_err() {
            connection
                .logger
                .error("Failed to send init acknowledge packet");
        }

        connection
            .logger
            .info("Acknowledged initialization for GameServer");
    }

    pub fn send_to_client(handler: Arc<RwLock<Self>>, mut packet: BasePacket) {
        tokio::spawn(async move {
            let handler = handler.read().await;
            let gameserver_connection = handler.connection.clone();
            let gameserver_connection = gameserver_connection.read().await;
            let raw_cmd = packet.get_raw_cmd();

            if raw_cmd != 895 && raw_cmd != 511 && raw_cmd != 537 {
                gameserver_connection
                    .logger
                    .debug("Got packet to send to client");
                packet.inspect_with_logger(&gameserver_connection.logger);
            }

            let aim_num = packet.reverse_read_short().unwrap();

            let mut pkt_for_client = packet.clone();
            pkt_for_client.discard_last((4 * 2) * (aim_num as usize) + 2);

            let client_list = handler.client_list.clone();
            let client_list = client_list.read().await;

            for _ in 0..aim_num {
                let player_id = packet.reverse_read_long().unwrap();
                let db_id = packet.reverse_read_long().unwrap();
                if let Some(connection) = client_list.get(&player_id) {
                    let connection = connection.read().await;
                    let player = connection.get_application_context().unwrap();
                    if player.get_db_id() == db_id {
                        if connection.send_data(packet.clone()).await.is_err() {
                            connection
                                .logger
                                .error("Unable to send packet to client, client was disconnected");
                        }
                    } else {
                        println!("db id wasnt a match");
                    }
                } else {
                    println!("client conneciton wasnt found for player id {}", player_id);
                }
            }
        });
    }

    pub async fn enter_map(
        handler: Arc<RwLock<Self>>,
        id: u32,
        player: &Player,
        act_id: u32,
        db_id: u32,
        world_id: u32,
        map: String,
        map_copy_no: i32,
        x: u32,
        y: u32,
        enter_type: u8,
        s_winer: u16,
    ) -> anyhow::Result<()> {
        let enter_map_packet = GameServerEnterMapPacket::new(
            act_id,
            player.get_password(),
            db_id,
            world_id,
            map,
            map_copy_no,
            x,
            y,
            enter_type,
            id,
            s_winer,
        )
        .to_base_packet()
        .unwrap();

        let readable_handler = handler.read().await;
        let connection = readable_handler.connection.clone();
        let connection = connection.read().await;

        connection.send_data(enter_map_packet).await
    }

    pub fn on_enter_map_result(handler: Arc<RwLock<Self>>, packet: BasePacket) {
        tokio::spawn(async move {
            let readable_handler = handler.clone();
            let readable_handler = readable_handler.read().await;
            let player_list = readable_handler.client_list.clone();
            let gate_server_comm_tx = readable_handler.gate_comm_tx.clone().unwrap();
            let conn = readable_handler.connection.clone();
            let conn = conn.read().await;

            let client_pkt =
                GameServerEnterMapResultPacket::from_base_packet(packet.clone()).unwrap();
            conn.logger
                .debug(&format!("Enter map result: {:?}", client_pkt));
            if client_pkt.ret_code == 0 {
                let game_addr = client_pkt.gm_addr.unwrap();
                gate_server_comm_tx
                    .send(GameServerGateCommands::SendPacketToClient(
                        client_pkt.player_id,
                        GameServerEnterMapResultPacket::for_client(packet),
                    ))
                    .await
                    .unwrap();

                let player_list = player_list.write().await;
                if let Some(connection) = player_list.get(&client_pkt.player_id) {
                    let mut connection = connection.write().await;
                    let player = connection.get_application_context_mut().unwrap();
                    player.set_current_game_server(handler.clone(), game_addr);

                    let mut enter_map_pkt_for_group = BasePacket::new();
                    enter_map_pkt_for_group
                        .write_cmd(Command::GMTGPPlayerEnterMap)
                        .unwrap();
                    enter_map_pkt_for_group
                        .write_char(client_pkt.is_switch.unwrap())
                        .unwrap();
                    enter_map_pkt_for_group
                        .write_long(client_pkt.player_id)
                        .unwrap();
                    enter_map_pkt_for_group
                        .write_long(player.get_group_addr())
                        .unwrap();
                    enter_map_pkt_for_group.build_packet().unwrap();

                    let _ = gate_server_comm_tx
                        .send(GameServerGateCommands::SendPacketToGroupServer(
                            enter_map_pkt_for_group,
                        ))
                        .await;
                }
            }
        });
    }

    pub fn on_map_entry(handler: Arc<RwLock<Self>>, mut packet: BasePacket) {
        tokio::spawn(async move {
            let map_name = packet.read_string().unwrap();
            let handler = handler.read().await;
            let game_server_list = handler.game_server_list.clone();
            let game_server_list = game_server_list.read().await;

            if let Some(game_server_with_map) =
                game_server_list.find_game_server_with_map(map_name).await
            {
                let mut pkt = packet.duplicate();
                pkt.write_cmd(Command::GTTGMMapEntry).unwrap();
                pkt.build_packet().unwrap();

                let game_server = game_server_with_map.read().await;
                let connection = game_server.get_connection();
                let connection = connection.read().await;

                connection.send_data(pkt).await.unwrap();
            }
        });
    }

    pub async fn reject_connection(handler: Arc<RwLock<Self>>, reason: GameServerRejectionReason) {}

    pub async fn is_connected(&self) -> bool {
        let connection = self.connection.read().await;
        connection.is_ready()
    }

    pub async fn send_packet(&self, packet: BasePacket) {
        let connection = self.connection.read().await;
        if connection.send_data(packet).await.is_err() {
            connection
                .logger
                .error("Failed to send to GameServer, connection was closed");
        }
    }

    pub fn get_connection(&self) -> Arc<RwLock<TcpConnection<GameServer>>> {
        self.connection.clone()
    }
}
