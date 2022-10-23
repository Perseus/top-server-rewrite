use serde::{ser, Serialize};

use super::error::{Error, Result};

pub struct Serializer {
  output: Vec<u8>, 
}

pub fn to_bytes<T>(value: &T) -> Result<Vec<u8>>
where
  T: Serialize {
    let mut serializer = Serializer {
      output: vec![]
    };
    value.serialize(&mut serializer);
    Ok(serializer.output)
  }

impl<'a> ser::Serializer for &'a mut Serializer {
  type Ok = ();

  type Error = Error;
  
  
  fn serialize_u8(self, v: u8) -> Result<Self::Ok> {
    self.output.push(v);
    Ok(())
  }

  fn serialize_char(self, v: char) -> Result<Self::Ok> {
    let utf8_buf: [u8; 4];
    v.encode_utf8(&mut utf8_buf);

    /* We don't want to accept any characters that don't fit in one byte */
    if utf8_buf.len() <= 1 {
      return Err(Error::ExpectedU8Character);
    }
    
    self.output.push(utf8_buf[0]);

    Ok(())
  }

  fn serialize_str(self, v: &str) -> Result<Self::Ok> {
    let str_len = v.len();
    
    self.output.push(str_len as u8);

    for byte in v.as_bytes() {
      self.output.push(*byte);
    }

    self.output.push(b'\0');
    
    Ok(())
  }
}