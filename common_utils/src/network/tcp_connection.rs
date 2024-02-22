use bytes::{Buf, BytesMut};
use colored::Color;
use std::borrow::Borrow;
use std::mem;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::select;
use tokio::time::interval;
use tokio::{io::AsyncReadExt, time::Instant};
use tokio_util::sync::CancellationToken;

use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex};

use crate::logger::{self, Logger};
use crate::packet::heartbeat::{self, HeartbeatPacket};
use crate::packet::{BasePacket, PacketWriter, TypedPacket};

use super::rpc::{self, RPCManager};

#[derive(Debug)]
pub struct TcpConnection<T> {
    id: u32,
    connection_type: String,
    stream: Option<TcpStream>,
    socket_addr: SocketAddr,
    application_context: Option<T>,
    recv_buffer: BytesMut,
    last_received_data_at: Arc<Mutex<Instant>>,
    send_channel: Option<mpsc::Sender<BasePacket>>,

    rpc_reply_recv_tx: mpsc::Sender<BasePacket>,
    rpc_reply_recv_rx: mpsc::Receiver<BasePacket>,

    rpc_mgr: Option<Arc<Mutex<RPCManager>>>,
    cancellation_token: CancellationToken,
    is_ready: bool,
    pub logger: Logger,
}

const MAX_PACKET_SIZE: usize = 8192;

impl<T> TcpConnection<T> {
    pub fn new(
        id: u32,
        connection_type: String,
        stream: TcpStream,
        socket_addr: SocketAddr,
        logger_prefix: String,
        logger_color: Color,
    ) -> Self {
        let rpc_send_channel = mpsc::channel::<BasePacket>(100);
        let rpc_reply_recv_channel = mpsc::channel::<BasePacket>(100);

        TcpConnection {
            id,
            connection_type,
            socket_addr,
            stream: Some(stream),
            application_context: None,
            recv_buffer: BytesMut::with_capacity(MAX_PACKET_SIZE),
            last_received_data_at: Arc::new(Mutex::new(Instant::now())),
            send_channel: None,
            rpc_reply_recv_tx: rpc_reply_recv_channel.0,
            rpc_reply_recv_rx: rpc_reply_recv_channel.1,
            rpc_mgr: None,
            cancellation_token: CancellationToken::new(),
            is_ready: false,
            logger: Logger::new(logger_color, logger_prefix, false),
        }
    }

    pub fn get_socket_addr(&self) -> &SocketAddr {
        &self.socket_addr
    }

    pub fn get_ip_as_u32(&self) -> anyhow::Result<u32> {
        if let IpAddr::V4(ip) = self.socket_addr.ip() {
            let ip_as_u32 = u32::from_le_bytes(ip.octets());
            Ok(ip_as_u32)
        } else {
            Err(anyhow::anyhow!("Not an IPv4 address"))
        }
    }

    fn heartbeat_handler(&self, data_send_channel_producer: mpsc::Sender<BasePacket>) {
        let last_received_data_at = self.last_received_data_at.clone();
        let cancellation_token = self.cancellation_token.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(100));
            loop {
                select! {
                    _ = interval.tick() => {
                        let last_received_data_at = last_received_data_at.lock().await;
                        if last_received_data_at.elapsed().as_secs() >= 2 {
                            let heartbeat_packet = HeartbeatPacket::default();
                            let base_packet: BasePacket = heartbeat_packet.to_base_packet().unwrap();
                            if let Err(err) = data_send_channel_producer.send(base_packet).await {
                                println!("Error sending heartbeat ping - {:?}", err);
                                break;
                            }
                        }
                    },

                    _ = cancellation_token.cancelled() => {
                        break;
                    }
                }
            }
        });
    }

    pub async fn start_processing(
        &mut self,
        data_recv_tx: mpsc::Sender<BasePacket>,
        mut data_send_rx: mpsc::Receiver<BasePacket>,
        data_send_tx: mpsc::Sender<BasePacket>,
    ) {
        self.logger.debug(&format!(
            "Starting to process connection for {}.{}",
            self.connection_type, self.id
        ));
        self.rpc_mgr = Some(Arc::new(Mutex::new(RPCManager::new(data_send_tx.clone()))));

        let stream = self.stream.take();
        let (mut reader, writer) = stream.unwrap().into_split();
        let mut cancellation_token = self.cancellation_token.clone();

        let logger = self.logger.clone();
        tokio::spawn(async move {
            let mut stream_writer = writer;
            loop {
                select!(
                    recv_result = data_send_rx.recv() => {
                        if recv_result.is_some() {
                            let packet = recv_result.unwrap();
                            let packet_as_vec = packet.get_as_bytes().to_vec();
                            let packet_as_slice = packet_as_vec.as_slice();

                            if let Ok(num_bytes_written) = stream_writer.try_write(packet_as_slice){
                                stream_writer.flush().await.unwrap();
                                if num_bytes_written > 2 {
                                    logger.debug("Sent packet to connection");
                                }
                            } else {
                                println!("Error writing to stream");
                            }
                        } else {
                            println!("Data send channel closed");
                            break;
                        }
                    },

                    _ = cancellation_token.cancelled() => {
                        break;
                    }
                );
            }
        });

        self.send_channel = Some(data_send_tx.clone());
        self.heartbeat_handler(data_send_tx);
        let last_received_data_at = self.last_received_data_at.clone();
        let rpc_mgr = self.rpc_mgr.clone().unwrap();
        cancellation_token = self.cancellation_token.clone();

        let logger = self.logger.clone();
        tokio::spawn(async move {
            let mut buffer = BytesMut::with_capacity(MAX_PACKET_SIZE);
            let logger = logger;
            loop {
                select!(
                    recv_result = reader.read_buf(&mut buffer) => {
                        match recv_result {
                            Ok(0) => {
                                cancellation_token.cancel();
                                break;
                            }

                            Ok(n) => {
                                let mut last_received_data_at = last_received_data_at.lock().await;
                                *last_received_data_at = Instant::now();

                                while let Ok(n) = BasePacket::check_frame(&buffer, false) {
                                    if n == 2 {
                                        buffer.advance(n);
                                        continue;
                                    }


                                    let packet = BasePacket::parse_frame(&mut buffer, n).unwrap();
                                    let mut rpc_mgr = rpc_mgr.lock().await;

                                    let is_for_rpc = rpc_mgr.check_packet_for_rpc_response(&packet).await;
                                    if is_for_rpc {
                                        if let Ok(()) = rpc_mgr.handle_rpc_reply(packet).await {
                                            buffer.advance(n);
                                            continue;
                                        } else {
                                            break;
                                        }
                                    }

                                    buffer.advance(n);
                                    data_recv_tx.send(packet).await.unwrap();
                                }
                            }

                            Err(err) => panic!("{:?}", err),
                        }
                    },

                    _ = cancellation_token.cancelled() => {
                        break;
                    }
                );
            }
        });
    }

    pub fn set_application_context(&mut self, context: T) {
        self.application_context = Some(context);
    }

    pub fn unset_application_context(&mut self) {
        self.application_context = None;
    }

    pub fn get_application_context(&self) -> Option<&T> {
        self.application_context.as_ref()
    }

    pub fn get_application_context_mut(&mut self) -> Option<&mut T> {
        self.application_context.as_mut()
    }

    pub fn get_id(&self) -> u32 {
        self.id
    }

    pub async fn send_data(&self, packet: BasePacket) -> anyhow::Result<()> {
        if let Some(send_channel) = &self.send_channel {
            send_channel.send(packet).await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("No send channel"))
        }
    }

    pub async fn close(&mut self, delay_in_ms: Duration) {
        tokio::time::sleep(delay_in_ms).await;
        self.cancellation_token.cancel();
    }

    pub fn mark_connected(&mut self) {
        self.logger.info("Marking connection as connected");
        self.is_ready = true;
    }

    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    pub async fn sync_rpc(&self, packet: BasePacket) -> anyhow::Result<BasePacket> {
        if let Some(mut rpc_mgr) = self.rpc_mgr.clone() {
            rpc_mgr = rpc_mgr.clone();
            // every packet gets a "SESSIONID" appended to it
            // on a packet being received, we look for a "SESSIONID"
            // if that exists, and there is a corresponding "SESSIONID" in our list of RPCs
            // we resolve the RPC with the packet

            let rpc_call = rpc_mgr.lock().await.make_sync_call(packet).await?;
            rpc_call.wait_for_result(Duration::from_secs(30)).await?;

            if let Some(result) = rpc_call.get_result().await {
                self.logger
                    .debug(format!("[sess_id={}] RPC Result", rpc_call.get_session_id()).as_str());
                result.inspect_with_logger(&self.logger);
                Ok(result)
            } else {
                Err(anyhow::anyhow!("No result"))
            }
        } else {
            Err(anyhow::anyhow!("No RPC manager"))
        }
    }
}
