use std::{net::Ipv4Addr, net::SocketAddr, sync::Arc, time::Duration};

use colored::Colorize;
use tokio::{
    net::TcpStream,
    sync::{mpsc, RwLock},
    time::sleep,
};

use super::{connection_handler::ConnectionHandler, tcp_connection::TcpConnection};

pub struct TcpClient {
    id: u32,
    server_type: String,
    target_ip: Ipv4Addr,
    target_port: u16,
}

impl TcpClient {
    pub fn new(id: u32, server_type: String, target_ip: Ipv4Addr, target_port: u16) -> TcpClient {
        TcpClient {
            id,
            server_type,
            target_ip,
            target_port,
        }
    }

    async fn handle_connection<HandlerType, ApplicationContextType, CommandChannelType>(
        id: u32,
        server_type: String,
        stream: TcpStream,
        socket_addr: SocketAddr,
    ) -> anyhow::Result<Arc<RwLock<HandlerType>>>
    where
        HandlerType: ConnectionHandler<ApplicationContextType, CommandChannelType>,
    {
        Ok(HandlerType::on_connect(
            id,
            server_type,
            stream,
            socket_addr,
        ))
    }

    pub async fn connect<HandlerType, ApplicationContextType, CommandChannelType>(
        &mut self,
        retry_interval_in_secs: u64,
    ) -> anyhow::Result<Arc<RwLock<HandlerType>>>
    where
        HandlerType: ConnectionHandler<ApplicationContextType, CommandChannelType>,
    {
        loop {
            match TcpStream::connect(format!("{}:{}", self.target_ip, self.target_port)).await {
                Ok(stream) => {
                    let socket_addr = stream.peer_addr();
                    println!(
                        "{}",
                        format!(
                            "Connected to {} at IP - {}",
                            self.server_type, self.target_ip
                        )
                        .green()
                    );
                    let connection = TcpClient::handle_connection::<
                        HandlerType,
                        ApplicationContextType,
                        CommandChannelType,
                    >(
                        self.id,
                        self.server_type.clone(),
                        stream,
                        socket_addr.unwrap(),
                    )
                    .await?;

                    return Ok(connection);
                }
                Err(err) => {
                    println!(
                        "{}",
                        format!(
                            "Unable to connect to {} at IP - {}, Port - {}, retrying in {} seconds. Error - {}",
                            self.server_type, self.target_ip, self.target_port, retry_interval_in_secs, err
                        )
                        .red()
                    );

                    sleep(Duration::from_secs(retry_interval_in_secs)).await;
                }
            }
        }
    }
}
