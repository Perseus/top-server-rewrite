use chrono::{Datelike, Local, Timelike};

#[derive(Debug)]
pub struct Player {
    chapstr: String,
    login_id: u32,
    account_id: u32,
    group_addr: u32,
    comm_key_len: u16,
    comm_text_key: String,
    is_active: bool,
    ip_addr: u32,
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
            is_active: false,
        }
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

    pub fn get_account_id(&self) -> u32 {
        self.account_id
    }

    pub fn get_ip_addr(&self) -> u32 {
        self.ip_addr
    }

    pub fn get_group_addr(&self) -> u32 {
        self.group_addr
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
        self.is_active = true;
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }
}
