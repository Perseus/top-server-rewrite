use serde::{ser, Serialize};

use super::error::{Error, Result};

pub struct Serializer {
    output: Vec<u8>,
}

pub fn to_bytes<T>(value: &T) -> Result<Vec<u8>>
where
    T: Serialize,
{
    let mut serializer = Serializer { output: vec![] };
    value.serialize(&mut serializer)?;
    Ok(serializer.output)
}

impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = ();

    type Error = Error;

    type SerializeSeq = Self;

    type SerializeTuple = Self;

    type SerializeTupleStruct = Self;

    type SerializeTupleVariant = Self;

    type SerializeMap = Self;

    type SerializeStruct = Self;

    type SerializeStructVariant = Self;

    fn serialize_u8(self, v: u8) -> Result<Self::Ok> {
        self.output.push(v);
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok> {
        let mut utf8_buf: [u8; 4] = [0; 4];
        v.encode_utf8(&mut utf8_buf);

        /* We don't want to accept any characters that don't fit in one byte */
        if utf8_buf.is_empty() || utf8_buf.len() > 1 {
            return Err(Error::ExpectedU8Character);
        }

        self.output.push(utf8_buf[0]);

        Ok(())
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok> {
        let str_len = v.len();

        self.output.push((str_len+1) as u8);

        for byte in v.as_bytes() {
            self.output.push(*byte);
        }

        self.output.push(b'\0');

        Ok(())
    }

    fn serialize_bool(self, v: bool) -> Result<Self::Ok> {
        self.output.push(v as u8);
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok> {
        for byte in v.iter() {
            self.output.push(*byte);
        }

        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok> {
        let as_bytes = v.to_be_bytes();
        for byte in as_bytes.iter() {
            self.output.push(*byte);
        }

        Ok(())
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok> {
        Err(Error::UnsupportedDataType)
    }

    fn serialize_i128(self, v: i128) -> Result<Self::Ok> {
        Err(Error::UnsupportedDataType)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok> {
        Err(Error::UnsupportedDataType)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok> {
        Err(Error::UnsupportedDataType)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok> {
        Err(Error::UnsupportedDataType)
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok> {
        Err(Error::UnsupportedDataType)
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(Error::UnsupportedDataType)
    }

    fn serialize_none(self) -> Result<Self::Ok> {
        Ok(())
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        Err(Error::UnsupportedDataType)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok> {
        let as_u8 = v.to_be_bytes();
        for byte in as_u8.iter() {
            self.serialize_u8(*byte)?;
        }

        Ok(())
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok> {
        let as_u8 = v.to_be_bytes();
        for byte in as_u8.iter() {
            self.serialize_u8(*byte)?;
        }

        Ok(())
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok> {
        let as_u8 = v.to_be_bytes();
        for byte in as_u8.iter() {
            self.serialize_u8(*byte)?;
        }

        Ok(())
    }

    fn serialize_u128(self, v: u128) -> Result<Self::Ok> {
        let as_u8 = v.to_be_bytes();
        for byte in as_u8.iter() {
            self.serialize_u8(*byte)?;
        }

        Ok(())
    }

    fn serialize_newtype_struct<T: ?Sized>(self, name: &'static str, value: &T) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        value.serialize(self)?;
        Ok(())
    }

    fn serialize_newtype_variant<T: ?Sized>(
            self,
            name: &'static str,
            variant_index: u32,
            variant: &'static str,
            value: &T,
        ) -> Result<Self::Ok>
        where
            T: Serialize {
        Err(Error::UnsupportedDataType)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        Err(Error::UnsupportedDataType)
    }
    
    fn serialize_struct(
            self,
            name: &'static str,
            len: usize,
        ) -> Result<Self::SerializeStruct> {
        Ok(self)
    }

    fn serialize_struct_variant(
            self,
            name: &'static str,
            variant_index: u32,
            variant: &'static str,
            len: usize,
        ) -> Result<Self::SerializeStructVariant> {
          Err(Error::UnsupportedDataType)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        Err(Error::UnsupportedDataType)
    }

    fn serialize_tuple_struct(
            self,
            name: &'static str,
            len: usize,
        ) -> Result<Self::SerializeTupleStruct> {
        Err(Error::UnsupportedDataType)
    }

    fn serialize_tuple_variant(
            self,
            name: &'static str,
            variant_index: u32,
            variant: &'static str,
            len: usize,
        ) -> Result<Self::SerializeTupleVariant> {
        Err(Error::UnsupportedDataType)
    }

    fn serialize_unit(self) -> Result<Self::Ok> {
        Ok(())
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok> {
      Err(Error::UnsupportedDataType)
    }

    fn serialize_unit_variant(
            self,
            name: &'static str,
            variant_index: u32,
            variant: &'static str,
        ) -> Result<Self::Ok> {
      Err(Error::UnsupportedDataType)
    }

    fn collect_seq<I>(self, iter: I) -> Result<Self::Ok>
    where
        I: IntoIterator,
        <I as IntoIterator>::Item: Serialize,
    {
        Ok(())
    }

    fn collect_map<K, V, I>(self, iter: I) -> Result<Self::Ok>
    where
        K: Serialize,
        V: Serialize,
        I: IntoIterator<Item = (K, V)>,
    {
        Ok(())
    }

    fn collect_str<T: ?Sized>(self, value: &T) -> Result<Self::Ok>
    where
        T: std::fmt::Display,
    {
        self.serialize_str(&value.to_string())
    }

    fn is_human_readable(&self) -> bool {
        true
    }
}

impl<'a> ser::SerializeSeq for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()>
        where
            T: Serialize {
        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(())
    }
}

impl<'a> ser::SerializeTuple for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()>
        where
            T: Serialize {
        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(())
    }
}

impl<'a> ser::SerializeTupleStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<()>
        where
            T: Serialize {
        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(())
    }
}

impl<'a> ser::SerializeTupleVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<()>
        where
            T: Serialize {
        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(())
    }
}

impl<'a> ser::SerializeMap for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_entry<K: ?Sized, V: ?Sized>(
            &mut self,
            key: &K,
            value: &V,
        ) -> Result<()>
        where
            K: Serialize,
            V: Serialize, {
        Ok(())
    }

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<()>
        where
            T: Serialize {
        Ok(())
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<()>
        where
            T: Serialize {
        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(())
    }
}

impl<'a> ser::SerializeStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(
            &mut self,
            key: &'static str,
            value: &T,
        ) -> Result<()>
        where
            T: Serialize {
        let serialized = value.serialize(&mut **self);
        serialized
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(())
    }

    fn skip_field(&mut self, key: &'static str) -> Result<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeStructVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: Serialize {
        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(())
    }

    fn skip_field(&mut self, key: &'static str) -> Result<()> {
        Ok(())
    }
}

mod test {
    use serde::Serialize;

    use super::{to_bytes};

    #[derive(Serialize)]
    struct TestPacket {
        name: String,
        password: String,
        attack: u32,
        defense: u128,
        mspd: f32
    }

    #[test]
    fn test_ser_se() {
        let packet = TestPacket{
            name: String::from("Perseus"),
            password: String::from("password"),
            attack: 10,
            defense: 5000,
            mspd: 50.3
        };

        assert_eq!(to_bytes(&packet).unwrap(), vec![
            8, 80, 101, 114, 115, 101, 117, 115, 0, 9, 112, 97, 115, 115, 119, 111, 114, 100, 0, 0, 0, 0,
            10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 19, 136, 66, 73, 51, 51
        ]);
    }
}