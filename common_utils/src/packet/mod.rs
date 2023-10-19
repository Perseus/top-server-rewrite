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
  use parser::{de::from_bytes};
  use super::*;

  fn get_test_packet() -> Vec<u8> {
    let packet_as_bytes = hex::decode("020000004500008831714000800600007f0000017f000001c42507b5f43d914c62c3d4a65018ffb8b6ac000000608000000001af0000000000076e6f62696c6c00000661646d696e000018dbe3d3bc096401abde005ccf5dc062c642a3d8b12d0f04930021303030303030303030303030303031304130373532453443323333394400039400887d2c");
    packet_as_bytes.unwrap()
  }


  /**
   * Login packet structure
   0000   02 00 00 00 45 00 00 88 31 71 40 00 80 06 00 00
  0010   7f 00 00 01 7f 00 00 01 c4 25 07 b5 f4 3d 91 4c
  0020   62 c3 d4 a6 50 18 ff b8 b6 ac 00 00 00 60 80 00
  0030   00 00 01 af 00 00 00 00 00 07 6e 6f 62 69 6c 6c
  0040   00 00 06 61 64 6d 69 6e 00 00 18 db e3 d3 bc 09
  0050   64 01 ab de 00 5c cf 5d c0 62 c6 42 a3 d8 b1 2d
  0060   0f 04 93 00 21 30 30 30 30 30 30 30 30 30 30 30
  0070   30 30 30 30 31 30 30 41 30 37 35 32 30 45 34 43
  0080   32 33 33 39 44 00 03 94 00 88 7d 2c


   * - Command (u16)
   * - "passport" (string)
   * - Account Name (string)
   * - Password (encrypted string - custom encryption algo?)
   * - Mac Address (full mac or 'unknown' if not found)
   * - Secret Key
   * - Client Version
   */
  #[derive(Deserialize, Debug)]
  struct LoginPacket {
    cmd: u16,
    passport: String,
    account_name: String,
    password: String,
    mac_address: String,
    secret_key: u16,
    client_version: u16
  }

  #[test]
  fn it_creates_a_packet_from_bytes() {
      let data = get_test_packet();
      let bytes_clone = data.clone();
      let packet: LoginPacket = from_bytes(&data).unwrap();

      println!("{:?}", packet);

  }
}