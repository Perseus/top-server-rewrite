use std::path::Path;
use colored::Colorize;

use crate::config::{GateServerConfig, self};

pub struct GateServer {
  config: GateServerConfig
}

impl GateServer {
  pub fn start() -> Self {
    Self::print_gate_start();

    let path = "./config.yaml";
    println!("Loading GateServer config from {}", path.blue());
    let config = config::parse_config(Path::new(path));

    GateServer {
      config
    }
  }

  fn print_gate_start() {
    println!("{}", r"------------------------------------------------------------------------------------------------------------------------".blue());
    println!("{}", r"
     _______      ___   .___________. _______         _______. _______ .______     ____    ____  _______ .______      
    /  _____|    /   \  |           ||   ____|       /       ||   ____||   _  \    \   \  /   / |   ____||   _  \     
    |  |  __     /  ^  \ `---|  |----`|  |__         |   (----`|  |__   |  |_)  |    \   \/   /  |  |__   |  |_)  |    
    |  | |_ |   /  /_\  \    |  |     |   __|         \   \    |   __|  |      /      \      /   |   __|  |      /     
    |  |__| |  /  _____  \   |  |     |  |____    .----)   |   |  |____ |  |\  \----.  \    /    |  |____ |  |\  \----.
    \______|  /__/     \__\  |__|     |_______|   |_______/    |_______|| _| `._____|   \__/     |_______|| _| `._____|
    ".blue());
    println!("{}", r"                                                                                                                  v0.1.0".red());
    println!("{}", r"                                                                                                              by Perseus".red());
    println!("{}", r"------------------------------------------------------------------------------------------------------------------------".blue());
  }
}