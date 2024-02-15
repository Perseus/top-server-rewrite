use chrono::{Datelike, Local, Timelike};

#[derive(Debug)]
pub struct Player {
    chapstr: String,
    login_id: u32,
    account_id: u32,
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
        }
    }

    pub fn get_chapstr(&self) -> String {
        self.chapstr.clone()
    }

    pub fn get_login_id(&self) -> u32 {
        self.login_id
    }

    pub fn get_account_id(&self) -> u32 {
        self.account_id
    }
}
