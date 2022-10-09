use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};
use serde::de::DeserializeOwned;

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigParser<T> {
  data: T
}

impl<T> ConfigParser<T> {
  pub fn parse_file(file_path: &Path) -> Self
  where 
    T: DeserializeOwned {
    let contents = match fs::read_to_string (file_path) {
      Ok(contents) => contents,
      Err(err) => panic!("Could not find config file at {:?}, error - {}", file_path, err)
    };

    let data: T = match serde_yaml::from_str(&contents) {
      Ok(yaml) => yaml,
      Err(err) => panic!("Unable to parse the YAML config, err - {}", err)
    };

    let config_parser: ConfigParser<T> =  ConfigParser {
      data
    };

    config_parser
  }

  pub fn get_data(self) -> T {
    self.data
  }
}


#[cfg(test)]
mod tests {
use std::path::Path;

use super::ConfigParser;

  use serde::{Serialize, Deserialize};

  #[derive(Serialize, Deserialize)]
  struct ConfigStructure {
    name: String,
    id: u64
  }
  
  #[test]
  #[should_panic]
  fn it_throws_an_error_if_file_doesnt_exist() {
    ConfigParser::<ConfigStructure>::parse_file(Path::new("./tests/yada.yml"));
  }

  #[test]
  #[should_panic]
  fn it_throws_an_error_if_the_file_is_not_valid_yaml() {
    ConfigParser::<ConfigStructure>::parse_file(Path::new("./tests/invalid_yaml.yml"));
  }

  #[test]
  fn it_parses_a_valid_yml_file_correctly() {
    let contents = ConfigParser::<ConfigStructure>::parse_file(Path::new("./tests/valid_yaml.yml"));
    let data = contents.get_data();
    
    assert_eq!(data.id, 5);
    assert_eq!(data.name, "This is the name");
  }
}