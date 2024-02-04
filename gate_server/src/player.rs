use chrono::{Datelike, Local, Timelike};

pub struct Player {
    chapstr: String,
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

        Player { chapstr }
    }

    pub fn get_chapstr(&self) -> String {
        self.chapstr.clone()
    }
}
