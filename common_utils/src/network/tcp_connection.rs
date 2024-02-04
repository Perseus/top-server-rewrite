use bytes::{Buf, BytesMut};
use std::mem;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tokio::{io::AsyncReadExt, time::Instant};

use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex};

use crate::packet::heartbeat::{self, HeartbeatPacket};
use crate::packet::{BasePacket, PacketWriter, TypedPacket};

pub struct TcpConnection<T> {
    id: String,
    stream: Option<TcpStream>,
    socket_addr: SocketAddr,
    application_context: Option<T>,
    recv_buffer: BytesMut,
    last_received_data_at: Arc<Mutex<Instant>>,
}

const MAX_PACKET_SIZE: usize = 4096;

impl<T> TcpConnection<T> {
    pub fn new(id: String, stream: TcpStream, socket_addr: SocketAddr) -> Self {
        let connection = TcpConnection {
            id,
            socket_addr,
            stream: Some(stream),
            application_context: None,
            recv_buffer: BytesMut::with_capacity(MAX_PACKET_SIZE),
            last_received_data_at: Arc::new(Mutex::new(Instant::now())),
        };

        connection
    }

    fn heartbeat_handler(&self, data_send_channel_producer: mpsc::Sender<BasePacket>) {
        let last_received_data_at = self.last_received_data_at.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(100));
            loop {
                interval.tick().await;
                let last_received_data_at = last_received_data_at.lock().await;
                if last_received_data_at.elapsed().as_secs() >= 2 {
                    let heartbeat_packet = HeartbeatPacket::default();
                    let base_packet: BasePacket = heartbeat_packet.to_base_packet().unwrap();
                    println!("Sending heartbeat ping");
                    data_send_channel_producer.send(base_packet).await;
                }
            }
        });
    }

    pub async fn start_processing(
        &mut self,
        _data_recv_channel_producer: mpsc::Sender<BasePacket>,
        mut data_send_channel_consumer: mpsc::Receiver<BasePacket>,
        mut data_send_channel_producer: mpsc::Sender<BasePacket>,
    ) {
        let stream = self.stream.take();
        let (mut reader, writer) = stream.unwrap().into_split();
        tokio::spawn(async move {
            let stream_writer = writer;
            loop {
                match data_send_channel_consumer.recv().await {
                    Some(packet) => {
                        let packet_as_vec = packet.get_as_bytes().to_vec();
                        let packet_as_slice = packet_as_vec.as_slice();

                        println!("Packet as slice {:?}", packet_as_slice);
                        let bytes_written = stream_writer.try_write(packet_as_slice);
                        println!("Bytes written - {:?}", bytes_written);
                    }
                    None => {}
                }
            }
        });

        self.heartbeat_handler(data_send_channel_producer);

        let mut buffer = BytesMut::with_capacity(MAX_PACKET_SIZE);
        loop {
            match reader.read_buf(&mut buffer).await {
                Ok(0) => {
                    println!("Connection closed by client.");
                    break;
                }

                Ok(n) => {
                    println!("Received {} bytes", n);
                    println!("Buffer: {:?}", buffer);
                    let mut last_received_data_at = self.last_received_data_at.lock().await;
                    *last_received_data_at = Instant::now();

                    match BasePacket::check_frame(&buffer) {
                        Err(crate::packet::FrameError::Incomplete) => {
                            println!("Incomplete frame");
                            continue;
                        }

                        Err(crate::packet::FrameError::Invalid) => {
                            println!("Invalid frame");
                            break;
                        }

                        Ok(n) => {
                            println!("Frame length - {}", n);
                            let packet = BasePacket::parse_frame(&mut buffer, n).unwrap();
                            println!("Packet - {:?}", packet);
                            if packet.get_len() == 2 {
                                println!("Received heartbeat ping");
                                buffer.advance(n);

                                continue;
                            }
                            _data_recv_channel_producer.send(packet).await.unwrap();
                            buffer.advance(n);
                        }
                    }
                }

                Err(err) => panic!("{:?}", err),
            }
        }
    }

    pub fn set_application_context(&mut self, context: T) {
        self.application_context = Some(context);
    }

    pub fn get_application_context(&self) -> Option<&T> {
        self.application_context.as_ref()
    }
}
