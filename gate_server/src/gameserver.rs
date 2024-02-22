use std::sync::Arc;

use common_utils::packet::BasePacket;
use tokio::{sync::RwLock, task::JoinSet};

use crate::gameserver_handler::{GameServerHandler, GameServerRejectionReason};

#[derive(Debug)]
pub struct GameServerList(Vec<Arc<RwLock<GameServerHandler>>>);

impl GameServerList {
    pub fn new() -> Self {
        GameServerList(Vec::new())
    }

    pub async fn check_for_duplicate_name_or_overlapping_maps(
        &self,
        game_server: &GameServer,
    ) -> Result<(), GameServerRejectionReason> {
        for server_handler in self.0.iter() {
            let readable_handler = server_handler.read().await;
            let server_connection = readable_handler.get_connection();
            let locked_server_connection = server_connection.read().await;
            let server_data = locked_server_connection.get_application_context().unwrap();

            if server_data.name == game_server.name {
                return Err(GameServerRejectionReason::DuplicateName);
            }

            for map in game_server.map_list.iter() {
                if server_data.map_list.contains(map) {
                    return Err(GameServerRejectionReason::OverlappingMapList);
                }
            }
        }

        Ok(())
    }

    pub async fn add_game_server(&mut self, handler: Arc<RwLock<GameServerHandler>>) {
        self.0.push(handler);
    }

    pub async fn find_game_server_with_map(
        &self,
        map_name: String,
    ) -> Option<Arc<RwLock<GameServerHandler>>> {
        for server_handler in self.0.iter() {
            let readable_handler = server_handler.read().await;
            let server_connection = readable_handler.get_connection();
            let locked_server_connection = server_connection.read().await;
            let server_data = locked_server_connection.get_application_context().unwrap();

            println!("Map list - {:?}", server_data.map_list);
            if server_data.map_list.contains(&map_name) {
                return Some(server_handler.clone());
            }
        }

        None
    }

    pub async fn send_to_all_game_servers_sync(
        &self,
        packet: BasePacket,
    ) -> anyhow::Result<Vec<BasePacket>> {
        let mut responses = Vec::new();
        let mut handles = JoinSet::new();
        for server_handler in self.0.iter() {
            let handler = server_handler.clone();
            let packet = packet.clone();
            let readable_handler = handler.read().await;
            let server_connection = readable_handler.get_connection().clone();

            handles.spawn(async move {
                let locked_server_connection = server_connection.read().await;
                locked_server_connection.sync_rpc(packet).await
            });
        }

        while let Some(result) = handles.join_next().await {
            if result.is_err() {
                return Err(anyhow::anyhow!(
                    "Something went wrong while trying to send an RPC to a GameServer"
                )
                .context(result.err().unwrap()));
            }

            let rpc_response = result.unwrap();
            if rpc_response.is_err() {
                return Err(anyhow::anyhow!("GameServer RPC response was an error")
                    .context(rpc_response.err().unwrap()));
            }

            responses.push(rpc_response.unwrap());
        }

        Ok(responses)
    }
}

#[derive(Debug)]
pub struct GameServer {
    pub name: String,
    pub map_list: Vec<String>,
}

impl GameServer {
    pub fn new() -> Self {
        GameServer {
            name: String::new(),
            map_list: Vec::new(),
        }
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn set_map_list(&mut self, map_list: Vec<String>) {
        self.map_list = map_list;
    }
}
