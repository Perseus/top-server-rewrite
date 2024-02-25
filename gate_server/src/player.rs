use std::sync::Arc;

use chrono::{Datelike, Local, Timelike};
use tokio::{sync::RwLock, time::Instant};

use crate::gameserver_handler::GameServerHandler;

#[derive(Debug, PartialEq)]
pub enum CurrentScreen {
    None,
    CharacterSelection,
    InGame,
}

#[derive(Debug)]
pub struct Player {
    chapstr: String,
    login_id: u32,
    account_id: u32,
    group_addr: u32,
    comm_key_len: u16,
    comm_text_key: String,
    current_screen: CurrentScreen,
    ip_addr: u32,
    password: String,
    db_id: u32,
    world_id: u32,
    garner_winner: u16,
    current_game_server: Option<Arc<RwLock<GameServerHandler>>>,
    game_addr: u32,
    group_last_pinged_time: Instant,
}

impl Player {
    pub fn new() -> Self {
        let current_time = Local::now();
        let chapstr = format!(
            "[{:02}-{:02} {:02}:{:02}:{:02}:{:03}]",
            current_time.month(),
            current_time.day(),
            current_time.hour(),
            current_time.minute(),
            current_time.second(),
            current_time.timestamp_subsec_millis()
        );

        Player {
            chapstr,
            login_id: 0,
            account_id: 0,
            group_addr: 0,
            comm_key_len: 0,
            comm_text_key: "".to_string(),
            ip_addr: 0,
            current_screen: CurrentScreen::None,
            password: "".to_string(),
            db_id: 0,
            world_id: 0,
            garner_winner: 0,
            current_game_server: None,
            game_addr: 0,
            group_last_pinged_time: Instant::now(),
        }
    }

    pub fn unset_game_server(&mut self) {
        self.current_game_server = None;
    }

    pub fn get_chapstr(&self) -> String {
        self.chapstr.clone()
    }

    pub fn set_chapstr(&mut self, chapstr: String) {
        self.chapstr = chapstr;
    }

    pub fn get_login_id(&self) -> u32 {
        self.login_id
    }

    pub fn get_db_id(&self) -> u32 {
        self.db_id
    }

    pub fn get_account_id(&self) -> u32 {
        self.account_id
    }

    pub fn get_ip_addr(&self) -> u32 {
        self.ip_addr
    }

    pub fn get_group_addr(&self) -> u32 {
        self.group_addr
    }

    pub fn get_game_addr(&self) -> u32 {
        self.game_addr
    }

    pub fn get_password(&self) -> String {
        self.password.clone()
    }

    pub fn get_last_group_pinged_time(&self) -> Instant {
        self.group_last_pinged_time
    }

    pub fn reset_last_group_pinged_time(&mut self) {
        self.group_last_pinged_time = Instant::now();
    }

    pub fn set_current_game_server(
        &mut self,
        game_server: Arc<RwLock<GameServerHandler>>,
        game_addr: u32,
    ) {
        self.current_game_server = Some(game_server);
        self.game_addr = game_addr;
    }

    pub fn get_current_game_server(&self) -> Option<Arc<RwLock<GameServerHandler>>> {
        self.current_game_server.clone()
    }

    pub fn set_logged_in_context(
        &mut self,
        group_addr: u32,
        login_id: u32,
        account_id: u32,
        comm_key_len: u16,
        comm_text_key: String,
        ip_addr: u32,
    ) {
        self.login_id = login_id;
        self.account_id = account_id;
        self.group_addr = group_addr;
        self.comm_key_len = comm_key_len;
        self.comm_text_key = comm_text_key;
        self.ip_addr = ip_addr;
        self.current_screen = CurrentScreen::CharacterSelection;
    }

    pub fn set_begin_play_context(
        &mut self,
        password: String,
        db_id: u32,
        world_id: u32,
        garner_winner: u16,
    ) {
        self.password = password;
        self.db_id = db_id;
        self.world_id = world_id;
        self.garner_winner = garner_winner;
    }

    pub fn is_active(&self) -> bool {
        self.current_screen != CurrentScreen::None
    }

    pub fn is_in_game(&self) -> bool {
        self.current_screen == CurrentScreen::InGame
    }

    pub fn set_current_screen(&mut self, current_screen: CurrentScreen) {
        self.current_screen = current_screen;
    }
}
