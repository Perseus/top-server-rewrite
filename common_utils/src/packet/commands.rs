use num_enum::{IntoPrimitive, TryFromPrimitive};

#[derive(PartialEq, Debug, TryFromPrimitive, Clone, IntoPrimitive)]
#[repr(u16)]
pub enum Command {
    None,

    // Client to GateServer
    CTGTLogin = 431,

    // GateServer to Client
    GTTCEstablishConnection = 940,
}
