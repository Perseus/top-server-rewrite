use std::fmt::Display;

use num_enum::{IntoPrimitive, TryFromPrimitive};

#[derive(PartialEq, Debug, TryFromPrimitive, Clone, IntoPrimitive)]
#[repr(u16)]
pub enum Command {
    None,

    // Client to GateServer
    CTGTLogin = 431,
    CTGTSelectCharacter = 433,

    // GameServer to Client
    GMTCEnterMap = 516,

    // GateServer to Client
    GTTCEstablishConnection = 940,
    GTTCMapCrash = 503,
    GTTCLogin = 931,

    // GateServer to GroupServer
    GTTGPInit = 2001,
    GTTGPSyncPlayerList = 2015,
    GTTGPUserLogin = 2003,
    GTTGPUserLogout = 2004,
    GTTGPUserBeginPlay = 2005,
    GTTGPUserDisconnected = 2011,

    // GameServer to GateServer
    GMTGTInit = 1501,

    // GateServer to GameServer
    GTTGMInitAcknowledge = 1005,
    GTTGMPlayerEnterMap = 1003,
    GTTGMKickCharacter = 1009,
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::None => write!(f, "None"),
            Command::CTGTLogin => write!(f, "CTGTLogin"),
            Command::CTGTSelectCharacter => write!(f, "CTGTSelectCharacter"),
            Command::GTTCEstablishConnection => write!(f, "GTTCEstablishConnection"),
            Command::GTTCMapCrash => write!(f, "GTTCMapCrash"),
            Command::GTTCLogin => write!(f, "GTTCLogin"),
            Command::GTTGPInit => write!(f, "GTTGPInit"),
            Command::GTTGPSyncPlayerList => write!(f, "GTTGPSyncPlayerList"),
            Command::GTTGPUserLogin => write!(f, "GTTGPUserLogin"),
            Command::GTTGPUserLogout => write!(f, "GTTGPUserLogout"),
            Command::GTTGPUserDisconnected => write!(f, "GTTGPUserDisconnected"),
            Command::GMTGTInit => write!(f, "GMTGTInit"),

            _ => write!(f, "Unknown, value: {}", self.clone() as u16),
        }
    }
}

#[derive(PartialEq, Debug, TryFromPrimitive, Clone, IntoPrimitive)]
#[repr(u16)]
pub enum ErrorCode {
    None,

    // Error codes - Gate/GameServer -> Client
    ErrNetworkException = 1,
    ErrClientNotLoggedIn = 6,
    ErrClientVersionMismatch = 7,
}
