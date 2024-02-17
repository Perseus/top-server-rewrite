use super::{BasePacket, Command, ErrorCode, PacketWriter, TypedPacket};

pub struct ClientMapCrashPacket {
    msg: String,
}

impl ClientMapCrashPacket {
    pub fn new(msg: String) -> Self {
        Self { msg }
    }
}

impl TypedPacket for ClientMapCrashPacket {
    fn from(base_packet: super::BasePacket) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            msg: "ClientMapCrashPacket".to_string(),
        })
    }

    fn to_base_packet(&self) -> anyhow::Result<BasePacket> {
        let mut pkt = BasePacket::new();
        pkt.write_cmd(Command::GTTCMapCrash)?;
        pkt.write_string(&self.msg)?;

        pkt.build_packet()?;
        Ok(pkt)
    }
}

pub struct ClientLoginErrorPacket {
    error_code: ErrorCode,
}

impl ClientLoginErrorPacket {
    pub fn new(error_code: ErrorCode) -> Self {
        Self { error_code }
    }
}

impl TypedPacket for ClientLoginErrorPacket {
    fn from(base_packet: super::BasePacket) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            error_code: ErrorCode::ErrNetworkException,
        })
    }

    fn to_base_packet(&self) -> anyhow::Result<BasePacket> {
        let mut pkt = BasePacket::new();
        pkt.write_cmd(Command::GTTCLogin)?;

        if self.error_code != ErrorCode::None {
            pkt.write_short(self.error_code.clone() as u16)?;
        }

        pkt.build_packet()?;
        Ok(pkt)
    }
}

pub struct ClientDisconnectedForGroupPacket {
    account_id: u32,
    ip_addr: u32,
    reason: String,
}

impl ClientDisconnectedForGroupPacket {
    pub fn new(account_id: u32, ip_addr: u32, reason: String) -> Self {
        Self {
            account_id,
            ip_addr,
            reason,
        }
    }
}

impl TypedPacket for ClientDisconnectedForGroupPacket {
    fn from(base_packet: super::BasePacket) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            account_id: 0,
            ip_addr: 0,
            reason: "".to_string(),
        })
    }

    fn to_base_packet(&self) -> anyhow::Result<BasePacket> {
        let mut pkt = BasePacket::new();
        pkt.write_cmd(Command::GTTGPUserDisconnected)?;
        pkt.write_long(self.account_id)?;
        pkt.write_long(self.ip_addr)?;
        pkt.write_string(&self.reason)?;

        pkt.build_packet()?;
        Ok(pkt)
    }
}

pub struct ClientLogoutFromGroupPacket {
    player_id: u32,
    group_addr: u32,
}

impl ClientLogoutFromGroupPacket {
    pub fn new(player_id: u32, group_addr: u32) -> Self {
        Self {
            player_id,
            group_addr,
        }
    }
}

impl TypedPacket for ClientLogoutFromGroupPacket {
    fn from(base_packet: super::BasePacket) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            player_id: 0,
            group_addr: 0,
        })
    }

    fn to_base_packet(&self) -> anyhow::Result<BasePacket> {
        let mut pkt = BasePacket::new();
        pkt.write_cmd(Command::GTTGPUserLogout)?;
        pkt.write_long(self.player_id)?;
        pkt.write_long(self.group_addr)?;

        pkt.build_packet()?;
        Ok(pkt)
    }
}
