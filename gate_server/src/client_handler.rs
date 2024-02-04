use common_utils::{
    network::{tcp_connection::TcpConnection, tcp_server::ServerHandler},
    packet::{
        commands::*, init::InitClientPacket, BasePacket, PacketReader, PacketWriter, TypedPacket,
    },
};
use tokio::sync::mpsc;

use crate::{
    gate_commands::GateCommands, gate_server::GateServer, metrics::CONNECTIONS_COUNTER,
    player::Player,
};

pub struct ClientHandler {
    gate_comm_channel: (mpsc::Sender<BasePacket>, mpsc::Receiver<BasePacket>),
}

impl ClientHandler {
    pub fn new(
        gate_comm_channel: (mpsc::Sender<BasePacket>, mpsc::Receiver<BasePacket>),
    ) -> ClientHandler {
        ClientHandler { gate_comm_channel }
    }
}

impl ServerHandler<Player> for ClientHandler {
    fn on_connect(
        id: String,
        stream: tokio::net::TcpStream,
        socket: std::net::SocketAddr,
    ) -> TcpConnection<Player> {
        let mut tcp_connection = TcpConnection::<Player>::new(id, stream, socket);

        CONNECTIONS_COUNTER.inc();

        // initialize the "player", which will contain any client-specific data
        let player = Player::new();
        tcp_connection.set_application_context(player);

        tcp_connection
    }

    fn on_connected(
        mut connection: TcpConnection<Player>,
    ) -> anyhow::Result<mpsc::Sender<BasePacket>> {
        let application_context = connection.get_application_context();
        if application_context.is_none() {
            return Err(anyhow::anyhow!("No application context").context("on_connected"));
        }

        let chapstr = application_context.unwrap().get_chapstr();
        let client_init_packet = InitClientPacket::new(chapstr).to_base_packet().unwrap();

        // channel where we get data from the client (recv)
        // separate thread pulling data from the client TCP connection, where a thread consumes it, converts it into a BasePacket, and then pushes it to recv_tx, and can be consumed through recv_rx
        let (recv_tx, mut recv_rx) = mpsc::channel(100);

        // channel used to send data to the client
        // push messages to _send_tx, they are received by a separate thread consuming from send_rx, and then sent to the client
        let (_send_tx, send_rx) = mpsc::channel(100);
        let channel_for_init_packet = _send_tx.clone();

        // send init packet, required for client to send auth info
        tokio::spawn(async move {
            channel_for_init_packet
                .send(client_init_packet)
                .await
                .unwrap();
        });

        let _send_tx_for_processor = _send_tx.clone();
        tokio::spawn(async move {
            connection
                .start_processing(recv_tx, send_rx, _send_tx_for_processor)
                .await;
        });

        tokio::spawn(async move {
            loop {
                match recv_rx.recv().await {
                    Some(packet) => ClientHandler::on_data(packet),
                    None => {
                        println!("Channel closed");
                        break;
                    }
                }
            }
        });

        let (gate_comm_tx, mut gate_comm_rx) = mpsc::channel::<BasePacket>(100);
        let channel_for_client_comm = _send_tx.clone();

        tokio::spawn(async move {
            loop {
                match gate_comm_rx.recv().await {
                    Some(packet) => {
                        channel_for_client_comm.send(packet).await.unwrap();
                    }
                    None => {
                        println!("Channel closed");
                        break;
                    }
                }
            }
        });

        return Ok(gate_comm_tx);
    }

    // fn on_disconnect(self) {
    //     println!("Client disconnected");
    //     todo!()
    // }

    fn on_data(mut packet: BasePacket) {
        match packet.read_cmd() {
            Some(Command::CTGt_Login) => {}

            _ => {
                todo!()
            }

            None => {
                todo!()
            }
        }
    }

    // fn on_error(&mut self, _stream: tokio::net::TcpListener, _error: &str) {
    //     todo!()
    // }
}

impl ClientHandler {
    async fn handle_user_login(&self, packet: BasePacket) {
        // let gate_command = GateCommands::SendToGroupServer(packet);
        // self.gate_comm_channel_tx.send(gate_command).await.unwrap();
    }
}
