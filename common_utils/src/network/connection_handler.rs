use std::{net::SocketAddr, sync::Arc};

use async_trait::async_trait;
use tokio::{
    net::TcpStream,
    sync::{mpsc, Mutex, RwLock},
};

use crate::packet::BasePacket;

#[async_trait]
pub trait ConnectionHandler<ApplicationContextType, CommandChannelType> {
    fn on_connect(
        id: u32,
        connection_type: String,
        stream: TcpStream,
        socket: SocketAddr,
    ) -> Arc<RwLock<Self>>;
    async fn on_connected(
        handler: Arc<RwLock<Self>>,
        parent_comm_tx: mpsc::Sender<CommandChannelType>,
    ) -> anyhow::Result<()>;
    // fn on_disconnect(self);
    fn on_data(&self, packet: BasePacket);
    // fn on_error(&mut self, stream: TcpListener, error: &str);
}
