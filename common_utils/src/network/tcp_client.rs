use std::{net::Ipv4Addr, net::SocketAddr, time::Duration};

use colored::Colorize;
use tokio::{net::TcpStream, sync::mpsc, time::sleep};

use super::{connection_handler::ConnectionHandler, tcp_connection::TcpConnection};

pub struct TcpClient {
    name: String,
    target_ip: Ipv4Addr,
    target_port: u16,
}

impl TcpClient {
    pub fn new(name: String, target_ip: Ipv4Addr, target_port: u16) -> TcpClient {
        TcpClient {
            name,
            target_ip,
            target_port,
        }
    }

    async fn handle_connection<HandlerType, ApplicationContextType, CommandChannelType>(
        name: String,
        stream: TcpStream,
        socket_addr: SocketAddr,
    ) -> anyhow::Result<TcpConnection<ApplicationContextType>>
    where
        HandlerType: ConnectionHandler<ApplicationContextType, CommandChannelType>,
    {
        let id = format!("{}.{}", name, nanoid::nanoid!(5)).to_string();
        Ok(HandlerType::on_connect(id.clone(), stream, socket_addr))
    }

    pub async fn connect<HandlerType, ApplicationContextType, CommandChannelType>(
        &mut self,
        retry_interval_in_secs: u64,
    ) -> anyhow::Result<TcpConnection<ApplicationContextType>>
    where
        HandlerType: ConnectionHandler<ApplicationContextType, CommandChannelType>,
    {
        loop {
            match TcpStream::connect(format!("{}:{}", self.target_ip, self.target_port)).await {
                Ok(stream) => {
                    let socket_addr = stream.peer_addr();
                    println!(
                        "{}",
                        format!("Connected to {} at IP - {}", self.name, self.target_ip).green()
                    );
                    let connection =
                        TcpClient::handle_connection::<
                            HandlerType,
                            ApplicationContextType,
                            CommandChannelType,
                        >(self.name.clone(), stream, socket_addr.unwrap())
                        .await?;

                    return Ok(connection);
                }
                Err(err) => {
                    println!(
                        "{}",
                        format!(
                            "Unable to connect to {} at IP - {}, Port - {}, retrying in {} seconds. Error - {}",
                            self.name, self.target_ip, self.target_port, retry_interval_in_secs, err
                        )
                        .red()
                    );

                    sleep(Duration::from_secs(retry_interval_in_secs)).await;
                }
            }
        }
    }
}
