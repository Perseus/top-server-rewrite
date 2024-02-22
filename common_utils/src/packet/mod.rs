pub mod auth;
pub mod commands;
pub mod heartbeat;
pub mod init;

use byteorder::{BigEndian, ByteOrder, LittleEndian, ReadBytesExt};
use bytes::{Buf, BufMut, BytesMut};
use num_enum::IntoPrimitive;
use num_enum::TryFromPrimitive;
use std::fmt::Write;
use std::io::Cursor;
use std::time::Duration;
use tokio::time::Instant;

use std::iter::FromIterator;
use std::mem::size_of;

use commands::*;

use crate::logger::Logger;
use crate::network::rpc::MAX_SESS_ID_VALUE;

const DEFAULT_HEADER: u32 = 0x80000000;

#[derive(Debug, PartialEq)]
pub enum FrameError {
    Incomplete,
    Invalid,
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
    fn read_session_id(&self) -> Option<u32>;
    fn reverse_read_char(&mut self) -> Option<u8>;
    fn reverse_read_short(&mut self) -> Option<u16>;
    fn reverse_read_long(&mut self) -> Option<u32>;
    fn reverse_read_bytes(&mut self, num_bytes: usize) -> Option<&[u8]>;
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
    fn write_session_id(&mut self, session_id: u32) -> anyhow::Result<()>;
    fn build_packet(&mut self) -> anyhow::Result<()>;
}

pub trait TypedPacket {
    fn from_base_packet(base_packet: BasePacket) -> anyhow::Result<Self>
    where
        Self: Sized;

    fn to_base_packet(&self) -> anyhow::Result<BasePacket>;
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
pub struct BasePacket {
    data: BytesMut,
    cmd: Command,
    raw_cmd: u16,
    size: u16,
    offset: u8,
    header: u32,
    reverse_offset: u8,
    tick_count: Instant,
    is_for_rpc: bool,
    session_id: Option<u32>,
    is_built: bool,
}

impl BasePacket {
    pub fn new() -> Self {
        // the offset is 4 since 4 bytes of the header are already consumed to identify the entire packet frame
        BasePacket {
            data: BytesMut::with_capacity(64),
            cmd: Command::None,
            raw_cmd: 0,
            size: 0,
            offset: 4,
            header: DEFAULT_HEADER,
            reverse_offset: 0,
            tick_count: Instant::now(),
            is_for_rpc: false,
            session_id: None,
            is_built: false,
        }
    }

    pub fn from_bytes(data: BytesMut) -> Self {
        let mut packet = BasePacket {
            data,
            cmd: Command::None,
            raw_cmd: 0,
            size: 0,
            offset: 0,
            header: 0,
            reverse_offset: 0,
            tick_count: Instant::now(),
            is_for_rpc: false,
            session_id: None,
            is_built: true,
        };

        if let Some(size) = packet.read_short() {
            packet.size = size;
        }

        if let Some(header) = packet.read_long() {
            packet.header = header;
            packet.session_id = Some(header);
        }

        if let Some(cmd) = packet.read_cmd() {
            packet.cmd = cmd;
        }

        packet
    }

    pub fn as_bytes(self) -> BytesMut {
        self.data
    }

    pub fn get_time_since_creation(&self) -> Duration {
        self.tick_count.elapsed()
    }

    pub fn is_for_rpc(&self) -> bool {
        self.is_for_rpc
    }

    pub fn get_len(&self) -> u16 {
        self.size
    }

    pub fn set_len(&mut self, len: u16) {
        self.size = len
    }

    pub fn get_session_id(&self) -> Option<u32> {
        self.session_id
    }

    pub fn get_remaining_packet_len(&self) -> usize {
        let offset = self.offset as usize;
        self.data.len() - offset
    }

    pub fn has_data(&self) -> bool {
        self.data.len() > (2 + 4 + 2)
    }

    fn get_total_packet_len(&self) -> usize {
        self.data.len()
    }

    fn increment_offset<T>(&mut self) {
        self.offset += size_of::<T>() as u8;
    }

    fn increment_reverse_offset<T>(&mut self) {
        self.reverse_offset += size_of::<T>() as u8;
    }

    fn has_enough_bytes_for_data<T>(&mut self) -> bool {
        let packet_len = self.get_remaining_packet_len();
        if packet_len < size_of::<T>() {
            return false;
        }

        true
    }

    pub fn get_as_bytes(self) -> BytesMut {
        self.data
    }

    pub fn duplicate(&self) -> Self {
        let mut packet = self.clone();
        packet.offset = 8; // size, header data and command have already been read
        packet.reverse_offset = 0;

        packet
    }

    pub fn check_frame(buffer: &BytesMut, could_be_rpc: bool) -> Result<usize, FrameError> {
        // if we don't have at least enough data for a "packet length" frame, we return false
        let packet_length = buffer
            .get(0..2)
            .ok_or(FrameError::Incomplete)?
            .read_u16::<BigEndian>()
            .map_err(|_| FrameError::Invalid)?;

        if buffer.remaining() < (packet_length as usize - 2) {
            return Err(FrameError::Incomplete);
        }

        Ok(packet_length as usize)
    }

    pub fn discard_last(&mut self, len: usize) {
        self.data.truncate(self.data.len() - len);
        self.set_len(self.data.len() as u16);
        if len > self.reverse_offset as usize {
            self.reverse_offset = 0;
        } else {
            self.reverse_offset -= len as u8;
        }
    }

    /// Removes all bytes in the range from the buffer
    /// RESETS THE OFFSET
    pub fn remove_range(&mut self, range: std::ops::Range<usize>) {
        let mut remaining_bytes = self.data.split_off(range.start);
        let end_bytes = remaining_bytes.split_off(range.end - range.start);
        self.data.unsplit(end_bytes);
        self.offset = 0;

        self.set_len(self.data.len() as u16);
    }

    // this function assumes that the data in the buffer has been previously checked by the `check_frame` function
    // never use this without using check_frame
    pub fn parse_frame(buffer: &mut BytesMut, len: usize) -> Result<Self, FrameError> {
        let final_buffer = BytesMut::from_iter(&buffer[0..len]);
        Ok(BasePacket::from_bytes(final_buffer))
    }

    pub fn reset_header(&mut self) {
        self.header = DEFAULT_HEADER;
        self.session_id = Some(DEFAULT_HEADER);
    }

    pub fn inspect_with_logger(&self, logger: &Logger) {
        logger.debug("------ Inspected Packet -------");
        logger.debug(format!("[Packet] Size: {}", self.size).as_str());
        logger.debug(format!("[Packet] Header: {:?}", self.data.get(2..6)).as_str());
        logger.debug(format!("[Packet] Command: {:?}", self.data.get(6..8)).as_str());
        logger.debug(format!("[Packet] Data: {:?}", self.data.get(8..self.size as usize)).as_str());
        logger.debug(format!("[Packet] Offset: {:?}", self.offset).as_str());
        logger.debug(format!("[Packet] Reverse Offset: {:?}", self.reverse_offset).as_str());
        logger.debug("-------------------------------");
    }

    pub fn inspect(&self) {
        println!("------ Inspected Packet -------");
        println!("[Packet] Size: {}", self.size);
        println!("[Packet] Header: {:?}", self.data.get(2..6));
        println!("[Packet] Command: {:?}", self.data.get(6..8));
        println!("[Packet] Data: {:?}", self.data.get(8..self.size as usize));
        println!("[Packet] Offset: {:?}", self.offset);
        println!("[Packet] Reverse Offset: {:?}", self.reverse_offset);
        println!("-------------------------------");
    }

    pub fn get_raw_cmd(&self) -> u16 {
        self.raw_cmd
    }
}

impl PacketReader for BasePacket {
    /// Returns a <Command> enum, reading the first 2 bytes of a packet (after the header)
    fn read_cmd(&mut self) -> Option<Command> {
        if self.cmd != Command::None || self.offset > 6 {
            return Some(self.cmd.clone());
        }

        let offset = self.offset as usize;
        if !self.has_enough_bytes_for_data::<u16>() {
            return None;
        }

        let data_len_to_read = size_of::<u16>();
        if let Some(mut command_data) = self.data.get(offset..offset + data_len_to_read) {
            let primitive_command = command_data.read_u16::<BigEndian>().ok()?;
            self.raw_cmd = primitive_command;
            if let Ok(command) = Command::try_from_primitive(primitive_command) {
                self.increment_offset::<u16>();
                self.cmd = command.clone();

                return Some(command);
            } else {
                println!("Unknown command -> {}", primitive_command);
                self.increment_offset::<u16>();
                self.cmd = Command::None;

                return Some(Command::None);
            }
        } else {
            println!("no cmd found");
        }

        None
    }

    fn read_char(&mut self) -> Option<u8> {
        let offset = self.offset as usize;
        if !self.has_enough_bytes_for_data::<u8>() {
            return None;
        }

        let data_len_to_read = size_of::<u8>();
        let mut data = self.data.get(offset..offset + data_len_to_read)?;
        let data_to_return = data.read_u8().ok()?;

        self.increment_offset::<u8>();

        Some(data_to_return)
    }

    fn read_short(&mut self) -> Option<u16> {
        let offset = self.offset as usize;
        if !self.has_enough_bytes_for_data::<u16>() {
            return None;
        }

        let data_len_to_read = size_of::<u16>();
        let mut data = self.data.get(offset..offset + data_len_to_read)?;
        let data_to_return = data.read_u16::<BigEndian>().ok()?;
        self.increment_offset::<u16>();

        Some(data_to_return)
    }

    fn read_long(&mut self) -> Option<u32> {
        let offset = self.offset as usize;
        if !self.has_enough_bytes_for_data::<u32>() {
            return None;
        }

        let data_len_to_read = size_of::<u32>();
        let mut data = self.data.get(offset..offset + data_len_to_read)?;
        let data_to_return = data.read_u32::<BigEndian>().ok()?;
        self.increment_offset::<u32>();

        Some(data_to_return)
    }

    fn read_long_long(&mut self) -> Option<u64> {
        let offset = self.offset as usize;
        if !self.has_enough_bytes_for_data::<u64>() {
            return None;
        }

        let data_len_to_read = size_of::<u64>();
        let mut data = self.data.get(offset..offset + data_len_to_read)?;
        let data_to_return = data.read_u64::<BigEndian>().ok()?;
        self.increment_offset::<u64>();

        Some(data_to_return)
    }

    fn read_float(&mut self) -> Option<f32> {
        let offset = self.offset as usize;
        if !self.has_enough_bytes_for_data::<f32>() {
            return None;
        }

        let data_len_to_read = size_of::<f32>();
        let mut data = self.data.get(offset..offset + data_len_to_read)?;
        let data_to_return = data.read_f32::<BigEndian>().ok()?;
        self.increment_offset::<f32>();

        Some(data_to_return)
    }

    fn read_sequence(&mut self) -> Option<&[u8]> {
        let offset = self.offset as usize;
        if !self.has_enough_bytes_for_data::<u16>() {
            return None;
        }

        let seq_len_data_len = size_of::<u16>();
        let sequence_length = self
            .data
            .get(offset..offset + seq_len_data_len)?
            .read_u16::<BigEndian>()
            .ok()? as usize;

        if self.get_remaining_packet_len() < sequence_length {
            return None;
        }

        let sequence_start_offset = offset + seq_len_data_len;

        // last byte is a null character for the sequence, we can ignore it
        let sequence = self
            .data
            .get(sequence_start_offset..sequence_start_offset + sequence_length - 1)?;
        self.offset += (seq_len_data_len + sequence_length) as u8;
        Some(sequence)
    }

    fn read_string(&mut self) -> Option<String> {
        let sequence = self.read_sequence()?;
        let buf: Vec<u8> = Vec::from(sequence);
        let string = String::from_utf8(buf).ok()?;

        Some(string)
    }

    /// Reads a character from the trailing end of a packet
    //
    /// Increments a "reverse" offset which tracks the data that has been previously returned from the end
    fn reverse_read_char(&mut self) -> Option<u8> {
        if self.get_total_packet_len() < self.reverse_offset as usize + size_of::<u8>() {
            return None;
        }

        self.increment_reverse_offset::<u8>();

        let start_index = self.get_total_packet_len() - self.reverse_offset as usize;
        let end_index = start_index + size_of::<u8>();

        let mut data = self.data.get(start_index..end_index)?;
        let data_to_return = data.read_u8().ok()?;

        Some(data_to_return)
    }

    /// Reads a short from the trailing end of a packet
    //
    /// Increments a "reverse" offset which tracks the data that has been previously returned from the end
    fn reverse_read_short(&mut self) -> Option<u16> {
        let data_size = size_of::<u16>();

        if self.get_total_packet_len() < (self.reverse_offset as usize + data_size) {
            return None;
        }

        self.increment_reverse_offset::<u16>();

        let start_index = self.get_total_packet_len() - self.reverse_offset as usize;
        let end_index = start_index + data_size;

        let mut data = self.data.get(start_index..end_index)?;
        let data_to_return = data.read_u16::<BigEndian>().ok()?;

        Some(data_to_return)
    }

    /// Reads a long from the trailing end of a packet
    //
    /// Increments a "reverse" offset which tracks the data that has been previously returned from the end
    fn reverse_read_long(&mut self) -> Option<u32> {
        let data_size = size_of::<u32>();

        if self.get_total_packet_len() < (self.reverse_offset as usize + data_size) {
            return None;
        }

        self.increment_reverse_offset::<u32>();

        let start_index = self.get_total_packet_len() - self.reverse_offset as usize;
        let end_index = start_index + data_size;

        let mut data = self.data.get(start_index..end_index)?;
        let data_to_return = data.read_u32::<BigEndian>().ok()?;

        Some(data_to_return)
    }

    /// Reads a set number of bytes from the trailing end of a packet
    ///
    /// Increments a "reverse" offset which tracks the data that has been previously returned from the end
    fn reverse_read_bytes(&mut self, num_bytes: usize) -> Option<&[u8]> {
        if self.get_total_packet_len() < (self.reverse_offset as usize + num_bytes) {
            return None;
        }

        for _ in 0..num_bytes {
            self.increment_reverse_offset::<u8>();
        }

        let end_index = self.get_total_packet_len() - self.reverse_offset as usize;
        let start_index = end_index - num_bytes;

        let data = self.data.get(start_index..end_index)?;

        Some(data)
    }

    fn read_session_id(&self) -> Option<u32> {
        // 2 bytes for the size
        let start_index = 2;

        // the session id can be found in the first 4 bytes of the header
        let end_index = 6;

        let mut data = self.data.get(start_index..end_index)?;
        if let Ok(mut session_id) = data.read_u32::<BigEndian>() {
            if session_id > MAX_SESS_ID_VALUE {
                session_id -= MAX_SESS_ID_VALUE;
            }

            return Some(session_id);
        }

        None
    }
}

impl PacketWriter for BasePacket {
    fn write_buffer(&mut self, mut buffer: Vec<u8>) -> anyhow::Result<()> {
        for _ in 0..buffer.len() {
            if let Some(el) = buffer.pop() {
                self.data.put_u8(el);
                self.size += 1;
            }
        }

        Ok(())
    }

    fn write_cmd(&mut self, cmd: Command) -> anyhow::Result<()> {
        let command: u16 = cmd.into();
        self.cmd = Command::try_from_primitive(command)?;
        self.raw_cmd = command;

        Ok(())
    }

    fn write_char(&mut self, char: u8) -> anyhow::Result<()> {
        self.write_buffer(vec![char])?;

        Ok(())
    }

    fn write_short(&mut self, data: u16) -> anyhow::Result<()> {
        let mut buf = vec![0; 2];

        LittleEndian::write_u16(&mut buf[..], data);

        self.write_buffer(buf)?;

        Ok(())
    }

    fn write_long(&mut self, data: u32) -> anyhow::Result<()> {
        let mut buf = vec![0; 4];

        LittleEndian::write_u32(&mut buf[..], data);
        self.write_buffer(buf)?;

        Ok(())
    }

    fn write_long_long(&mut self, data: u64) -> anyhow::Result<()> {
        let mut buf = vec![0; 8];

        LittleEndian::write_u64(&mut buf[..], data);
        self.write_buffer(buf)?;

        Ok(())
    }

    fn write_float(&mut self, data: f32) -> anyhow::Result<()> {
        let mut buf = vec![0; 4];

        LittleEndian::write_f32(&mut buf[..], data);
        self.write_buffer(buf)?;

        Ok(())
    }

    fn write_sequence(&mut self, sequence: &[u8], len: u16) -> anyhow::Result<()> {
        let mut buf = Vec::with_capacity(len as usize);

        self.write_short(len + 1)?;

        buf.push(b'\0');

        for i in 0..len as usize {
            if let Some(char) = sequence.get((len as usize) - i - 1) {
                buf.push(*char);
            }
        }

        self.write_buffer(buf)?;

        Ok(())
    }

    fn write_string(&mut self, string: &str) -> anyhow::Result<()> {
        self.write_sequence(string.as_bytes(), string.len() as u16)?;

        Ok(())
    }

    fn write_session_id(&mut self, session_id: u32) -> anyhow::Result<()> {
        self.session_id = Some(session_id);

        Ok(())
    }

    fn build_packet(&mut self) -> anyhow::Result<()> {
        if self.is_built {
            // delete bytes 0..8 because size, header and command are already written
            self.data = self.data.split_off(8);
        }
        let size = (self.data.len() + 4 + 2 + 2) as u16; // total data + header + length of the size data itself + length of command

        let size_buf = size.to_be_bytes().to_vec();
        let mut header_buf = self.header.to_be_bytes().to_vec();
        // replace the first N bytes of the header with the session_id if it exists
        if let Some(session_id) = self.session_id {
            let session_id_buf = session_id.to_be_bytes().to_vec();
            header_buf = session_id_buf;
        }

        let mut cmd_buf: Vec<u8> = vec![0; 2];
        BigEndian::write_u16(&mut cmd_buf[..], self.raw_cmd);

        let full_buf = [size_buf, header_buf, cmd_buf].concat();

        let mut byte_buffer = BytesMut::from_iter(full_buf);
        let empty_buffer = BytesMut::new();

        let existing_data = std::mem::replace(&mut self.data, empty_buffer);
        byte_buffer.unsplit(existing_data);

        self.data = byte_buffer;
        self.is_built = true;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::iter::FromIterator;

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
        let data = get_test_packet_one();
        let bytes_clone = data.clone();
        let packet = BasePacket::from_bytes(data);

        assert_eq!(packet.data, bytes_clone);
        assert_eq!(packet.size, 40);
    }

    #[test]
    fn it_reads_strings_correctly() {
        let data = get_test_packet_one();
        let mut packet = BasePacket::from_bytes(data);

        let _ = packet.read_cmd().unwrap();
        let char_name = packet.read_string().unwrap();
        let chat_channel = packet.read_string().unwrap();
        let chat_content = packet.read_string().unwrap();

        assert_eq!(char_name, "killt1");
        assert_eq!(chat_channel, "Local");
        assert_eq!(chat_content, "a");
    }

    #[test]
    fn it_reads_reverse_data_correctly() {
        let data = get_test_packet_one();
        let mut packet = BasePacket::from_bytes(data);

        let last_char = packet.reverse_read_char().unwrap();
        assert_eq!(last_char, 1);

        let last_short = packet.reverse_read_short().unwrap();
        assert_eq!(last_short, 28672);
    }

    #[test]
    fn it_creates_an_empty_packet() {
        let packet = BasePacket::new();
        assert_eq!(packet.data, BytesMut::new());
    }
}
