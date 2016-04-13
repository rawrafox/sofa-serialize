extern crate byteorder;
extern crate rustc_serialize;

pub mod decoder;
pub mod decoder_error;

pub mod encoder;

use std::collections::BTreeMap;
use std::io;

use rustc_serialize::json::Json;

pub use encoder::Encoder;

/// Represents a Sofa value
#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub enum Value {
  Nil,
  Boolean(bool),
  U8(u8), U16(u16), U32(u32), U64(u64),
  I8(i8), I16(i16), I32(i32), I64(i64),
  F32(f32), F64(f64),
  Binary(Vec<u8>),
  String(String),
  Array(self::Array),
  Map(self::Map),
  Object(String, self::Object),
  Struct(String, self::Array),
  GUID([u8; 16])
}

pub type Array = Vec<Value>;
pub type Map = BTreeMap<String, Value>;
pub type Object = BTreeMap<String, Value>;

pub trait Serialize {
    fn serialize(&self, encoder: &mut Encoder) -> io::Result<()>;

    fn encode(&self) -> io::Result<Vec<u8>> {
        let mut result = Vec::new();

        {
            let mut encoder = Encoder::new(&mut result, vec![]);
            try!(self.serialize(&mut encoder));
        }

        return Ok(result);
    }
}

impl Serialize for Value {
    fn serialize(&self, encoder: &mut Encoder) -> io::Result<()> {
        match *self {
            Value::Nil => try!(encoder.emit_nil()),
            Value::Boolean(v) => try!(encoder.emit_bool(v)),
            Value::U8(v) => try!(encoder.emit_u8(v)),
            Value::U16(v) => try!(encoder.emit_u16(v)),
            Value::U32(v) => try!(encoder.emit_u32(v)),
            Value::U64(v) => try!(encoder.emit_u64(v)),
            Value::I8(v) => try!(encoder.emit_i8(v)),
            Value::I16(v) => try!(encoder.emit_i16(v)),
            Value::I32(v) => try!(encoder.emit_i32(v)),
            Value::I64(v) => try!(encoder.emit_i64(v)),
            Value::F32(v) => try!(encoder.emit_f32(v)),
            Value::F64(v) => try!(encoder.emit_f64(v)),
            Value::Binary(ref v) => try!(encoder.emit_binary(v)),
            Value::String(ref v) => try!(encoder.emit_string(v)),
            Value::Array(ref v) => {
                try!(encoder.emit_array_header(v.len() as u64));

                for element in v {
                    try!(element.serialize(encoder));
                }
            }
            Value::Map(ref v) => {
                try!(encoder.emit_map_header(v.len() as u64));

                for (key, value) in v {
                    try!(encoder.emit_string_fragment(key));
                    try!(value.serialize(encoder));
                }
            }
            Value::Struct(ref n, ref v) => {
                try!(encoder.emit_struct_header(n, v.len() as u64));

                for element in v {
                    try!(element.serialize(encoder));
                }
            }
            Value::Object(ref n, ref v) => {
                try!(encoder.emit_object_header(n, v.len() as u64));

                for (key, value) in v {
                    try!(encoder.emit_string_fragment(key));
                    try!(value.serialize(encoder));
                }
            }
            Value::GUID(v) => try!(encoder.emit_guid(v)),
        }

        return Ok(());
    }
}

impl Serialize for Json {
    fn serialize(&self, encoder: &mut Encoder) -> io::Result<()> {
        match *self {
            Json::I64(v) =>  {
                try!(encoder.emit_fixnum_from_i64(v));
            }
            Json::U64(v) => {
                try!(encoder.emit_fixnum_from_u64(v));
            }
            Json::F64(v) => {
                try!(encoder.emit_f64(v));
            }
            Json::String(ref v) => {
                try!(encoder.emit_string(v))
            }
            Json::Boolean(v) => try!(encoder.emit_bool(v)),
            Json::Array(ref v) => {
                try!(encoder.emit_array_header(v.len() as u64));

                for element in v {
                    try!(element.serialize(encoder));
                }
            }
            Json::Object(ref v) => {
                try!(encoder.emit_map_header(v.len() as u64));

                for (key, value) in v {
                    try!(encoder.emit_string_fragment(key));
                    try!(value.serialize(encoder));
                }
            }
            Json::Null => try!(encoder.emit_nil())
        }

        return Ok(());
    }
}

#[cfg(test)]
mod tests {
    use super::Value;
    use super::Serialize;

    use std::collections::BTreeMap;

    #[test]
    fn encodes_nil_document() {
        assert_eq!(Value::Nil.encode().unwrap(), vec![0x01]);
    }

    #[test]
    fn encodes_bool_documents() {
        assert_eq!(Value::Boolean(false).encode().unwrap(), vec![0x02]);
        assert_eq!(Value::Boolean(true).encode().unwrap(), vec![0x03]);
    }

    #[test]
    fn encodes_unsigned_documents() {
        assert_eq!(Value::U8(0x12).encode().unwrap(), vec![0x10, 0x12]);
        assert_eq!(Value::U16(0x1234).encode().unwrap(), vec![0x11, 0x34, 0x12]);
        assert_eq!(Value::U32(0x12345678).encode().unwrap(), vec![0x12, 0x78, 0x56, 0x34, 0x12]);
        assert_eq!(Value::U64(0x123456789ABCDEF0).encode().unwrap(), vec![0x13, 0xF0, 0xDE, 0xBC, 0x9A, 0x78, 0x56, 0x34, 0x12]);
    }

    #[test]
    fn encodes_signed_documents() {
        assert_eq!(Value::I8(-1).encode().unwrap(), vec![0x20, 0xFF]);
        assert_eq!(Value::I16(-2).encode().unwrap(), vec![0x21, 0xFE, 0xFF]);
        assert_eq!(Value::I32(-3).encode().unwrap(), vec![0x22, 0xFD, 0xFF, 0xFF, 0xFF]);
        assert_eq!(Value::I64(-4).encode().unwrap(), vec![0x23, 0xFC, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn encodes_float_documents() {
        assert_eq!(Value::F32(0.0).encode().unwrap(), vec![0x32, 0x00, 0x00, 0x00, 0x00]);
        assert_eq!(Value::F64(0.0).encode().unwrap(), vec![0x33, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn encodes_binary_documents() {
        assert_eq!(Value::Binary(vec![0x10, 0x00, 0x01]).encode().unwrap(), vec![0x40, 0x03, 0x10, 0x00, 0x01]);
    }

    #[test]
    fn encodes_string_documents() {
        assert_eq!(Value::String("abc".to_string()).encode().unwrap(), vec![0xC3, 0x61, 0x62, 0x63]);
    }

    #[test]
    fn encodes_array_documents() {
        assert_eq!(Value::Array(vec![Value::Boolean(true), Value::U8(0x01)]).encode().unwrap(), vec![0xB2, 0x03, 0x10, 0x01]);
    }

    #[test]
    fn encodes_map_documents() {
        let mut map = BTreeMap::new();

        map.insert("abc".to_string(), Value::Boolean(true));

        assert_eq!(Value::Map(map).encode().unwrap(), vec![0xA1, 0x03, 0x61, 0x62, 0x63, 0x03]);
    }

    #[test]
    fn encodes_struct_documents() {
        assert_eq!(Value::Struct("Herp".to_string(), vec![Value::Boolean(true), Value::U8(0x01)]).encode().unwrap(), vec![0x60, 0x04, 0x48, 0x65, 0x72, 0x70, 0x02, 0x03, 0x10, 0x01]);
    }

    #[test]
    fn encodes_object_documents() {
        let mut map = BTreeMap::new();

        map.insert("abc".to_string(), Value::Boolean(true));

        assert_eq!(Value::Object("Herp".to_string(), map).encode().unwrap(), vec![0x61, 0x04, 0x48, 0x65, 0x72, 0x70, 0x01, 0x03, 0x61, 0x62, 0x63, 0x03]);
    }
}
