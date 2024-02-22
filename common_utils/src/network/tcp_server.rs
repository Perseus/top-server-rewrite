use std::{net::SocketAddr, sync::Arc};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, Mutex},
    task::AbortHandle,
};

use crate::packet::BasePacket;

use super::{connection_handler::ConnectionHandler, tcp_connection::TcpConnection};

pub struct TcpServer {
    server: TcpListener,
}

impl TcpServer {
    pub async fn start(
        connect_addr: String,
        client_comm_channel_tracker_tx: mpsc::Sender<TcpStream>,
    ) -> anyhow::Result<()> {
        let server = match TcpListener::bind(&connect_addr).await {
            Ok(listener) => listener,
            Err(err) => panic!(
                "Unable to start the server at {}, error - {:?}",
                connect_addr, err
            ),
        };

        println!("started server at port at {:?}", connect_addr);

        loop {
            let (socket, socket_addr) = server.accept().await?;
            client_comm_channel_tracker_tx.send(socket).await.unwrap();
        }
    }
}
