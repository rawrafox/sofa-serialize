extern crate byteorder;
extern crate rustc_serialize;

use std::collections::BTreeMap;
use std::io;
use std::io::{Write};
use std::string;
use std::{i8, i16, i32};
use std::{u8, u16, u32};

use byteorder::{LittleEndian, WriteBytesExt};

use rustc_serialize::json::Json;

pub struct Encoder<'a> {
    writer: &'a mut (io::Write + 'a),
    dictionary: BTreeMap<String, u64>
}

impl<'a> Encoder<'a> {
    pub fn new(writer: &'a mut io::Write, dictionary: Vec<String>) -> Encoder<'a> {
        let mut map = BTreeMap::new();

        for (i, e) in dictionary.iter().enumerate() {
            map.insert(e.clone(), i as u64);
        }

        return Encoder { writer: writer, dictionary: map };
    }

    fn emit_length(&mut self, length: u64) -> io::Result<()> {
        match length {
            x if x < 0xF0 => {
                try!(self.writer.write_u8(x as u8));
            }
            x if x <= 0xFF => {
                try!(self.writer.write_u8(0xF0));
                try!(self.writer.write_u8(x as u8));
            }
            x if x <= 0xFFFF => {
                try!(self.writer.write_u8(0xF1));
                try!(self.writer.write_u16::<LittleEndian>(x as u16));
            }
            x if x <= 0xFFFFFFFF => {
                try!(self.writer.write_u8(0xF2));
                try!(self.writer.write_u32::<LittleEndian>(x as u32));
            }
            x => {
                try!(self.writer.write_u8(0xF3));
                try!(self.writer.write_u64::<LittleEndian>(x as u64));
            }
        }

        return Ok(());
    }

    fn emit_dictionary_string(&mut self, n: u64) -> io::Result<()> {
        match n {
            x if x <= 0xFF => {
                try!(self.writer.write_u8(0xF8));
                try!(self.writer.write_u8(x as u8));
            }
            x if x <= 0xFFFF => {
                try!(self.writer.write_u8(0xF9));
                try!(self.writer.write_u16::<LittleEndian>(x as u16));
            }
            x if x <= 0xFFFFFFFF => {
                try!(self.writer.write_u8(0xFA));
                try!(self.writer.write_u32::<LittleEndian>(x as u32));
            }
            x => {
                try!(self.writer.write_u8(0xFB));
                try!(self.writer.write_u64::<LittleEndian>(x as u64));
            }
        }

        return Ok(());
    }

    fn emit_string_fragment(&mut self, string: &str) -> io::Result<()> {
        let slice = string.as_bytes();

        let dictionary_index = self.dictionary.get(string).map(|x| *x);

        match dictionary_index {
            Some(i) => try!(self.emit_dictionary_string(i)),
            None => {
                try!(self.emit_length(slice.len() as u64));
                try!(self.writer.write_all(slice));
            }
        }

        return Ok(());
    }

    fn emit_nil(&mut self) -> io::Result<()> {
        try!(self.writer.write_u8(0x01));

        return Ok(());
    }

    fn emit_bool(&mut self, v: bool) -> io::Result<()> {
        try!(self.writer.write_u8(if v { 0x03 } else { 0x02} ));

        return Ok(());
    }

    fn emit_u8(&mut self, v: u8) -> io::Result<()> {
        try!(self.writer.write_u8(0x10));
        try!(self.writer.write_u8(v));

        return Ok(());
    }

    fn emit_u16(&mut self, v: u16) -> io::Result<()> {
        try!(self.writer.write_u8(0x11));
        try!(self.writer.write_u16::<LittleEndian>(v));

        return Ok(());
    }

    fn emit_u32(&mut self, v: u32) -> io::Result<()> {
        try!(self.writer.write_u8(0x12));
        try!(self.writer.write_u32::<LittleEndian>(v));

        return Ok(());
    }

    fn emit_u64(&mut self, v: u64) -> io::Result<()> {
        try!(self.writer.write_u8(0x13));
        try!(self.writer.write_u64::<LittleEndian>(v));

        return Ok(());
    }

    fn emit_i8(&mut self, v: i8) -> io::Result<()> {
        try!(self.writer.write_u8(0x20));
        try!(self.writer.write_i8(v));

        return Ok(());
    }

    fn emit_i16(&mut self, v: i16) -> io::Result<()> {
        try!(self.writer.write_u8(0x21));
        try!(self.writer.write_i16::<LittleEndian>(v));

        return Ok(());
    }

    fn emit_i32(&mut self, v: i32) -> io::Result<()> {
        try!(self.writer.write_u8(0x22));
        try!(self.writer.write_i32::<LittleEndian>(v));

        return Ok(());
    }

    fn emit_i64(&mut self, v: i64) -> io::Result<()> {
        try!(self.writer.write_u8(0x23));
        try!(self.writer.write_i64::<LittleEndian>(v));

        return Ok(());
    }

    fn emit_f32(&mut self, v: f32) -> io::Result<()> {
        try!(self.writer.write_u8(0x32));
        try!(self.writer.write_f32::<LittleEndian>(v));

        return Ok(());
    }

    fn emit_f64(&mut self, v: f64) -> io::Result<()> {
        try!(self.writer.write_u8(0x33));
        try!(self.writer.write_f64::<LittleEndian>(v));

        return Ok(());
    }

    fn emit_binary(&mut self, v: &[u8]) -> io::Result<()> {
        try!(self.writer.write_u8(0x40));
        try!(self.emit_length(v.len() as u64));
        try!(self.writer.write_all(v));

        return Ok(());
    }

    fn emit_string(&mut self, v: &str) -> io::Result<()> {
        try!(self.writer.write_u8(0x41));
        try!(self.emit_string_fragment(v));

        return Ok(());
    }

    fn emit_array_header(&mut self, length: u64) -> io::Result<()> {
        try!(self.writer.write_u8(0x50));
        try!(self.emit_length(length));

        return Ok(());
    }

    fn emit_map_header(&mut self, length: u64) -> io::Result<()> {
        try!(self.writer.write_u8(0x51));
        try!(self.emit_length(length));

        return Ok(());
    }

    fn emit_struct_header(&mut self, ty: &str, length: u64) -> io::Result<()> {
        try!(self.writer.write_u8(0x60));
        try!(self.emit_string_fragment(ty));
        try!(self.emit_length(length));

        return Ok(());
    }

    fn emit_object_header(&mut self, ty: &str, length: u64) -> io::Result<()> {
        try!(self.writer.write_u8(0x61));
        try!(self.emit_string_fragment(ty));
        try!(self.emit_length(length));

        return Ok(());
    }

    fn emit_guid(&mut self, v: [u8; 16]) -> io::Result<()> {
        try!(self.writer.write_u8(0x70));
        try!(self.writer.write_all(&v));

        return Ok(());
    }
}

/// Represents a Sofa value
#[derive(Clone, PartialOrd, Debug)]
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
pub type Map = BTreeMap<Value, Value>;
pub type Object = BTreeMap<string::String, Value>;

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
                    try!(key.serialize(encoder));
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
                writeln!(&mut io::stderr(), "I");
                if v >= i8::MIN as i64 && v <= i8::MAX as i64 {
                    try!(encoder.emit_i8(v as i8));
                } else if v >= i16::MIN as i64 && v <= i16::MAX as i64 {
                    try!(encoder.emit_i16(v as i16));
                } else if v >= i32::MIN as i64 && v <= i32::MAX as i64 {
                    try!(encoder.emit_i32(v as i32));
                } else {
                    try!(encoder.emit_i64(v));
                }
            }
            Json::U64(v) => {
                writeln!(&mut io::stderr(), "U");
                if v <= u8::MAX as u64 {
                    try!(encoder.emit_u8(v as u8));
                } else if v <= u16::MAX as u64 {
                    try!(encoder.emit_u16(v as u16));
                } else if v <= u32::MAX as u64 {
                    try!(encoder.emit_u32(v as u32));
                } else {
                    try!(encoder.emit_u64(v));
                }
            }
            Json::F64(v) => {
                writeln!(&mut io::stderr(), "F");
                try!(encoder.emit_f64(v));
            }
            Json::String(ref v) => {
                try!(encoder.emit_string(v))
            }
            Json::Boolean(v) => try!(encoder.emit_bool(v)),
            Json::Array(ref v) => {
                writeln!(&mut io::stderr(), "A {}", v.len());
                try!(encoder.emit_array_header(v.len() as u64));

                for element in v {
                    try!(element.serialize(encoder));
                }
            }
            Json::Object(ref v) => {
                writeln!(&mut io::stderr(), "O {}", v.len());
                try!(encoder.emit_object_header("", v.len() as u64));

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

impl std::cmp::PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        return self.encode().unwrap() == other.encode().unwrap();
    }
}

impl std::cmp::Eq for Value {

}

impl std::cmp::Ord for Value {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        return if self.eq(other) { std::cmp::Ordering::Equal } else { self.partial_cmp(other).unwrap() };
    }
}

#[cfg(test)]
mod tests {
    use super::Encoder;
    use super::Value;

    use std::collections::BTreeMap;

    #[test]
    fn encodes_short_length() {
        let mut result = Vec::new();

        {
            Encoder::new(&mut result).emit_length(0x00).unwrap();
        }

        assert_eq!(result, vec![0x00]);

        let mut result = Vec::new();

        {
            Encoder::new(&mut result).emit_length(0xEF).unwrap();
        }

        assert_eq!(result, vec![0xEF]);
    }

    #[test]
    fn encodes_u8_length() {
        let mut result = Vec::new();

        {
            Encoder::new(&mut result).emit_length(0xF0).unwrap();
        }

        assert_eq!(result, vec![0xF0, 0xF0]);

        let mut result = Vec::new();

        {
            Encoder::new(&mut result).emit_length(0xFF).unwrap();
        }

        assert_eq!(result, vec![0xF0, 0xFF]);
    }

    #[test]
    fn encodes_u16_length() {
        let mut result = Vec::new();

        {
            Encoder::new(&mut result).emit_length(0xFFFF).unwrap();
        }

        assert_eq!(result, vec![0xF1, 0xFF, 0xFF]);
    }

    #[test]
    fn encodes_u32_length() {
        let mut result = Vec::new();

        {
            Encoder::new(&mut result).emit_length(0xFFFFFFFF).unwrap();
        }

        assert_eq!(result, vec![0xF2, 0xFF, 0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn encodes_u64_length() {
        let mut result = Vec::new();

        {
            Encoder::new(&mut result).emit_length(0xFFFFFFFFFFFFFFFF).unwrap();
        }

        assert_eq!(result, vec![0xF3, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
    }

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
        assert_eq!(Value::String("abc".to_string()).encode().unwrap(), vec![0x41, 0x03, 0x61, 0x62, 0x63]);
    }

    #[test]
    fn encodes_array_documents() {
        assert_eq!(Value::Array(vec![Value::Boolean(true), Value::U8(0x01)]).encode().unwrap(), vec![0x50, 0x02, 0x03, 0x10, 0x01]);
    }

    #[test]
    fn encodes_map_documents() {
        let mut map = BTreeMap::new();

        map.insert(Value::String("abc".to_string()), Value::Boolean(true));

        assert_eq!(Value::Map(map).encode().unwrap(), vec![0x51, 0x01, 0x41, 0x03, 0x61, 0x62, 0x63, 0x03]);
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
