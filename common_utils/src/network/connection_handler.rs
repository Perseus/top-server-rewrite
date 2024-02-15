use std::{net::SocketAddr, sync::Arc};

use tokio::{
    net::TcpStream,
    sync::{mpsc, Mutex},
};

use super::tcp_connection::TcpConnection;
use crate::packet::BasePacket;

pub trait ConnectionHandler<ApplicationContextType, CommandChannelType> {
    fn on_connect(
        id: String,
        stream: TcpStream,
        socket: SocketAddr,
    ) -> TcpConnection<ApplicationContextType>;
    fn on_connected(
        connection: Arc<Mutex<TcpConnection<ApplicationContextType>>>,
    ) -> anyhow::Result<()>;
    // fn on_disconnect(self);
    fn on_data(packet: BasePacket);
    // fn on_error(&mut self, stream: TcpListener, error: &str);
}
