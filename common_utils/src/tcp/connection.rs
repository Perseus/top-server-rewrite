use tokio::net::TcpStream;

pub struct TcpConnection {
  stream: TcpStream
}

impl TcpConnection {
  pub fn new(stream: TcpStream) -> Self {
    TcpConnection { stream }
  }

  
}