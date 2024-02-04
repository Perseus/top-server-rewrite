use std::net::SocketAddr;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc,
    task::AbortHandle,
};

use crate::packet::BasePacket;

use super::tcp_connection::TcpConnection;

pub struct TcpServer {
    server: TcpListener,
}
pub trait ServerHandler<ApplicationContextType> {
    fn on_connect(
        id: String,
        stream: TcpStream,
        socket: SocketAddr,
    ) -> TcpConnection<ApplicationContextType>;
    fn on_connected(
        connection: TcpConnection<ApplicationContextType>,
    ) -> anyhow::Result<mpsc::Sender<BasePacket>>;
    // fn on_disconnect(self);
    fn on_data(packet: BasePacket);
    // fn on_error(&mut self, stream: TcpListener, error: &str);
}

impl TcpServer {
    pub async fn start<HandlerType, ApplicationContextType>(
        connect_addr: String,
        client_comm_channel_tracker_tx: mpsc::Sender<(String, mpsc::Sender<BasePacket>)>,
    ) -> anyhow::Result<()>
    where
        HandlerType: ServerHandler<ApplicationContextType>,
    {
        let server = match TcpListener::bind(&connect_addr).await {
            Ok(listener) => listener,
            Err(err) => panic!(
                "Unable to start the server at {}, error - {:?}",
                connect_addr, err
            ),
        };

        loop {
            let (socket, socket_addr) = server.accept().await?;
            TcpServer::handle_new_connection::<HandlerType, ApplicationContextType>(
                socket,
                socket_addr,
                client_comm_channel_tracker_tx.clone(),
            )
            .await?;
        }
    }

    async fn handle_new_connection<HandlerType, ApplicationContextType>(
        socket: TcpStream,
        socket_addr: SocketAddr,
        client_comm_channel_tracker_tx: mpsc::Sender<(String, mpsc::Sender<BasePacket>)>,
    ) -> anyhow::Result<()>
    where
        HandlerType: ServerHandler<ApplicationContextType>,
    {
        let id = nanoid::nanoid!();
        let handler = HandlerType::on_connect(id.clone(), socket, socket_addr);
        if let Ok(client_comm_channel) = HandlerType::on_connected(handler) {
            client_comm_channel_tracker_tx
                .send((id, client_comm_channel))
                .await?;
        } else {
            // HandlerType::on_disconnect();
        }

        Ok(())
    }
}
