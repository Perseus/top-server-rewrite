use std::io::Read;

use num::Unsigned;
use serde::de::{
    self, DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, VariantAccess,
    Visitor,
};
use serde::Deserialize;
use std::mem::size_of;

use super::error::{Error, Result};

pub struct Deserializer<'de> {
    input: &'de [u8],
}

impl<'de> Deserializer<'de> {
    pub fn from_bytes(input: &'de [u8]) -> Self {
        Deserializer { input }
    }
}

pub fn from_bytes<'a, T>(b: &'a [u8]) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_bytes(b);
    let t = T::deserialize(&mut deserializer)?;
    Ok(t)
}

struct Walker<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    first: bool
}

impl<'a, 'de> Walker<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        Walker {
            de,
            first: true
        }
    }
}

impl<'de, 'a> SeqAccess<'de> for Walker<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
        where
            T: DeserializeSeed<'de> {
        self.first = false;
        seed.deserialize(&mut *self.de).map(Some)
    }
}

impl<'de> Deserializer<'de> {
    fn peek_byte(&mut self) -> Result<u8> {
        match self.input.bytes().next() {
            Some(byte) => {
                if byte.is_ok() {
                    Ok(byte.unwrap())
                } else {
                    Err(Error::Eof)
                }
            }

            None => Err(Error::Eof),
        }
    }

    fn next_byte(&mut self) -> Result<u8> {
        let byte = self.peek_byte()?;
        self.input = &self.input[1..];
        Ok(byte)
    }

    fn parse_bool(&mut self) -> Result<bool> {
        let next_char = self.next_byte()?;

        Ok(next_char != 0)
    }

    // fn parse_uint<T>(&mut self) -> Result<T> where T: UnsignedInt {
    //   let num_of_bytes = size_of::<T>();
    //   let vec: Vec<u8> = vec![];

    //   for i in 0..num_of_bytes {
    //     let byte = self.next_byte()?;
    //     vec.push(byte);
    //   }

    // }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_bool(self.parse_bool()?)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_char(self.next_byte()? as char)
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut bytes: [u8; 4] = [0; 4];
        for i in 0..4 {
            let byte = self.next_byte()?;
            bytes[i] = byte;
        }

        visitor.visit_f32(f32::from_be_bytes(bytes))
    }

    fn deserialize_f64<V>(self, _: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_i128<V>(self, _: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_i16<V>(self, _: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_i32<V>(self, _: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_i64<V>(self, _: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_i8<V>(self, _: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_identifier<V>(self, _: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_ignored_any<V>(self, _: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_map<V>(self, _: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_newtype_struct<V>(self, _: &'static str, _: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_option<V>(self, _: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_seq<V>(self, _: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let str_len = self.next_byte()?;
        let mut bytes: Vec<u8> = Vec::new();
        for _ in 0..(str_len - 1) {
            bytes.push(self.next_byte()?);
        }

        let str = match String::from_utf8(bytes) {
            Ok(str) => str,
            Err(error) => String::from(""),
        };

        visitor.visit_str(&str)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let str_len = self.next_byte()?;
        if str_len < 1 {
            return Err(Error::ZeroStringLength);
        }

        let mut bytes: Vec<u8> = Vec::new();
        for _ in 0..(str_len - 1) {
            bytes.push(self.next_byte()?);
        }

        self.next_byte()?;

        let str = match String::from_utf8(bytes) {
            Ok(str) => str,
            Err(error) => String::from(""),
        };
        visitor.visit_str(&str)
    }

    fn deserialize_struct<V>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let value = visitor.visit_seq(Walker::new(self));
        value
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut bytes: [u8; 16] = [0; 16];
        for i in 0..16 {
            let byte = self.next_byte()?;
            bytes[i] = byte;
        }

        visitor.visit_u128(u128::from_be_bytes(bytes))
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut bytes: [u8; 4] = [0; 4];
        for i in 0..4 {
            let byte = self.next_byte()?;
            bytes[i] = byte;
        }

        visitor.visit_u32(u32::from_be_bytes(bytes))
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut bytes: [u8; 8] = [0; 8];
        for i in 0..8 {
            let byte = self.next_byte()?;
            bytes[i] = byte;
        }

        visitor.visit_u64(u64::from_be_bytes(bytes))
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut bytes: [u8; 1] = [0; 1];
        for i in 0..1 {
            let byte = self.next_byte()?;
            bytes[i] = byte;
        }

        visitor.visit_u8(u8::from_be_bytes(bytes))
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut bytes: [u8; 2] = [0; 2];
        for i in 0..2 {
            let byte = self.next_byte()?;
            bytes[i] = byte;
        }

        visitor.visit_u16(u16::from_be_bytes(bytes))
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_unit_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
}

mod test {
  use serde::Deserialize;
  use derive_more::{Display};

  use super::{from_bytes, Error};
  use super::super::error::{Result};

  #[derive(Deserialize, Debug)]
  struct TestPacket {
    name: String,
    password: String
  }

  #[test]
  fn test_deser_test_packet() {
    let bytes: Vec<u8> = vec![8, 80, 101, 114, 115, 101, 117, 115, 0, 9, 112, 97, 115, 115, 119, 111, 114, 100, 0, 0, 0, 0,
    10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 19, 136, 66, 73, 51, 51];
   
    let as_struct: TestPacket = from_bytes(&bytes).unwrap();

    assert_eq!(as_struct.name, String::from("Perseus"));
    assert_eq!(as_struct.password, String::from("password"));
  }

}