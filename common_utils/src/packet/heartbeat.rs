use super::*;
use anyhow::anyhow;

#[derive(Default)]
pub struct HeartbeatPacket {}

impl super::TypedPacket for HeartbeatPacket {
    fn from_base_packet(base_packet: super::BasePacket) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        if base_packet.get_len() != 2 {
            return Err(anyhow!("Invalid packet length for a heartbeat packet"));
        }

        Ok(HeartbeatPacket {})
    }

    // heartbeat packets are a special case of packets which have no command
    // only a length of 2 denoting the packet itself and nothing else
    fn to_base_packet(&self) -> anyhow::Result<super::BasePacket> {
        let mut base_packet = super::BasePacket::new();
        base_packet.write_short(2)?;

        Ok(base_packet)
    }
}
