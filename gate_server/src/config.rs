use std::{net::{Ipv4Addr}, path::Path};
use common_utils::parser::config_parser::ConfigParser;
use serde::Deserialize;


#[derive(Debug, Deserialize, PartialEq)]
struct GroupConfig {
  ip: Ipv4Addr,
  port: u16,
  ping_duration: u16
}

#[derive(Debug, Deserialize, PartialEq)]
struct ClientConfig {
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
struct GameServerConfig {
  ip: Ipv4Addr,
  port: u16,
  ping_duration: u16,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct GateServerConfig {
  group_config: GroupConfig,
  client_config: ClientConfig,
  game_server_config: GameServerConfig,
}

pub fn parse_config(path: &Path) -> GateServerConfig {
  let parser = ConfigParser::<GateServerConfig>::parse_file(path);

  parser.get_data()
}

mod tests {
    use std::{net::{SocketAddrV4, Ipv4Addr}, path::Path};

    use crate::config::{GroupConfig, GameServerConfig};

    use super::{parse_config, ClientConfig};

  #[test]
  fn it_should_parse_the_gateserver_config_correctly() {
    let data = parse_config(Path::new("./tests/test_gate_config.yaml"));
    
    assert_eq!(data.client_config, ClientConfig {
        ip: Ipv4Addr::new(127, 0, 0, 1),
        ddos_protection: true,
        max_connections: 500,
        max_login_per_ip: 50,
        ping_duration: 180,
        port: 3000,
        wpe_protection: true,
        wpe_version: 30
    });

    assert_eq!(data.group_config, GroupConfig {
      ip: Ipv4Addr::new(127, 0, 0, 1),
      ping_duration: 180,
      port: 3001
    });

    assert_eq!(data.game_server_config, GameServerConfig {
      ip: Ipv4Addr::new(127, 0, 0, 1),
      ping_duration: 180,
      port: 3002
    });
  }
}