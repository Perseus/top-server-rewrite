use common_utils::packet::*;

pub struct GameServerInitPacket {
    pub game_server_name: String,
    pub map_list: Vec<String>,
}

impl TypedPacket for GameServerInitPacket {
    fn from_base_packet(mut base_packet: common_utils::packet::BasePacket) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let game_server_name = base_packet.read_string().unwrap();
        let map_list_as_string = base_packet.read_string().unwrap();

        Ok(Self {
            game_server_name,
            map_list: map_list_as_string
                .split(';')
                .map(|s| s.to_string())
                .collect(),
        })
    }

    fn to_base_packet(&self) -> anyhow::Result<BasePacket> {
        unimplemented!()
    }
}

pub struct GameServerEnterMapPacket {
    act_id: u32,
    password: String,
    db_id: u32,
    world_id: u32,
    map: String,
    map_copy_no: i32,
    x: u32,
    y: u32,
    enter_type: u8,
    player_id: u32,
    s_winer: u16,
}

impl GameServerEnterMapPacket {
    pub fn new(
        act_id: u32,
        password: String,
        db_id: u32,
        world_id: u32,
        map: String,
        map_copy_no: i32,
        x: u32,
        y: u32,
        enter_type: u8,
        player_id: u32,
        s_winer: u16,
    ) -> Self {
        Self {
            act_id,
            password,
            db_id,
            world_id,
            map,
            map_copy_no,
            x,
            y,
            enter_type,
            player_id,
            s_winer,
        }
    }
}

impl TypedPacket for GameServerEnterMapPacket {
    fn from_base_packet(mut base_packet: common_utils::packet::BasePacket) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            act_id: 0,
            password: "".to_string(),
            db_id: 0,
            world_id: 0,
            map: "".to_string(),
            map_copy_no: 0,
            x: 0,
            y: 0,
            enter_type: 0,
            player_id: 0,
            s_winer: 0,
        })
    }

    fn to_base_packet(&self) -> anyhow::Result<BasePacket> {
        let mut base_packet = BasePacket::new();

        base_packet.write_cmd(commands::Command::GTTGMPlayerEnterMap)?;
        base_packet.write_long(self.act_id)?;
        base_packet.write_string(&self.password)?;
        base_packet.write_long(self.db_id)?;
        base_packet.write_long(self.world_id)?;
        base_packet.write_string(&self.map)?;

        if self.map_copy_no < 0 {
            base_packet.write_long(0xFFFFFFFF)?;
        } else {
            base_packet.write_long(self.map_copy_no as u32)?;
        }

        base_packet.write_long(self.x)?;
        base_packet.write_long(self.y)?;
        base_packet.write_char(self.enter_type)?;
        base_packet.write_long(self.player_id)?;
        base_packet.write_short(self.s_winer)?;
        base_packet.build_packet()?;

        Ok(base_packet)
    }
}

#[derive(Debug)]
pub struct GameServerEnterMapResultPacket {
    pub aim_num: u16,
    pub player_id: u32,
    pub db_id: u32,
    pub ret_code: u16,
    pub gm_addr: Option<u32>,
    pub total_players: Option<u32>,
    pub is_switch: Option<u8>,
}

impl GameServerEnterMapResultPacket {
    pub fn for_client(mut base_packet: BasePacket) -> BasePacket {
        let aim_num = base_packet.read_short().unwrap() as usize;

        base_packet.discard_last(2 + (4 * 2 * aim_num) + (4 * 2) + 1);
        base_packet.build_packet().unwrap();

        base_packet
    }
}

impl TypedPacket for GameServerEnterMapResultPacket {
    fn from_base_packet(mut base_packet: BasePacket) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let aim_num = base_packet.reverse_read_short().unwrap();
        let player_id = base_packet.reverse_read_long().unwrap();
        let db_id = base_packet.reverse_read_long().unwrap();
        let ret_code = base_packet.read_short().unwrap();
        let gm_addr = base_packet.reverse_read_long();
        let total_players = base_packet.reverse_read_long();
        let is_switch = base_packet.reverse_read_char();

        Ok(Self {
            aim_num,
            player_id,
            db_id,
            ret_code,
            gm_addr,
            total_players,
            is_switch,
        })
    }

    fn to_base_packet(&self) -> anyhow::Result<BasePacket> {
        unimplemented!()
    }
}

pub struct GameServerLeaveMapPacket {
    is_character_offline: u8,
    player_addr: u32,
    gm_addr: u32,
}

impl GameServerLeaveMapPacket {
    pub fn new(is_character_offline: u8, player_addr: u32, gm_addr: u32) -> Self {
        Self {
            is_character_offline,
            player_addr,
            gm_addr,
        }
    }
}

impl TypedPacket for GameServerLeaveMapPacket {
    fn from_base_packet(mut base_packet: common_utils::packet::BasePacket) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            is_character_offline: base_packet.read_char().unwrap(),
            player_addr: base_packet.read_long().unwrap(),
            gm_addr: base_packet.read_long().unwrap(),
        })
    }

    fn to_base_packet(&self) -> anyhow::Result<BasePacket> {
        let mut base_packet = BasePacket::new();

        base_packet.write_cmd(commands::Command::GTTGMPlayerLeaveMap)?;
        base_packet.write_char(self.is_character_offline)?;
        base_packet.write_long(self.player_addr)?;
        base_packet.write_long(self.gm_addr)?;
        base_packet.build_packet()?;

        Ok(base_packet)
    }
}
