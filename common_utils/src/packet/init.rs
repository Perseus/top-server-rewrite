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

pub struct InitGroupPacket {
    version: u16,
    gate_server_name: String,
}

impl InitGroupPacket {
    pub fn new(version: u16, gate_server_name: String) -> Self {
        InitGroupPacket {
            version,
            gate_server_name,
        }
    }
}

impl TypedPacket for InitGroupPacket {
    fn from(base_packet: BasePacket) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(InitGroupPacket {
            version: 0,
            gate_server_name: "".to_string(),
        })
    }

    fn to_base_packet(&self) -> anyhow::Result<BasePacket> {
        let mut base_packet = BasePacket::new();
        base_packet.write_cmd(super::Command::GTTGPInit)?;
        base_packet.write_short(self.version)?;
        base_packet.write_string(self.gate_server_name.as_str())?;

        base_packet.build_packet()?;
        Ok(base_packet)
    }
}

/**
 * length - 25
 * header - 02 (sess_id)
 *
 * command - 2015
 *
 * (LONG) - PlayerCount
 * (STRING) - GateServer Name
 *
 * Array -
 *    (
 *          (LONG) - Player pointer addr in gateserver
 *          (LONG) - Player's login ID
 *          (LONG) - Player's account ID
 *    )
 */

pub struct PlayerForGroupServer {
    pub player_pointer: u32,
    pub login_id: u32,
    pub account_id: u32,
}
pub struct SyncPlayerListToGroupServerPacket {
    player_count: u32,
    gate_server_name: String,
    players: Vec<PlayerForGroupServer>,
}

impl SyncPlayerListToGroupServerPacket {
    pub fn new(
        player_count: u32,
        gate_server_name: String,
        players: Vec<PlayerForGroupServer>,
    ) -> Self {
        SyncPlayerListToGroupServerPacket {
            player_count,
            gate_server_name,
            players,
        }
    }
}

impl TypedPacket for SyncPlayerListToGroupServerPacket {
    fn from(base_packet: BasePacket) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(SyncPlayerListToGroupServerPacket {
            player_count: 0,
            gate_server_name: "".to_string(),
            players: vec![],
        })
    }

    fn to_base_packet(&self) -> anyhow::Result<BasePacket> {
        let mut base_packet = BasePacket::new();
        base_packet.write_cmd(super::Command::GTTGPSyncPlayerList)?;
        base_packet.write_long(self.player_count)?;
        base_packet.write_string(self.gate_server_name.as_str())?;

        for player in self.players.iter() {
            base_packet.write_long(player.player_pointer)?;
            base_packet.write_long(player.login_id)?;
            base_packet.write_long(player.account_id)?;
        }

        base_packet.build_packet()?;

        Ok(base_packet)
    }
}
