use common_utils::packet::BasePacket;

pub enum GateCommands {
    SendToGroupServer(BasePacket),
}
