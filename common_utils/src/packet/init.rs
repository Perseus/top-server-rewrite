use super::{BasePacket, PacketWriter, TypedPacket};

pub struct InitClientPacket {
    pub chapstr: String,
}

impl InitClientPacket {
    pub fn new(chapstr: String) -> Self {
        InitClientPacket { chapstr }
    }
}

impl TypedPacket for InitClientPacket {
    fn from(base_packet: BasePacket) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(InitClientPacket {
            chapstr: "".to_string(),
        })
    }

    fn to_base_packet(&self) -> anyhow::Result<BasePacket> {
        let mut base_packet = BasePacket::new();
        base_packet.write_cmd(super::Command::GTTCEstablishConnection)?;
        base_packet.write_string(self.chapstr.as_str())?;

        base_packet.build_packet()?;

        Ok(base_packet)
    }
}
