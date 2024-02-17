use num_enum::{IntoPrimitive, TryFromPrimitive};

#[derive(PartialEq, Debug, TryFromPrimitive, Clone, IntoPrimitive)]
#[repr(u16)]
pub enum Command {
    None,

    // Client to GateServer
    CTGTLogin = 431,
    CTGTSelectCharacter = 433,

    // GateServer to Client
    GTTCEstablishConnection = 940,
    GTTCMapCrash = 503,
    GTTCLogin = 931,

    // GateServer to GroupServer
    GTTGPInit = 2001,
    GTTGPSyncPlayerList = 2015,
    GTTGPUserLogin = 2003,
    GTTGPUserLogout = 2004,
    GTTGPUserDisconnected = 2011,
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
