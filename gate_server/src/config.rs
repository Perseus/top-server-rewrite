use common_utils::parser::config_parser::ConfigParser;
use serde::Deserialize;
use std::{net::Ipv4Addr, path::Path};

#[derive(Debug, Deserialize, PartialEq)]
pub struct GroupConfig {
    ip: Ipv4Addr,
    port: u16,
    ping_duration: u16,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct ClientConfig {
    ip: Ipv4Addr,
    port: u16,
    ping_duration: u16,
    max_connections: u32,
    wpe_protection: bool,
    wpe_version: u16,
    max_login_per_ip: u16,
    ddos_protection: bool,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct GameServerConfig {
    ip: Ipv4Addr,
    port: u16,
    ping_duration: u16,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct MainConfig {
    name: String,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct GateServerConfig {
    pub main: MainConfig,
    pub group_config: GroupConfig,
    pub client_config: ClientConfig,
    pub game_server_config: GameServerConfig,
}

impl GateServerConfig {
    pub fn get_server_ip_and_port_for_client(&self) -> (Ipv4Addr, u16) {
        (self.client_config.ip, self.client_config.port)
    }

    pub fn get_server_ip_and_port_for_group_server(&self) -> (Ipv4Addr, u16) {
        (self.group_config.ip, self.group_config.port)
    }

    pub fn get_ip_and_port_for_game_server(&self) -> (Ipv4Addr, u16) {
        (self.game_server_config.ip, self.game_server_config.port)
    }

    pub fn get_name(&self) -> String {
        self.main.name.clone()
    }
}

pub fn parse_config(path: &Path) -> GateServerConfig {
    let parser = ConfigParser::<GateServerConfig>::parse_file(path);

    parser.get_data()
}

mod tests {}
