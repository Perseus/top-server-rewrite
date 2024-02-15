use colored::{Color, Colorize};

#[derive(Debug)]
pub struct Logger {
    color: Color,
    prefix: String,
    enable_file_logging: bool,
}

impl Logger {
    pub fn new(color: Color, prefix: String, enable_file_logging: bool) -> Self {
        Logger {
            color,
            prefix,
            enable_file_logging,
        }
    }

    fn get_timestamp(&self) -> String {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S.%3f");
        timestamp.to_string()
    }

    pub fn info(&self, message: &str) {
        let timestamp = self.get_timestamp();
        println!(
            "{}{}{}",
            format!("[{}]", timestamp).color(self.color),
            format!("[{}]: ", self.prefix).color(self.color),
            message.color(self.color)
        );
    }

    pub fn debug(&self, message: &str) {
        let timestamp = self.get_timestamp();
        println!(
            "{}{}{}",
            format!("[{}]", timestamp).color(self.color),
            format!("[{}]: ", self.prefix).color(self.color),
            message.color(self.color),
        );
    }

    pub fn error(&self, message: &str) {
        let timestamp = self.get_timestamp();
        println!(
            "{}{}{}",
            format!("[{}]", timestamp).red(),
            format!("[{}]: ", self.prefix).red(),
            message.red()
        );
    }
}
