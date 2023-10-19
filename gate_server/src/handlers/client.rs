use tokio::{net::TcpStream, io::{AsyncRead, AsyncWrite, Ready, self, AsyncWriteExt}};
use super::super::entities::player::{Player};

pub struct Client {
  socket: TcpStream,
  player: Option<Player>
}


impl Client {
  pub fn new(socket: TcpStream) -> Self {
    Client {
      socket,
      player: None
    }
  }

  async fn on_connect(&mut self) -> anyhow::Result<()> {
    // TODO: Handle max connections & metrics
    
    Ok(())
  }
  
  /**
   * Client connection flow
   * 
   * - Client initiates TCP connection (usual handshake happens)
   * - Server sends an intialization packet
   */
  async fn on_connected(&mut self) -> anyhow::Result<()> {
    self.player = Some(Player::new());

    Ok(())
  }

  pub async fn start_receiving_data(&mut self) -> anyhow::Result<()> {
    self.on_connect().await?;
    self.socket.readable().await?;
    self.on_connected().await?;

    let mut buf: [u8; 1024] = [0; 1024];

    loop {

      match self.socket.try_read(&mut buf) {
        Ok(0) => break,
        Ok(n) => {
          if n == 2 {
            self.socket.write_all(&[0, 2]).await?;
            self.socket.flush().await?;
            println!("Wrote to socket");
          }
          println!("read {} bytes - {:?}",  buf.len(), buf);
        },
        Err(ref e) if (e.kind() == io::ErrorKind::WouldBlock) => {
          continue;
        }
        Err(e) => {
          return Err(e.into());
        }
      }
    }

    Ok(())
  }
}