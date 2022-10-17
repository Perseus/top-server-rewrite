use super::PacketData;

pub struct TestCommandPacket {
}

impl PacketData for TestCommandPacket {
  fn new() -> Self {
    TestCommandPacket {  }
  }
}