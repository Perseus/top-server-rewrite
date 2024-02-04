use common_utils::{
    network::{tcp_connection::*, tcp_server::*},
    packet::*,
};
use tokio::sync::mpsc;

use crate::group_server::GroupServer;

pub struct GroupServerHandler {}

impl GroupServerHandler {
    pub fn new() -> Self {
        GroupServerHandler {}
    }
}

impl ServerHandler<GroupServer> for GroupServerHandler {
    fn on_connect(
        id: String,
        stream: tokio::net::TcpStream,
        socket: std::net::SocketAddr,
    ) -> TcpConnection<GroupServer> {
        todo!()
    }

    fn on_connected(
        connection: TcpConnection<GroupServer>,
    ) -> anyhow::Result<mpsc::Sender<BasePacket>> {
        todo!()
    }

    fn on_data(packet: BasePacket) {
        todo!()
    }
}
