mod packets;
mod parser;

use byteorder::{BigEndian, ByteOrder, LittleEndian, ReadBytesExt};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use num_enum::IntoPrimitive;
use num_enum::TryFromPrimitive;
use std::iter::FromIterator;
use std::{io::Read, mem::size_of};
use std::fmt::Write;
use std::io::Cursor;
use std::io::Error;

use self::packets::TestCommandPacket;

#[derive(PartialEq, Debug, TryFromPrimitive, Clone, IntoPrimitive)]
#[repr(u16)]
pub enum Command {
    None,
    TestCommand = 1514,
    EstablishClientConnection = 940,
}

const DEFAULT_HEADER: u32 = 2147483648;

#[derive(Debug, PartialEq)]
pub enum FrameError {
    Incomplete,
    Invalid,
}

pub trait PacketData {
  fn new() -> Self;
}

/**
    The structure of a packet is ->
    First 2 bytes -> Length of the packet
    Next 4 bytes -> Header
    The first 6 bytes ^ are in BigEndian while everything else is in LittleEndian
    Next 2 bytes -> Command
    The rest of the bytes are datapoints related to the command in the form of
    different data types (strings, floats, longs etc)
*/
#[derive(Debug, Clone)]
pub struct Packet<T> {
    raw_data: BytesMut,
    data: T,
    cmd: Command,
    size: u16,
    offset: u8,
    header: u32,
    reverse_offset: u8,
}


pub trait PacketReader {
    fn read_cmd(&mut self) -> Option<Command>;
    fn read_char(&mut self) -> Option<u8>;
    fn read_short(&mut self) -> Option<u16>;
    fn read_long(&mut self) -> Option<u32>;
    fn read_long_long(&mut self) -> Option<u64>;
    fn read_sequence(&mut self) -> Option<&[u8]>;
    fn read_string(&mut self) -> Option<String>;
    fn read_float(&mut self) -> Option<f32>;
    fn reverse_read_char(&mut self) -> Option<u8>;
    fn reverse_read_short(&mut self) -> Option<u16>;
    fn reverse_read_long(&mut self) -> Option<u32>;
}

pub trait PacketWriter {
    fn write_buffer(&mut self, buffer: Vec<u8>) -> anyhow::Result<()>;
    fn write_cmd(&mut self, cmd: Command) -> anyhow::Result<()>;
    fn write_char(&mut self, char: u8) -> anyhow::Result<()>;
    fn write_short(&mut self, data: u16) -> anyhow::Result<()>;
    fn write_long(&mut self, data: u32) -> anyhow::Result<()>;
    fn write_long_long(&mut self, data: u64) -> anyhow::Result<()>;
    fn write_sequence(&mut self, sequence: &[u8], len: u16) -> anyhow::Result<()>;
    fn write_string(&mut self, string: &str) -> anyhow::Result<()>;
    fn write_float(&mut self, data: f32) -> anyhow::Result<()>;
    fn build_packet(&mut self) -> anyhow::Result<()>;
}

impl<PacketType> Packet<PacketType> {

  // pub fn new() -> Self {
  //   // the offset is 4 since 4 bytes of the header are already consumed to identify the entire packet frame
  //   Packet {
  //       data: BytesMut::with_capacity(64),
  //       cmd: Command::None,
  //       size: 0,
  //       offset: 4,
  //       header: DEFAULT_HEADER,
  //       reverse_offset: 0,
  //   }
  // }

}



#[cfg(test)]
mod test {
    use bincode::{Options, serialize};
    use bytes::BytesMut;
    use serde::{Serialize, Deserialize};

  fn get_test_packet() -> Vec<u8> {
    vec![
      0, 104, 128, 0, 0, 0
    ]
  }

  #[test]
  fn it_reads_the_packet_metadata_correctly() {
    let data = get_test_packet();


  }

  use std::iter::FromIterator;

  use crate::packet::packets::TestCommandPacket;

use super::*;

  fn get_test_packet_one() -> BytesMut {
      let data = BytesMut::from_iter(vec![
          0, 40, 128, 0, 0, 0, 5, 234, 0, 7, 107, 105, 108, 108, 116, 49, 0, 0, 6, 76, 111, 99,
          97, 108, 0, 0, 2, 97, 0, 99, 0, 0, 13, 67, 1, 57, 41, 112, 0, 1,
      ]);
      data
  }

  #[test]
  fn it_creates_a_packet_from_bytes() {
  }

//   #[test]
//   fn it_creates_a_packet_from_bytes() {
//       let data = get_test_packet_one();
//       let bytes_clone = data.clone();
//       let packet: Packet<TestCommandPacket> = Packet::from_bytes(TestCommandPacket {}, data);

//       assert_eq!(packet.raw_data, bytes_clone);
//       assert_eq!(packet.size, 40);
//   }

//   #[test]
//   fn it_reads_a_command_correctly() {
//       let data = get_test_packet_one();
//       let mut packet: Packet<TestCommandPacket> = Packet::from_bytes(TestCommandPacket {}, data);

//       let command = packet.read_cmd().unwrap();
//       assert_eq!(command, Command::TestCommand);
//   }

//   #[test]
//   fn it_reads_strings_correctly() {
//       let data = get_test_packet_one();
//       let mut packet: Packet<TestCommandPacket> = Packet::from_bytes(TestCommandPacket {}, data);

//       let _ = packet.read_cmd().unwrap();
//       let char_name = packet.read_string().unwrap();
//       let chat_channel = packet.read_string().unwrap();
//       let chat_content = packet.read_string().unwrap();

//       assert_eq!(char_name, "killt1");
//       assert_eq!(chat_channel, "Local");
//       assert_eq!(chat_content, "a");
//   }

//   #[test]
//   fn it_reads_reverse_data_correctly() {
//       let data = get_test_packet_one();
//       let mut packet: Packet<TestCommandPacket> = Packet::from_bytes(TestCommandPacket {}, data);

//       let last_char = packet.reverse_read_char().unwrap();
//       assert_eq!(last_char, 1);

//       let last_short = packet.reverse_read_short().unwrap();
//       assert_eq!(last_short, 28672);
//   }

//   // #[test]
//   // fn it_creates_an_empty_packet() {
//   //     let packet = Packet::new();
//   //     assert_eq!(packet.raw_data, BytesMut::new());
//   // }

//   // #[test]
//   // fn it_builds_a_packet_correctly() {
//   //     let mut w_packet = Packet::new();

//   //     w_packet.write_cmd(Command::TestCommand).unwrap();
//   //     w_packet.write_short(10).unwrap();
//   //     w_packet.write_long(200).unwrap();
//   //     w_packet.write_string("Hello").unwrap();
//   //     w_packet.write_char('A' as u8).unwrap();
//   //     w_packet.build_packet().unwrap();

//   //     let mut r_packet = w_packet.duplicate();
//   //     assert_eq!(r_packet.read_cmd().unwrap(), Command::TestCommand);
//   //     assert_eq!(r_packet.read_short().unwrap(), 10);
//   //     assert_eq!(r_packet.read_long().unwrap(), 200);
//   //     assert_eq!(r_packet.read_string().unwrap(), "Hello");
//   //     assert_eq!(r_packet.read_char().unwrap(), 'A' as u8);
//   // }
}