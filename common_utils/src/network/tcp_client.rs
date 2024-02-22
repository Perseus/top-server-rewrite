use std::{net::Ipv4Addr, net::SocketAddr, sync::Arc, time::Duration};

use colored::Colorize;
use tokio::{
    net::{TcpSocket, TcpStream},
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

    pub async fn connect<HandlerType, ApplicationContextType, CommandChannelType>(
        &mut self,
        retry_interval_in_secs: u64,
    ) -> anyhow::Result<TcpStream>
    where
        HandlerType: ConnectionHandler<ApplicationContextType, CommandChannelType>,
    {
        loop {
            match TcpStream::connect(format!("{}:{}", self.target_ip, self.target_port)).await {
                Ok(stream) => {
                    println!(
                        "{}",
                        format!(
                            "Connected to {} at IP - {}",
                            self.server_type, self.target_ip
                        )
                        .green()
                    );

                    return Ok(stream);
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
