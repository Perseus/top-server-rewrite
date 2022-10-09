use std::path::Path;

use config::parse_config;

mod config;

fn main() {
  let config = parse_config(Path::new("./config.yaml"));

}